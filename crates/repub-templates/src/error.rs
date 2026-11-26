//! Template error types.

use thiserror::Error;

/// Template errors.
#[derive(Error, Debug)]
pub enum TemplateError {
    /// Template not found
    #[error("Template not found: {0}")]
    NotFound(String),

    /// Placeholder not found
    #[error("Placeholder not found: {0}")]
    PlaceholderNotFound(String),

    /// Required placeholder missing
    #[error("Required placeholder missing: {0}")]
    MissingRequired(String),

    /// Invalid placeholder value
    #[error("Invalid placeholder value for '{0}': {1}")]
    InvalidValue(String, String),

    /// Shape not found in template
    #[error("Shape not found in template: {0}")]
    ShapeNotFound(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid template format
    #[error("Invalid template format: {0}")]
    InvalidFormat(String),
}

/// Result type for template operations.
pub type TemplateResult<T> = Result<T, TemplateError>;
