//! Error types for the editing engine.

use thiserror::Error;

/// Errors that editing operations can report.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum EditError {
    /// The referenced shape was not found on the page or document.
    #[error("shape not found")]
    ShapeNotFound,
    /// The referenced page index was out of range.
    #[error("page index {0} out of range")]
    PageOutOfRange(usize),
    /// The operation targeted a shape of the wrong kind.
    #[error("shape is not of the expected kind")]
    WrongShapeKind,
}

/// Result type for editing operations.
pub type EditResult<T> = Result<T, EditError>;
