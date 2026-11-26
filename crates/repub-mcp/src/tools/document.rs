//! Document management tools.

use std::path::PathBuf;

use repub_format_native::{self as native, SaveOptions};
use repub_model::{Document, PageOrientation, PageSize};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::Tool;
use crate::error::{McpError, McpResult};
use crate::protocol::{ToolDefinition, ToolResult};
use crate::session::Session;

/// Create a new document.
pub struct CreateDocument;

#[derive(Debug, Deserialize)]
struct CreateDocumentArgs {
    #[serde(default = "default_size")]
    size: String,
    #[serde(default = "default_orientation")]
    orientation: String,
    #[serde(default = "default_pages")]
    pages: u32,
}

fn default_size() -> String {
    "letter".to_string()
}
fn default_orientation() -> String {
    "portrait".to_string()
}
fn default_pages() -> u32 {
    1
}

impl Tool for CreateDocument {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_create_document".to_string(),
            description: "Create a new Repub document".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "size": {
                        "type": "string",
                        "description": "Page size: letter, legal, tabloid, a3, a4, a5, or WxH in points (e.g., '612x792')",
                        "default": "letter"
                    },
                    "orientation": {
                        "type": "string",
                        "enum": ["portrait", "landscape"],
                        "default": "portrait"
                    },
                    "pages": {
                        "type": "integer",
                        "description": "Initial number of pages",
                        "default": 1,
                        "minimum": 1
                    }
                }
            }),
        }
    }

    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: CreateDocumentArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let page_size = parse_page_size(&args.size)?;
        let orientation = parse_orientation(&args.orientation)?;

        let mut doc = Document::new(page_size, orientation);

        // Add additional pages
        for _ in 1..args.pages {
            doc.add_page();
        }

        session.create_document(doc);

        Ok(ToolResult::json(json!({
            "success": true,
            "message": format!("Created new document with {} page(s)", args.pages),
            "pages": args.pages
        })))
    }
}

/// Open an existing document.
pub struct OpenDocument;

#[derive(Debug, Deserialize)]
struct OpenDocumentArgs {
    path: String,
}

impl Tool for OpenDocument {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_open_document".to_string(),
            description: "Open an existing Repub document".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to the document file"
                    }
                },
                "required": ["path"]
            }),
        }
    }

    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: OpenDocumentArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let path = PathBuf::from(&args.path);

        if !path.exists() {
            return Err(McpError::InvalidArgument(format!(
                "File not found: {}",
                args.path
            )));
        }

        let doc = native::load(&path)?;
        let page_count = doc.pages().len();

        session.open_document(doc, path);

        Ok(ToolResult::json(json!({
            "success": true,
            "message": format!("Opened document with {} page(s)", page_count),
            "path": args.path,
            "pages": page_count
        })))
    }
}

/// Save the current document.
pub struct SaveDocument;

#[derive(Debug, Deserialize)]
struct SaveDocumentArgs {
    path: Option<String>,
    #[serde(default)]
    compress: bool,
}

impl Tool for SaveDocument {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_save_document".to_string(),
            description: "Save the current document".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "path": {
                        "type": "string",
                        "description": "Path to save to (optional, uses current path if not specified)"
                    },
                    "compress": {
                        "type": "boolean",
                        "description": "Whether to compress the output",
                        "default": false
                    }
                }
            }),
        }
    }

    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: SaveDocumentArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let doc = session.document()?;

        let path = if let Some(p) = args.path {
            PathBuf::from(p)
        } else if let Some(ref p) = session.path {
            p.clone()
        } else {
            return Err(McpError::InvalidArgument(
                "No path specified and document has not been saved before".to_string(),
            ));
        };

        let save_opts = if args.compress {
            SaveOptions::compressed()
        } else {
            SaveOptions::default()
        };

        native::save(doc, &path, &save_opts)?;
        session.mark_saved(path.clone());

        Ok(ToolResult::json(json!({
            "success": true,
            "message": format!("Saved document to {}", path.display()),
            "path": path.to_string_lossy()
        })))
    }
}

/// Close the current document.
pub struct CloseDocument;

impl Tool for CloseDocument {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_close_document".to_string(),
            description: "Close the current document".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    fn execute(&self, session: &mut Session, _args: Value) -> McpResult<ToolResult> {
        let had_unsaved = session.dirty;
        session.close_document();

        Ok(ToolResult::json(json!({
            "success": true,
            "message": "Document closed",
            "had_unsaved_changes": had_unsaved
        })))
    }
}

/// Get document information.
pub struct GetDocumentInfo;

impl Tool for GetDocumentInfo {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_get_document_info".to_string(),
            description: "Get information about the current document".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {}
            }),
        }
    }

    fn execute(&self, session: &mut Session, _args: Value) -> McpResult<ToolResult> {
        let doc = session.document()?;

        let pages: Vec<Value> = doc
            .pages()
            .iter()
            .enumerate()
            .map(|(i, page)| {
                let obj_count: usize = page.layers().iter().map(|l| l.shapes().len()).sum();
                json!({
                    "index": i,
                    "width": page.size.width,
                    "height": page.size.height,
                    "objects": obj_count
                })
            })
            .collect();

        let total_objects: usize = pages
            .iter()
            .filter_map(|p| p["objects"].as_u64())
            .sum::<u64>() as usize;

        Ok(ToolResult::json(json!({
            "pages": pages.len(),
            "total_objects": total_objects,
            "page_details": pages,
            "path": session.path.as_ref().map(|p| p.to_string_lossy().to_string()),
            "has_unsaved_changes": session.dirty
        })))
    }
}

/// Parse a page size string.
fn parse_page_size(s: &str) -> McpResult<PageSize> {
    match s.to_lowercase().as_str() {
        "letter" => Ok(PageSize::Letter),
        "legal" => Ok(PageSize::Legal),
        "tabloid" => Ok(PageSize::Tabloid),
        "a3" => Ok(PageSize::A3),
        "a4" => Ok(PageSize::A4),
        "a5" => Ok(PageSize::A5),
        custom if custom.contains('x') => {
            let parts: Vec<&str> = custom.split('x').collect();
            if parts.len() != 2 {
                return Err(McpError::InvalidArgument(format!(
                    "Invalid size format: {}",
                    s
                )));
            }
            let width: f64 = parts[0]
                .trim()
                .parse()
                .map_err(|_| McpError::InvalidArgument(format!("Invalid width: {}", parts[0])))?;
            let height: f64 = parts[1]
                .trim()
                .parse()
                .map_err(|_| McpError::InvalidArgument(format!("Invalid height: {}", parts[1])))?;
            Ok(PageSize::Custom { width, height })
        }
        _ => Err(McpError::InvalidArgument(format!(
            "Unknown page size: {}",
            s
        ))),
    }
}

/// Parse an orientation string.
fn parse_orientation(s: &str) -> McpResult<PageOrientation> {
    match s.to_lowercase().as_str() {
        "portrait" => Ok(PageOrientation::Portrait),
        "landscape" => Ok(PageOrientation::Landscape),
        _ => Err(McpError::InvalidArgument(format!(
            "Unknown orientation: {}",
            s
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_page_size() {
        assert!(matches!(parse_page_size("letter"), Ok(PageSize::Letter)));
        assert!(matches!(parse_page_size("A4"), Ok(PageSize::A4)));
        assert!(matches!(
            parse_page_size("612x792"),
            Ok(PageSize::Custom { .. })
        ));
    }

    #[test]
    fn test_parse_orientation() {
        assert!(matches!(
            parse_orientation("portrait"),
            Ok(PageOrientation::Portrait)
        ));
        assert!(matches!(
            parse_orientation("LANDSCAPE"),
            Ok(PageOrientation::Landscape)
        ));
    }

    #[test]
    fn test_create_document() {
        let tool = CreateDocument;
        let mut session = Session::new();

        let result = tool
            .execute(
                &mut session,
                json!({
                    "size": "letter",
                    "pages": 2
                }),
            )
            .unwrap();

        assert!(!result.is_error);
        assert!(session.document.is_some());
        assert_eq!(session.document().unwrap().pages().len(), 2);
    }
}
