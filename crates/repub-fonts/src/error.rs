//! Font errors and result types.

use thiserror::Error;

/// Errors that can occur while loading or matching fonts.
#[derive(Debug, Error)]
pub enum FontError {
    /// Font not found.
    #[error("Font not found: {0}")]
    FontNotFound(String),

    /// Invalid font data.
    #[error("Invalid font data: {0}")]
    InvalidFont(String),

    /// Internal font error.
    #[error("Internal font error: {0}")]
    Internal(String),
}

/// Result type for font operations.
pub type FontResult<T> = Result<T, FontError>;
