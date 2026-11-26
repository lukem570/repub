//! Errors for raster and HTML export.

use thiserror::Error;

/// Errors that can occur while exporting to a raster image or HTML.
#[derive(Error, Debug)]
pub enum RasterExportError {
    /// An underlying I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The requested page index does not exist.
    #[error("page {0} is out of range")]
    PageOutOfRange(usize),

    /// The scene could not be rasterized (e.g. the pixmap could not be sized).
    #[error("rasterization failed: {0}")]
    Render(String),

    /// Encoding the pixmap to the target image format failed.
    #[error("image encode error: {0}")]
    Encode(String),
}

/// Result type for raster/HTML export operations.
pub type RasterExportResult<T> = Result<T, RasterExportError>;
