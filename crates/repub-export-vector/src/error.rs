//! Errors for vector export (PDF and SVG).

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur while exporting to a vector format.
#[derive(Error, Debug)]
pub enum VectorError {
    /// An underlying I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The requested page index does not exist.
    #[error("page {0} is out of range")]
    PageOutOfRange(usize),

    /// A PDF generation failure.
    #[error("PDF error: {0}")]
    Pdf(String),

    /// An SVG generation failure.
    #[error("SVG error: {0}")]
    Svg(String),

    /// The output path was invalid.
    #[error("invalid path: {0}")]
    InvalidPath(PathBuf),
}

/// Result type for vector export operations.
pub type VectorResult<T> = Result<T, VectorError>;
