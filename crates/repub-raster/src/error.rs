//! Error types for the CPU rasterizer.

use thiserror::Error;

/// Errors that can occur while rasterizing or exporting a scene.
#[derive(Debug, Error)]
pub enum RasterError {
    /// A pixmap could not be allocated for the given dimensions.
    #[error("invalid pixmap dimensions: {width}x{height}")]
    InvalidDimensions {
        /// Requested width in pixels.
        width: u32,
        /// Requested height in pixels.
        height: u32,
    },

    /// An image could not be decoded.
    #[error("image error: {0}")]
    Image(String),

    /// A scene could not be exported to a file or byte buffer.
    #[error("export error: {0}")]
    Export(String),
}

impl From<std::io::Error> for RasterError {
    fn from(err: std::io::Error) -> Self {
        RasterError::Export(err.to_string())
    }
}

impl From<image::ImageError> for RasterError {
    fn from(err: image::ImageError) -> Self {
        RasterError::Image(err.to_string())
    }
}

/// Result type for rasterizer operations.
pub type RasterResult<T> = Result<T, RasterError>;
