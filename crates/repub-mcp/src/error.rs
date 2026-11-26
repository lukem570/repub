//! MCP error types.

use thiserror::Error;

/// MCP server errors.
#[derive(Error, Debug)]
pub enum McpError {
    /// No document is currently open
    #[error("No document is open. Use repub_create_document or repub_open_document first.")]
    NoDocument,

    /// Invalid page index
    #[error("Invalid page index: {0}. Document has {1} pages.")]
    InvalidPage(usize, usize),

    /// Invalid object ID
    #[error("Object not found: {0}")]
    ObjectNotFound(String),

    /// Invalid argument
    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    /// Unknown tool
    #[error("Unknown tool: {0}")]
    UnknownTool(String),

    /// Unknown resource
    #[error("Unknown resource: {0}")]
    UnknownResource(String),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Native format error
    #[error("Format error: {0}")]
    Format(#[from] repub_format_native::NativeError),

    /// Export error
    #[error("Export error: {0}")]
    Export(String),

    /// Protocol error
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Session error
    #[error("Session error: {0}")]
    Session(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for MCP operations.
pub type McpResult<T> = Result<T, McpError>;
