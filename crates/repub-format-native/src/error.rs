//! Errors for the native format.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur while saving or loading the native format.
#[derive(Error, Debug)]
pub enum NativeError {
    /// An underlying I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A JSON (de)serialization failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// A ZIP container failure (for the compressed `.repubz` format).
    #[error("ZIP error: {0}")]
    Zip(String),

    /// The file to load does not exist.
    #[error("file not found: {0}")]
    FileNotFound(PathBuf),

    /// The on-disk format version is not understood by this reader.
    #[error("version mismatch: expected {expected}, found {found}")]
    VersionMismatch {
        /// The version this reader writes and understands.
        expected: String,
        /// The version found in the file.
        found: String,
    },

    /// The compressed container was missing its document entry.
    #[error("invalid structure: {0}")]
    InvalidStructure(String),
}

impl From<zip::result::ZipError> for NativeError {
    fn from(e: zip::result::ZipError) -> Self {
        NativeError::Zip(e.to_string())
    }
}

/// Result type for native format operations.
pub type NativeResult<T> = Result<T, NativeError>;
