//! Layout errors and result types.

use thiserror::Error;

/// Errors that can occur during layout.
#[derive(Debug, Error)]
pub enum LayoutError {
    /// Font not found
    #[error("Font not found: {0}")]
    FontNotFound(String),

    /// Invalid font data
    #[error("Invalid font data: {0}")]
    InvalidFont(String),

    /// Text frame has no content area
    #[error("Text frame has no content area")]
    NoContentArea,

    /// Layout overflow (content doesn't fit)
    #[error("Layout overflow: {0}")]
    Overflow(String),

    /// Invalid layout constraint
    #[error("Invalid layout constraint: {0}")]
    InvalidConstraint(String),

    /// Circular frame link detected
    #[error("Circular frame link detected")]
    CircularLink,

    /// Internal layout error
    #[error("Internal layout error: {0}")]
    Internal(String),
}

/// Result type for layout operations.
pub type LayoutResult<T> = Result<T, LayoutError>;
