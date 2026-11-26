//! MCP tool implementations.

pub mod document;
pub mod export;
pub mod objects;
pub mod page;
pub mod template;

use serde_json::Value;

use crate::error::McpResult;
use crate::protocol::{ToolDefinition, ToolResult};
use crate::session::Session;

/// Trait for MCP tools.
pub trait Tool: Send + Sync {
    /// Get the tool definition.
    fn definition(&self) -> ToolDefinition;

    /// Execute the tool.
    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult>;
}

/// Get all available tools.
pub fn all_tools() -> Vec<Box<dyn Tool>> {
    vec![
        // Document tools
        Box::new(document::CreateDocument),
        Box::new(document::OpenDocument),
        Box::new(document::SaveDocument),
        Box::new(document::CloseDocument),
        Box::new(document::GetDocumentInfo),
        // Page tools
        Box::new(page::AddPage),
        Box::new(page::DeletePage),
        Box::new(page::ListPages),
        // Object tools
        Box::new(objects::AddTextFrame),
        Box::new(objects::AddShape),
        Box::new(objects::AddImage),
        Box::new(objects::AddWordArt),
        Box::new(objects::ListObjects),
        Box::new(objects::DeleteObject),
        // Export tools
        Box::new(export::ExportDocument),
        Box::new(export::RenderPreview),
        // Template tools
        Box::new(template::ListTemplates),
        Box::new(template::GetTemplate),
        Box::new(template::CreateFromTemplate),
        Box::new(template::SearchTemplates),
    ]
}
