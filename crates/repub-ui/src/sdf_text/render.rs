//! wgpu pipeline and egui paint callback for distance-field (MTSDF) text.
//!
//! One callback is emitted per text frame, so text keeps its z-order among the
//! egui shapes drawn before and after it. Each callback owns a vertex-buffer
//! "slot" for the frame; slots are reused across frames and grow on demand.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use eframe::{egui_wgpu, wgpu};
use wgpu::util::DeviceExt;

use super::atlas::{self, ATLAS_SIZE, SPREAD};

/// Vertex layout shared with `sdf_text.wgsl`.
#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    /// Normalized device coordinates within the callback viewport.
    pub position: [f32; 2],
    pub uv: [f32; 2],
    /// Straight-alpha sRGB color.
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Params {
    spread: f32,
    dilate_px: f32,
    _pad: [f32; 2],
}

/// Set once the pipeline is registered with egui-wgpu.
static INSTALLED: AtomicBool = AtomicBool::new(false);
/// Next free vertex-buffer slot this frame.
static NEXT_SLOT: AtomicUsize = AtomicUsize::new(0);

pub fn installed() -> bool {
    INSTALLED.load(Ordering::Acquire)
}

/// Reset slot allocation; call once per frame before drawing text.
pub fn begin_frame() {
    NEXT_SLOT.store(0, Ordering::Relaxed);
}

pub fn next_slot() -> usize {
    NEXT_SLOT.fetch_add(1, Ordering::Relaxed)
}

struct Slot {
    buffer: wgpu::Buffer,
    capacity: u64,
    vertex_count: u32,
}

/// GPU state stored in egui-wgpu's callback resources.
struct Resources {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    params: wgpu::Buffer,
    /// Created once the atlas is built and uploaded.
    bind_group: Option<wgpu::BindGroup>,
    /// The atlas texture and the atlas generation it holds
    texture: Option<(wgpu::Texture, u64)>,
    slots: Vec<Slot>,
}

/// Create the pipeline and register it with egui-wgpu. Starts the atlas build.
pub fn install(render_state: &egui_wgpu::RenderState) {
    let device = &render_state.device;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("sdf_text"),
        source: wgpu::ShaderSource::Wgsl(include_str!("sdf_text.wgsl").into()),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sdf_text"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("sdf_text"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sdf_text"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: std::mem::size_of::<Vertex>() as u64,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: render_state.target_format,
                // Same blending as egui-wgpu: premultiplied alpha
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        // Match egui's MSAA sample count: the desktop app asks for 4x, the browser can't
        multisample: wgpu::MultisampleState {
            count: if cfg!(target_arch = "wasm32") { 1 } else { 4 },
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    });

    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("sdf_text"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        ..Default::default()
    });

    let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("sdf_text params"),
        contents: bytemuck::bytes_of(&Params {
            spread: SPREAD,
            dilate_px: 0.0,
            _pad: [0.0; 2],
        }),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    render_state
        .renderer
        .write()
        .callback_resources
        .insert(Resources {
            pipeline,
            bind_group_layout,
            sampler,
            params,
            bind_group: None,
            texture: None,
            slots: Vec::new(),
        });
    INSTALLED.store(true, Ordering::Release);
    atlas::build_in_background();
}

/// Paint callback for one text frame (or, with `slot: None`, just the atlas upload).
pub struct TextCallback {
    pub slot: Option<usize>,
    pub vertices: Vec<Vertex>,
}

impl egui_wgpu::CallbackTrait for TextCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen_descriptor: &egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let Some(res) = callback_resources.get_mut::<Resources>() else {
            return Vec::new();
        };

        // Fonts loaded or glyphs baked since the upload: refresh the texture
        if let (Some((texture, uploaded)), Some(atlas)) = (res.texture.as_mut(), atlas::get()) {
            if *uploaded != atlas.generation {
                queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &atlas.pixels,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: Some((ATLAS_SIZE * atlas::CHANNELS) as u32),
                        rows_per_image: Some(ATLAS_SIZE as u32),
                    },
                    wgpu::Extent3d {
                        width: ATLAS_SIZE as u32,
                        height: ATLAS_SIZE as u32,
                        depth_or_array_layers: 1,
                    },
                );
                *uploaded = atlas.generation;
            }
        }

        // Upload the atlas the first time it is available
        if res.bind_group.is_none() {
            if let Some(atlas) = atlas::get() {
                let texture = device.create_texture_with_data(
                    queue,
                    &wgpu::TextureDescriptor {
                        label: Some("sdf_text atlas"),
                        size: wgpu::Extent3d {
                            width: ATLAS_SIZE as u32,
                            height: ATLAS_SIZE as u32,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        // Linear (not sRGB): the channels are distances, not colours
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                        view_formats: &[],
                    },
                    wgpu::util::TextureDataOrder::LayerMajor,
                    &atlas.pixels,
                );
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                res.texture = Some((texture, atlas.generation));
                res.bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("sdf_text"),
                    layout: &res.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&res.sampler),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: res.params.as_entire_binding(),
                        },
                    ],
                }));
            }
        }

        let Some(slot) = self.slot else {
            return Vec::new();
        };
        let bytes: &[u8] = bytemuck::cast_slice(&self.vertices);
        let needed = (bytes.len() as u64).max(256).next_power_of_two();
        while res.slots.len() <= slot {
            res.slots.push(Slot {
                buffer: vertex_buffer(device, 256),
                capacity: 256,
                vertex_count: 0,
            });
        }
        let entry = &mut res.slots[slot];
        if entry.capacity < needed {
            entry.buffer = vertex_buffer(device, needed);
            entry.capacity = needed;
        }
        if !bytes.is_empty() {
            queue.write_buffer(&entry.buffer, 0, bytes);
        }
        entry.vertex_count = self.vertices.len() as u32;
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let (Some(res), Some(slot)) = (callback_resources.get::<Resources>(), self.slot) else {
            return;
        };
        let (Some(bind_group), Some(entry)) = (res.bind_group.as_ref(), res.slots.get(slot)) else {
            return;
        };
        if entry.vertex_count == 0 {
            return;
        }
        let used = entry.vertex_count as u64 * std::mem::size_of::<Vertex>() as u64;
        render_pass.set_pipeline(&res.pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.set_vertex_buffer(0, entry.buffer.slice(..used));
        render_pass.draw(0..entry.vertex_count, 0..1);
    }
}

fn vertex_buffer(device: &wgpu::Device, size: u64) -> wgpu::Buffer {
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sdf_text vertices"),
        size,
        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}
