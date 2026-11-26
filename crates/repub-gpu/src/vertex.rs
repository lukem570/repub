//! Vertex formats and the view uniform, ported from the old `repub-render-gpu`.
//!
//! Shapes (fills, strokes, tessellated glyph outlines) use [`ShapeVertex`] with
//! a per-vertex color, so gradients are realized by baking a sampled color into
//! each vertex rather than with a dedicated gradient pipeline. Images use
//! [`TexturedVertex`]. [`ViewUniform`] carries the orthographic projection that
//! maps page points (scaled to device pixels) into clip space.

use bytemuck::{Pod, Zeroable};

/// A vertex for solid/gradient shape rendering: position plus an RGBA color.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ShapeVertex {
    /// Position in device pixels (page points × device scale).
    pub position: [f32; 2],
    /// Straight (non-premultiplied) RGBA color in 0–1.
    pub color: [f32; 4],
}

impl ShapeVertex {
    /// Create a shape vertex.
    pub fn new(x: f32, y: f32, color: [f32; 4]) -> Self {
        Self {
            position: [x, y],
            color,
        }
    }

    /// The vertex buffer layout for the shape pipeline.
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ShapeVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// A vertex for textured rendering (images): position, UV, and a tint color.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TexturedVertex {
    /// Position in device pixels.
    pub position: [f32; 2],
    /// Texture coordinates in 0–1.
    pub tex_coords: [f32; 2],
    /// Straight RGBA tint in 0–1 (opacity is folded into the alpha).
    pub color: [f32; 4],
}

impl TexturedVertex {
    /// Create a textured vertex.
    pub fn new(x: f32, y: f32, u: f32, v: f32, color: [f32; 4]) -> Self {
        Self {
            position: [x, y],
            tex_coords: [u, v],
            color,
        }
    }

    /// The vertex buffer layout for the textured pipeline.
    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TexturedVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// View uniform: the orthographic view-projection matrix and viewport size.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct ViewUniform {
    /// Column-major 4×4 view-projection matrix.
    pub view_proj: [[f32; 4]; 4],
    /// Viewport size in device pixels.
    pub viewport_size: [f32; 2],
    /// Padding to a 16-byte boundary.
    pub _padding: [f32; 2],
}

impl ViewUniform {
    /// Orthographic projection mapping device-pixel `(0,0)-(width,height)` to
    /// clip space, flipping Y so the origin is top-left.
    pub fn orthographic(width: f32, height: f32) -> Self {
        let view_proj = [
            [2.0 / width, 0.0, 0.0, 0.0],
            [0.0, -2.0 / height, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [-1.0, 1.0, 0.0, 1.0],
        ];
        Self {
            view_proj,
            viewport_size: [width, height],
            _padding: [0.0, 0.0],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_vertex_size() {
        assert_eq!(std::mem::size_of::<ShapeVertex>(), 24);
    }

    #[test]
    fn textured_vertex_size() {
        assert_eq!(std::mem::size_of::<TexturedVertex>(), 32);
    }

    #[test]
    fn view_uniform_size() {
        assert_eq!(std::mem::size_of::<ViewUniform>(), 80);
    }
}
