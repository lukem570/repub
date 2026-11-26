//! Publisher (`.pub`) format I/O errors.

use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during Publisher format operations.
#[derive(Error, Debug)]
pub enum FormatError {
    /// I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// OLE/CFB file error.
    #[error("OLE file error: {0}")]
    Ole(String),

    /// Unsupported format.
    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    /// Invalid file structure.
    #[error("Invalid file structure: {0}")]
    InvalidStructure(String),

    /// Missing required data.
    #[error("Missing required data: {0}")]
    MissingData(String),

    /// Version mismatch.
    #[error("Version mismatch: expected {expected}, found {found}")]
    VersionMismatch {
        /// The expected version string.
        expected: String,
        /// The version string found in the file.
        found: String,
    },

    /// File not found.
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// Parse error.
    #[error("Parse error: {0}")]
    Parse(String),

    /// Publisher import error (wraps specific import failures).
    #[error("Publisher import error: {0}")]
    PubImport(#[from] PubImportError),

    /// Strict import found items in the file it could not fully read.
    ///
    /// Returned by the strict import path when the parser had to skip, place a
    /// placeholder for, or otherwise not fully interpret one or more items in
    /// the `.pub` file. Each entry describes one unread item.
    #[error("strict import: {} item(s) in the .pub file could not be fully read:\n  {}", .0.len(), .0.join("\n  "))]
    UnreadItems(Vec<String>),
}

/// Specific errors that can occur during Publisher (`.pub`) file import.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PubImportError {
    /// Required OLE stream not found in the file.
    #[error("Required stream not found: {0}")]
    MissingStream(String),

    /// Invalid Quill stream header (missing or corrupted CHNKINK magic bytes).
    #[error("Invalid Quill header: expected CHNKINK magic bytes")]
    InvalidQuillHeader,

    /// Cannot locate or parse the Contents stream trailer.
    #[error("Invalid Contents trailer: cannot locate chunk directory")]
    InvalidContentsTrailer,

    /// Publisher file version is not supported.
    #[error("Unsupported Publisher version: {0}")]
    UnsupportedVersion(u32),

    /// Data within the file is corrupted or doesn't match expected format.
    #[error("Corrupted data: {0}")]
    CorruptedData(String),
}

/// Result type for Publisher format operations.
pub type FormatResult<T> = Result<T, FormatError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pub_import_error_messages() {
        let err = PubImportError::MissingStream("Contents".to_string());
        assert_eq!(err.to_string(), "Required stream not found: Contents");

        let err = PubImportError::InvalidQuillHeader;
        assert!(err.to_string().contains("CHNKINK"));

        let err = PubImportError::InvalidContentsTrailer;
        assert!(err.to_string().contains("trailer"));

        let err = PubImportError::UnsupportedVersion(5);
        assert_eq!(err.to_string(), "Unsupported Publisher version: 5");

        let err = PubImportError::CorruptedData("chunk offset out of bounds".to_string());
        assert!(err.to_string().contains("chunk offset out of bounds"));
    }

    #[test]
    fn test_pub_import_error_into_format_error() {
        let pub_err = PubImportError::MissingStream("Escher".to_string());
        let format_err: FormatError = pub_err.into();
        match format_err {
            FormatError::PubImport(inner) => {
                assert_eq!(inner, PubImportError::MissingStream("Escher".to_string()));
            }
            _ => panic!("Expected PubImport variant"),
        }
    }
}
