//! The facade error type, wrapping each backend's errors.

use std::path::PathBuf;
use thiserror::Error;

/// An error from any format operation dispatched through the facade.
#[derive(Error, Debug)]
pub enum FormatError {
    /// An error from the native (`.repub`/`.repubz`) backend.
    #[error("native format error: {0}")]
    Native(#[from] repub_format_native::NativeError),

    /// An error from the vector (PDF/SVG) backend.
    #[error("vector export error: {0}")]
    Vector(#[from] repub_export_vector::VectorError),

    /// An error from the raster/HTML backend.
    #[error("raster export error: {0}")]
    Raster(#[from] repub_export_raster::RasterExportError),

    /// An underlying I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The file extension is not recognized by any backend.
    #[error("unsupported format for path: {0}")]
    UnsupportedExtension(PathBuf),

    /// The path has no extension to dispatch on.
    #[error("path has no extension: {0}")]
    MissingExtension(PathBuf),

    /// An error from the Microsoft Publisher (`.pub`) backend.
    #[error("pub format error: {0}")]
    Pub(#[from] repub_format_pub::FormatError),
}

/// Result type for facade operations.
pub type FormatResult<T> = Result<T, FormatError>;
