//! # Repub MCP
//!
//! Model Context Protocol server for AI integration.
//!
//! This crate provides an MCP server that enables AI assistants (like Claude)
//! to create and manipulate Repub documents programmatically.
//!
//! ## Features
//!
//! - Document creation and manipulation tools
//! - Page and object management
//! - Export to PDF, PNG, SVG
//! - Template system integration
//! - Resource exposure for document inspection
//!
//! ## Usage
//!
//! Start the MCP server via the CLI:
//!
//! ```bash
//! repub serve --mcp
//! ```
//!
//! Or use programmatically:
//!
//! ```ignore
//! use repub_mcp::McpServer;
//!
//! #[tokio::main]
//! async fn main() {
//!     let server = McpServer::new();
//!     server.run_stdio().await.unwrap();
//! }
//! ```

#![warn(missing_docs)]

pub mod error;
pub mod protocol;
pub mod server;
pub mod tools;
pub mod resources;
pub mod session;

pub use error::{McpError, McpResult};
pub use server::McpServer;
pub use session::Session;
