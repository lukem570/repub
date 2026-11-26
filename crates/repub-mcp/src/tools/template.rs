//! Template MCP tools.

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use repub_templates::{PlaceholderValues, TemplateCategory, TemplateLibrary};

use super::Tool;
use crate::error::{McpError, McpResult};
use crate::protocol::{ToolDefinition, ToolResult};
use crate::session::Session;

/// List available templates.
pub struct ListTemplates;

#[derive(Debug, Deserialize)]
struct ListTemplatesArgs {
    category: Option<String>,
}

impl Tool for ListTemplates {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "list_templates".to_string(),
            description: "List available document templates".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "category": {
                        "type": "string",
                        "description": "Filter by category: business, marketing, publications, personal, office",
                        "enum": ["business", "marketing", "publications", "personal", "office", "custom"]
                    }
                }
            }),
        }
    }

    fn execute(&self, _session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: ListTemplatesArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let library = TemplateLibrary::with_builtins();

        let category = args
            .category
            .as_ref()
            .and_then(|c| match c.to_lowercase().as_str() {
                "business" => Some(TemplateCategory::Business),
                "marketing" => Some(TemplateCategory::Marketing),
                "publications" => Some(TemplateCategory::Publications),
                "personal" => Some(TemplateCategory::Personal),
                "office" => Some(TemplateCategory::Office),
                "custom" => Some(TemplateCategory::Custom),
                _ => None,
            });

        let templates: Vec<_> = if let Some(cat) = category {
            library.by_category(cat)
        } else {
            library.all().collect()
        };

        let items: Vec<_> = templates
            .iter()
            .map(|t| {
                json!({
                    "id": t.metadata.id,
                    "name": t.metadata.name,
                    "category": t.metadata.category.name(),
                    "description": t.metadata.description,
                    "tags": t.metadata.tags,
                    "placeholders": t.placeholders.len(),
                })
            })
            .collect();

        Ok(ToolResult::json(json!(items)))
    }
}

/// Get template details.
pub struct GetTemplate;

#[derive(Debug, Deserialize)]
struct GetTemplateArgs {
    id: String,
}

impl Tool for GetTemplate {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "get_template".to_string(),
            description: "Get details about a specific template".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "Template ID"
                    }
                },
                "required": ["id"]
            }),
        }
    }

    fn execute(&self, _session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: GetTemplateArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let library = TemplateLibrary::with_builtins();

        let template = library
            .get(&args.id)
            .ok_or_else(|| McpError::InvalidArgument(format!("Template not found: {}", args.id)))?;

        let placeholders: Vec<_> = template
            .placeholders
            .iter()
            .map(|p| {
                json!({
                    "id": p.id,
                    "name": p.name,
                    "type": format!("{:?}", p.placeholder_type),
                    "required": p.required,
                    "default": p.default_value,
                    "description": p.description,
                    "example": p.example,
                })
            })
            .collect();

        let info = json!({
            "id": template.metadata.id,
            "name": template.metadata.name,
            "category": template.metadata.category.name(),
            "description": template.metadata.description,
            "tags": template.metadata.tags,
            "version": template.metadata.version,
            "placeholders": placeholders,
        });

        Ok(ToolResult::json(info))
    }
}

/// Create document from template.
pub struct CreateFromTemplate;

#[derive(Debug, Deserialize)]
struct CreateFromTemplateArgs {
    template_id: String,
    placeholders: Option<std::collections::HashMap<String, String>>,
}

impl Tool for CreateFromTemplate {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "create_from_template".to_string(),
            description: "Create a new document from a template with placeholder values"
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "template_id": {
                        "type": "string",
                        "description": "Template ID to use"
                    },
                    "placeholders": {
                        "type": "object",
                        "description": "Key-value pairs for placeholder substitution",
                        "additionalProperties": { "type": "string" }
                    }
                },
                "required": ["template_id"]
            }),
        }
    }

    fn execute(&self, session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: CreateFromTemplateArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let library = TemplateLibrary::with_builtins();

        let template = library.get(&args.template_id).ok_or_else(|| {
            McpError::InvalidArgument(format!("Template not found: {}", args.template_id))
        })?;

        // Build placeholder values
        let values: PlaceholderValues = args.placeholders.unwrap_or_default().into();

        // Apply template
        let document = template
            .apply(&values)
            .map_err(|e| McpError::Internal(e.to_string()))?;

        // Store in session
        session.create_document(document);

        let info = json!({
            "template": template.metadata.name,
            "message": format!("Created document from template '{}'", template.metadata.id),
        });

        Ok(ToolResult::json(info))
    }
}

/// Search templates.
pub struct SearchTemplates;

#[derive(Debug, Deserialize)]
struct SearchTemplatesArgs {
    query: String,
}

impl Tool for SearchTemplates {
    fn definition(&self) -> ToolDefinition {
        ToolDefinition {
            name: "search_templates".to_string(),
            description: "Search templates by name, description, or tags".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    fn execute(&self, _session: &mut Session, args: Value) -> McpResult<ToolResult> {
        let args: SearchTemplatesArgs =
            serde_json::from_value(args).map_err(|e| McpError::InvalidArgument(e.to_string()))?;

        let library = TemplateLibrary::with_builtins();
        let results = library.search(&args.query);

        let items: Vec<_> = results
            .iter()
            .map(|t| {
                json!({
                    "id": t.metadata.id,
                    "name": t.metadata.name,
                    "category": t.metadata.category.name(),
                    "description": t.metadata.description,
                })
            })
            .collect();

        Ok(ToolResult::json(json!(items)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_templates_tool() {
        let tool = ListTemplates;
        let mut session = Session::new();

        let result = tool.execute(&mut session, json!({})).unwrap();
        assert!(!result.is_error);
    }

    #[test]
    fn test_get_template_tool() {
        let tool = GetTemplate;
        let mut session = Session::new();

        let result = tool
            .execute(
                &mut session,
                json!({
                    "id": "business-card-modern"
                }),
            )
            .unwrap();
        assert!(!result.is_error);
    }

    #[test]
    fn test_create_from_template_tool() {
        let tool = CreateFromTemplate;
        let mut session = Session::new();

        let result = tool
            .execute(
                &mut session,
                json!({
                    "template_id": "business-card-modern",
                    "placeholders": {
                        "name": "Test User",
                        "title": "Developer"
                    }
                }),
            )
            .unwrap();
        assert!(!result.is_error);
    }

    #[test]
    fn test_search_templates_tool() {
        let tool = SearchTemplates;
        let mut session = Session::new();

        let result = tool
            .execute(
                &mut session,
                json!({
                    "query": "business"
                }),
            )
            .unwrap();
        assert!(!result.is_error);
    }
}
