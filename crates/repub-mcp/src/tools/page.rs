//! Page management tools.

use serde::Deserialize;
use serde_json::{json, Value};

use super::Tool;
use crate::error::{McpError, McpResult};
use crate::protocol::{ToolDefinition, ToolResult};
use crate::session::Session;

/// Add a new page.
pub struct AddPage;

#[derive(Debug, Deserialize)]
struct AddPageArgs {
    #[serde(default)]
    position: Option<i32>,
}

impl Tool for AddPage {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_add_page".to_string(),
            description: "Add a new page to the document".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "position": {
                        "type": "integer",
                        "description": "Position to insert (0-based index, -1 or omit for end)"
                    }
                }
            }),
        }
    }

    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: AddPageArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let doc = session.document_mut()?;
        let page_count = doc.pages().len();

        let position = match args.position {
            Some(p) if p >= 0 => {
                let pos = p as usize;
                if pos > page_count {
                    return Err(McpError::InvalidArgument(format!(
                        "Position {} is out of range (document has {} pages)",
                        pos, page_count
                    )));
                }
                Some(pos)
            }
            _ => None,
        };

        if let Some(pos) = position {
            doc.insert_page(pos);
            Ok(ToolResult::json(json!({
                "success": true,
                "message": format!("Added page at position {}", pos),
                "position": pos,
                "total_pages": doc.pages().len()
            })))
        } else {
            doc.add_page();
            Ok(ToolResult::json(json!({
                "success": true,
                "message": "Added page at end",
                "position": doc.pages().len() - 1,
                "total_pages": doc.pages().len()
            })))
        }
    }
}

/// Delete a page.
pub struct DeletePage;

#[derive(Debug, Deserialize)]
struct DeletePageArgs {
    page: usize,
}

impl Tool for DeletePage {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_delete_page".to_string(),
            description: "Delete a page from the document".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "page": {
                        "type": "integer",
                        "description": "Page index to delete (0-based)"
                    }
                },
                "required": ["page"]
            }),
        }
    }

    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: DeletePageArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let doc = session.document_mut()?;
        let page_count = doc.pages().len();

        if args.page >= page_count {
            return Err(McpError::InvalidPage(args.page, page_count));
        }

        if page_count == 1 {
            return Err(McpError::InvalidArgument(
                "Cannot delete the last page of a document".to_string(),
            ));
        }

        doc.remove_page(args.page);

        Ok(ToolResult::json(json!({
            "success": true,
            "message": format!("Deleted page {}", args.page),
            "total_pages": doc.pages().len()
        })))
    }
}

/// List all pages.
pub struct ListPages;

impl Tool for ListPages {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "repub_list_pages".to_string(),
            description: "List all pages in the document".to_string(),
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
                let layer_count = page.layers().len();

                json!({
                    "index": i,
                    "width": page.size.width,
                    "height": page.size.height,
                    "layers": layer_count,
                    "objects": obj_count
                })
            })
            .collect();

        Ok(ToolResult::json(json!({
            "total": pages.len(),
            "pages": pages
        })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_model::{Document, PageOrientation, PageSize};

    fn setup_session() -> Session {
        let mut session = Session::new();
        let doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        session.create_document(doc);
        session
    }

    #[test]
    fn test_add_page() {
        let tool = AddPage;
        let mut session = setup_session();

        let result = tool.execute(&mut session, json!({})).unwrap();
        assert!(!result.is_error);
        assert_eq!(session.document().unwrap().pages().len(), 2);
    }

    #[test]
    fn test_add_page_at_position() {
        let tool = AddPage;
        let mut session = setup_session();
        session.document_mut().unwrap().add_page(); // Now 2 pages

        let result = tool.execute(&mut session, json!({"position": 1})).unwrap();
        assert!(!result.is_error);
        assert_eq!(session.document().unwrap().pages().len(), 3);
    }

    #[test]
    fn test_list_pages() {
        let tool = ListPages;
        let mut session = setup_session();

        let result = tool.execute(&mut session, json!({})).unwrap();
        assert!(!result.is_error);
    }
}
