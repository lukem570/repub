//! CLI error types.

use std::path::PathBuf;

use thiserror::Error;

/// CLI-specific errors.
#[derive(Error, Debug)]
pub enum CliError {
    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// Invalid file format
    #[error("Invalid file format: {0}")]
    InvalidFormat(String),

    /// Unsupported operation
    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    /// Invalid argument
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Format error
    #[error("Format error: {0}")]
    Format(#[from] repub_format::FormatError),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Document validation error
    #[error("Validation error: {0}")]
    Validation(String),
}

/// Result type for CLI operations.
pub type CliResult<T> = Result<T, CliError>;
