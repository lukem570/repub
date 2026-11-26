//! MCP resource implementations.

use serde_json::{json, Value};

use crate::error::{McpError, McpResult};
use crate::protocol::{ResourceContent, ResourceDefinition};
use crate::session::Session;

/// Get available resources.
pub fn list_resources() -> Vec<ResourceDefinition> {
    vec![
        ResourceDefinition {
            uri: "repub://document/current".to_string(),
            name: "Current Document".to_string(),
            description: Some("The currently open document structure".to_string()),
            mime_type: Some("application/json".to_string()),
        },
        ResourceDefinition {
            uri: "repub://document/info".to_string(),
            name: "Document Info".to_string(),
            description: Some("Summary information about the current document".to_string()),
            mime_type: Some("application/json".to_string()),
        },
    ]
}

/// Read a resource by URI.
pub fn read_resource(session: &Session, uri: &str) -> McpResult<ResourceContent> {
    match uri {
        "repub://document/current" => {
            let doc = session.document()?;
            // The model document is serialization-free; serialize it through the
            // native-format DTO layer (pretty JSON).
            let bytes =
                repub_format_native::save_bytes(doc, &repub_format_native::SaveOptions::default())?;
            let json = String::from_utf8(bytes).map_err(|e| McpError::Internal(e.to_string()))?;

            Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: Some(json),
                blob: None,
            })
        }

        "repub://document/info" => {
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

            let info = json!({
                "pages": pages.len(),
                "page_details": pages,
                "path": session.path.as_ref().map(|p| p.to_string_lossy().to_string()),
                "has_unsaved_changes": session.dirty
            });

            Ok(ResourceContent {
                uri: uri.to_string(),
                mime_type: Some("application/json".to_string()),
                text: Some(serde_json::to_string_pretty(&info).unwrap()),
                blob: None,
            })
        }

        _ => Err(McpError::UnknownResource(uri.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_model::{Document, PageOrientation, PageSize};

    #[test]
    fn test_list_resources() {
        let resources = list_resources();
        assert!(!resources.is_empty());
    }

    #[test]
    fn test_read_document_info() {
        let mut session = Session::new();
        let doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        session.create_document(doc);

        let result = read_resource(&session, "repub://document/info");
        assert!(result.is_ok());
    }
}
