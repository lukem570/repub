//! Errors raised while setting up or driving the GPU renderer.

use thiserror::Error;

/// An error from the GPU renderer.
#[derive(Error, Debug)]
pub enum GpuError {
    /// No GPU adapter was available. In headless/CI environments this is the
    /// expected signal to skip GPU-dependent work rather than fail.
    #[error("no suitable GPU adapter found")]
    NoAdapter,

    /// The wgpu device could not be created.
    #[error("failed to create GPU device: {0}")]
    DeviceCreation(String),

    /// A path could not be tessellated into triangles.
    #[error("tessellation error: {0}")]
    Tessellation(String),

    /// Reading pixels back from the GPU failed.
    #[error("readback error: {0}")]
    Readback(String),

    /// The requested render size was zero or otherwise invalid.
    #[error("invalid render size: {0}x{1}")]
    InvalidSize(u32, u32),
}

/// Result type for GPU operations.
pub type GpuResult<T> = Result<T, GpuError>;
