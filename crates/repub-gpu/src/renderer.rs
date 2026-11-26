//! The [`GpuRenderer`]: a windowless executor of a [`Scene`].
//!
//! The renderer walks [`Scene::primitives`] in back-to-front order, turning
//! each into GPU draw commands recorded into a single render pass against an
//! offscreen target, then reads the target back to RGBA8 pixels.
//!
//! It is a **dumb executor**: all geometry, text layout and effect resolution
//! happened in `compose`. This crate only tessellates the resolved paths/glyph
//! outlines, builds image quads, and draws them.

use repub_geometry::Rect;
use repub_render::{RenderOptions, Renderer};
use repub_scene::{Paint, Primitive, Scene};

use crate::context::GpuContext;
use crate::effects;
use crate::error::{GpuError, GpuResult};
use crate::image as img;
use crate::paint::{color_array, color_fn};
use crate::tessellation::{self, Tessellation};
use crate::text::TextRenderer;
use crate::texture;
use crate::vertex::{ShapeVertex, TexturedVertex};

/// A GPU renderer owning a headless context and font state.
pub struct GpuRenderer {
    context: GpuContext,
    text: TextRenderer,
    options: RenderOptions,
    /// Pixels from the last [`Renderer::render_scene`] call.
    last_pixels: Option<RenderedImage>,
}

/// A rendered RGBA8 image with its device dimensions.
#[derive(Clone)]
pub struct RenderedImage {
    /// Width in device pixels.
    pub width: u32,
    /// Height in device pixels.
    pub height: u32,
    /// Row-major straight RGBA8 pixels.
    pub pixels: Vec<u8>,
}

/// One recorded draw: a shape batch or a textured image quad.
enum Draw {
    Shape {
        vertices: Vec<ShapeVertex>,
        indices: Vec<u32>,
        clip: Option<Rect>,
    },
    Image {
        vertices: Vec<TexturedVertex>,
        indices: Vec<u32>,
        texture: DecodedUpload,
        clip: Option<Rect>,
    },
}

struct DecodedUpload {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

/// An uploaded draw ready to record into the render pass.
enum GpuDraw {
    Shape {
        vb: wgpu::Buffer,
        ib: wgpu::Buffer,
        count: u32,
        clip: Option<Rect>,
    },
    Image {
        vb: wgpu::Buffer,
        ib: wgpu::Buffer,
        count: u32,
        bind: wgpu::BindGroup,
        clip: Option<Rect>,
    },
}

impl GpuRenderer {
    /// Create a renderer with a headless context, blocking on device setup.
    ///
    /// Returns [`GpuError::NoAdapter`] when no GPU is available; callers that
    /// want to skip gracefully (tests, CI) can match on it.
    pub fn new_blocking() -> GpuResult<Self> {
        Ok(Self {
            context: GpuContext::new_blocking()?,
            text: TextRenderer::new(),
            options: RenderOptions::default(),
            last_pixels: None,
        })
    }

    /// Set the render options (DPI/scale/background/antialias).
    pub fn with_options(mut self, options: RenderOptions) -> Self {
        self.options = options;
        self
    }

    /// Render a scene to straight RGBA8 pixels sized to the scene × device
    /// scale.
    pub fn render_to_pixels(&mut self, scene: &Scene) -> GpuResult<Vec<u8>> {
        Ok(self.render(scene)?.pixels)
    }

    /// Render a scene, returning the pixels and their device dimensions.
    pub fn render(&mut self, scene: &Scene) -> GpuResult<RenderedImage> {
        let scale = self.options.pixels_per_point();
        let width = ((scene.size.width * scale).round() as u32).max(1);
        let height = ((scene.size.height * scale).round() as u32).max(1);
        if width == 0 || height == 0 {
            return Err(GpuError::InvalidSize(width, height));
        }

        self.context.set_viewport(width, height);
        let (target, view) = self.context.render_target(width, height);

        let clear = self.clear_color(scene);
        let draws = self.build_draws(scene, scale)?;

        // Upload all GPU buffers before opening the render pass, preserving the
        // back-to-front order so later primitives draw over earlier ones.
        let mut gpu_draws: Vec<GpuDraw> = Vec::with_capacity(draws.len());
        for draw in &draws {
            match draw {
                Draw::Shape {
                    vertices,
                    indices,
                    clip,
                } => {
                    if indices.is_empty() {
                        continue;
                    }
                    gpu_draws.push(GpuDraw::Shape {
                        vb: self.context.vertex_buffer(vertices),
                        ib: self.context.index_buffer(indices),
                        count: indices.len() as u32,
                        clip: *clip,
                    });
                }
                Draw::Image {
                    vertices,
                    indices,
                    texture,
                    clip,
                } => {
                    if indices.is_empty() {
                        continue;
                    }
                    let (_tex, bind) =
                        self.context
                            .upload_texture(texture.width, texture.height, &texture.data);
                    gpu_draws.push(GpuDraw::Image {
                        vb: self.context.vertex_buffer(vertices),
                        ib: self.context.index_buffer(indices),
                        count: indices.len() as u32,
                        bind,
                        clip: *clip,
                    });
                }
            }
        }

        let mut encoder =
            self.context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("scene encoder"),
                });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("scene pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(clear),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            for draw in &gpu_draws {
                match draw {
                    GpuDraw::Shape {
                        vb,
                        ib,
                        count,
                        clip,
                    } => {
                        pass.set_pipeline(&self.context.shape_pipeline);
                        pass.set_bind_group(0, &self.context.view_bind_group, &[]);
                        apply_scissor(&mut pass, clip, scale, width, height);
                        pass.set_vertex_buffer(0, vb.slice(..));
                        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..*count, 0, 0..1);
                    }
                    GpuDraw::Image {
                        vb,
                        ib,
                        count,
                        bind,
                        clip,
                    } => {
                        pass.set_pipeline(&self.context.textured_pipeline);
                        pass.set_bind_group(0, &self.context.view_bind_group, &[]);
                        pass.set_bind_group(1, bind, &[]);
                        apply_scissor(&mut pass, clip, scale, width, height);
                        pass.set_vertex_buffer(0, vb.slice(..));
                        pass.set_index_buffer(ib.slice(..), wgpu::IndexFormat::Uint32);
                        pass.draw_indexed(0..*count, 0, 0..1);
                    }
                }
            }
        }
        self.context.queue.submit(Some(encoder.finish()));

        let pixels = self.context.read_target(&target, width, height)?;
        Ok(RenderedImage {
            width,
            height,
            pixels,
        })
    }

    fn clear_color(&self, scene: &Scene) -> wgpu::Color {
        let color = match &scene.background {
            Some(Paint::Solid(c)) => Some(*c),
            Some(other) => Some(other.primary_color()),
            None => self.options.background,
        };
        match color {
            Some(c) => wgpu::Color {
                r: srgb_to_linear(c.r),
                g: srgb_to_linear(c.g),
                b: srgb_to_linear(c.b),
                a: c.a,
            },
            None => wgpu::Color::TRANSPARENT,
        }
    }

    fn build_draws(&mut self, scene: &Scene, scale: f64) -> GpuResult<Vec<Draw>> {
        let mut draws = Vec::new();
        for prim in &scene.primitives {
            match prim {
                Primitive::FillPath {
                    path,
                    paint,
                    fill_rule,
                    transform,
                    clip,
                    effects,
                } => {
                    if let Some(shadow) = effects::drop_shadow(effects) {
                        let shadow_t = transform.then(&repub_geometry::Transform2D::translate(
                            shadow.offset_x,
                            shadow.offset_y,
                        ));
                        let color = color_array(&shadow.color, 1.0);
                        let map = tessellation::device_mapper(shadow_t, scale);
                        let lyon = tessellation::to_lyon(path, &map);
                        let geo = tessellation::fill(&lyon, *fill_rule, &move |_| color)?;
                        push_shape(&mut draws, geo, *clip);
                    }
                    let color = color_fn(paint, scale, 1.0);
                    let map = tessellation::device_mapper(*transform, scale);
                    let lyon = tessellation::to_lyon(path, &map);
                    let geo = tessellation::fill(&lyon, *fill_rule, &color)?;
                    push_shape(&mut draws, geo, *clip);
                }
                Primitive::StrokePath {
                    path,
                    paint,
                    width,
                    cap,
                    join,
                    dash: _,
                    transform,
                    clip,
                    effects,
                } => {
                    let device_width = (*width * scale) as f32;
                    if let Some(shadow) = effects::drop_shadow(effects) {
                        let shadow_t = transform.then(&repub_geometry::Transform2D::translate(
                            shadow.offset_x,
                            shadow.offset_y,
                        ));
                        let color = color_array(&shadow.color, 1.0);
                        let map = tessellation::device_mapper(shadow_t, scale);
                        let lyon = tessellation::to_lyon(path, &map);
                        let geo =
                            tessellation::stroke(&lyon, device_width, *cap, *join, &move |_| {
                                color
                            })?;
                        push_shape(&mut draws, geo, *clip);
                    }
                    let color = color_fn(paint, scale, 1.0);
                    let map = tessellation::device_mapper(*transform, scale);
                    let lyon = tessellation::to_lyon(path, &map);
                    let geo = tessellation::stroke(&lyon, device_width, *cap, *join, &color)?;
                    push_shape(&mut draws, geo, *clip);
                }
                Primitive::Glyphs {
                    font,
                    size,
                    glyphs,
                    paint,
                    transform,
                    clip,
                    effects: _,
                } => {
                    let geo = self
                        .text
                        .tessellate_run(font, *size, glyphs, paint, *transform, scale)?;
                    push_shape(&mut draws, geo, *clip);
                }
                Primitive::Image {
                    source,
                    dest,
                    crop,
                    opacity,
                    transform,
                    clip,
                    effects: _,
                } => match texture::decode(source) {
                    Some(decoded) => {
                        let quad =
                            img::image_quad(dest, crop.as_ref(), *opacity, *transform, scale);
                        draws.push(Draw::Image {
                            vertices: quad.vertices.to_vec(),
                            indices: quad.indices.to_vec(),
                            texture: DecodedUpload {
                                width: decoded.width,
                                height: decoded.height,
                                data: decoded.data,
                            },
                            clip: *clip,
                        });
                    }
                    None => {
                        let (vertices, indices) = img::placeholder_quad(dest, *transform, scale);
                        draws.push(Draw::Shape {
                            vertices: vertices.to_vec(),
                            indices: indices.to_vec(),
                            clip: *clip,
                        });
                    }
                },
            }
        }
        Ok(draws)
    }
}

fn push_shape(draws: &mut Vec<Draw>, geo: Tessellation, clip: Option<Rect>) {
    if geo.is_empty() {
        return;
    }
    draws.push(Draw::Shape {
        vertices: geo.vertices,
        indices: geo.indices,
        clip,
    });
}

fn apply_scissor(
    pass: &mut wgpu::RenderPass,
    clip: &Option<Rect>,
    scale: f64,
    width: u32,
    height: u32,
) {
    let rect = match clip {
        Some(r) => r,
        None => {
            pass.set_scissor_rect(0, 0, width, height);
            return;
        }
    };
    let x = (rect.x() * scale).floor().max(0.0) as u32;
    let y = (rect.y() * scale).floor().max(0.0) as u32;
    let w = (rect.width() * scale).ceil() as u32;
    let h = (rect.height() * scale).ceil() as u32;
    let x = x.min(width);
    let y = y.min(height);
    let w = w.min(width - x);
    let h = h.min(height - y);
    pass.set_scissor_rect(x, y, w.max(1), h.max(1));
}

/// Convert an sRGB component (0–1) to linear for a clear color, since the target
/// is an `*Srgb` format whose clear value is interpreted as linear.
fn srgb_to_linear(c: f64) -> f64 {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

impl Renderer for GpuRenderer {
    fn render_scene(&mut self, scene: &Scene) {
        // The trait returns nothing; stash the result for later retrieval and
        // ignore GPU errors here (callers wanting the pixels or error use
        // `render`/`render_to_pixels`).
        if let Ok(image) = self.render(scene) {
            self.last_pixels = Some(image);
        }
    }

    fn options(&self) -> RenderOptions {
        self.options
    }
}

impl GpuRenderer {
    /// The pixels produced by the most recent [`Renderer::render_scene`] call.
    pub fn last_image(&self) -> Option<&RenderedImage> {
        self.last_pixels.as_ref()
    }
}
