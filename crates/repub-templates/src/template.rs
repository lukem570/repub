//! Template definition and application.
//!
//! A [`Template`] pairs a document built from the model with a set of
//! placeholders. The placeholder-description types carry `serde` derives (they
//! are a serialized asset format), but the model [`Document`] is not itself
//! serializable, so `Template` is constructed in code (see the built-in
//! templates) rather than loaded from JSON.

use repub_model::shape::{ImageSource, ShapeKind};
use repub_model::text::StyledText;
use repub_model::Document;
use serde::{Deserialize, Serialize};

use crate::error::{TemplateError, TemplateResult};
use crate::library::TemplateCategory;
use crate::placeholder::{Placeholder, PlaceholderType, PlaceholderValue, PlaceholderValues};

/// A document template.
#[derive(Debug, Clone)]
pub struct Template {
    /// Template metadata
    pub metadata: TemplateMetadata,
    /// Placeholders in this template
    pub placeholders: Vec<Placeholder>,
    /// The template document
    pub document: Document,
    /// Preview thumbnail (base64 PNG)
    pub thumbnail: Option<String>,
}

/// Template metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMetadata {
    /// Unique template ID
    pub id: String,
    /// Display name
    pub name: String,
    /// Description
    pub description: String,
    /// Category
    pub category: TemplateCategory,
    /// Tags for searching
    pub tags: Vec<String>,
    /// Author
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Version
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "1.0".to_string()
}

impl Template {
    /// Create a new template from a document.
    pub fn new(id: &str, name: &str, category: TemplateCategory, document: Document) -> Self {
        Self {
            metadata: TemplateMetadata {
                id: id.to_string(),
                name: name.to_string(),
                description: String::new(),
                category,
                tags: Vec::new(),
                author: None,
                version: default_version(),
            },
            placeholders: Vec::new(),
            document,
            thumbnail: None,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, desc: &str) -> Self {
        self.metadata.description = desc.to_string();
        self
    }

    /// Add tags.
    pub fn with_tags(mut self, tags: &[&str]) -> Self {
        self.metadata.tags = tags.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Add a placeholder.
    pub fn add_placeholder(&mut self, placeholder: Placeholder) {
        self.placeholders.push(placeholder);
    }

    /// Get a placeholder by ID.
    pub fn get_placeholder(&self, id: &str) -> Option<&Placeholder> {
        self.placeholders.iter().find(|p| p.id == id)
    }

    /// Get required placeholders.
    pub fn required_placeholders(&self) -> Vec<&Placeholder> {
        self.placeholders.iter().filter(|p| p.required).collect()
    }

    /// Validate placeholder values.
    pub fn validate(&self, values: &PlaceholderValues) -> TemplateResult<()> {
        for placeholder in &self.placeholders {
            if placeholder.required {
                let has_value = values.has(&placeholder.id) || placeholder.default_value.is_some();
                if !has_value {
                    return Err(TemplateError::MissingRequired(placeholder.id.clone()));
                }
            }

            if let Some(value) = values.get(&placeholder.id) {
                if let PlaceholderValue::Text(text) = value {
                    if !placeholder.validate(text) {
                        return Err(TemplateError::InvalidValue(
                            placeholder.id.clone(),
                            format!("Value '{}' doesn't match required pattern", text),
                        ));
                    }
                }
            }
        }

        Ok(())
    }

    /// Apply placeholder values to create a new document.
    pub fn apply(&self, values: &PlaceholderValues) -> TemplateResult<Document> {
        self.validate(values)?;

        let mut doc = self.document.clone();

        for placeholder in &self.placeholders {
            let value: Option<PlaceholderValue> = values
                .get(&placeholder.id)
                .cloned()
                .or_else(|| placeholder.default_value.as_ref().map(|s| PlaceholderValue::Text(s.clone())));

            if let Some(value) = value {
                Self::apply_placeholder_to_doc(&mut doc, placeholder, &value)?;
            }
        }

        Ok(doc)
    }

    /// Apply a single placeholder value.
    fn apply_placeholder_to_doc(doc: &mut Document, placeholder: &Placeholder, value: &PlaceholderValue) -> TemplateResult<()> {
        let kind = doc
            .find_shape_mut(placeholder.object_id)
            .map(|(_, shape)| &mut shape.kind)
            .ok_or_else(|| TemplateError::ShapeNotFound(placeholder.id.clone()))?;

        match (placeholder.placeholder_type, value) {
            (
                PlaceholderType::Text
                | PlaceholderType::MultiLineText
                | PlaceholderType::Email
                | PlaceholderType::Phone
                | PlaceholderType::Url,
                PlaceholderValue::Text(text),
            ) => {
                if let ShapeKind::TextFrame(data) = kind {
                    data.content = StyledText::plain(text);
                }
            }
            (PlaceholderType::Date, PlaceholderValue::Date(date)) => {
                if let ShapeKind::TextFrame(data) = kind {
                    let formatted = date.format("%B %d, %Y").to_string();
                    data.content = StyledText::plain(&formatted);
                }
            }
            (PlaceholderType::Date, PlaceholderValue::Text(text)) => {
                if let ShapeKind::TextFrame(data) = kind {
                    data.content = StyledText::plain(text);
                }
            }
            (PlaceholderType::Number | PlaceholderType::Currency, PlaceholderValue::Number(n)) => {
                if let ShapeKind::TextFrame(data) = kind {
                    let formatted = if placeholder.placeholder_type == PlaceholderType::Currency {
                        format!("${:.2}", n)
                    } else {
                        format!("{}", n)
                    };
                    data.content = StyledText::plain(&formatted);
                }
            }
            (PlaceholderType::Image, PlaceholderValue::Image(img)) => {
                if let ShapeKind::Picture(data) = kind {
                    if img.source.starts_with("http://") || img.source.starts_with("https://") {
                        data.source = ImageSource::Url(img.source.clone());
                    } else {
                        data.source = ImageSource::File(img.source.clone());
                    }
                }
            }
            _ => {
                // Type mismatch - fall back to plain text
                if let ShapeKind::TextFrame(data) = kind {
                    let text = match value {
                        PlaceholderValue::Text(s) => s.clone(),
                        PlaceholderValue::Number(n) => n.to_string(),
                        PlaceholderValue::Date(d) => d.format("%Y-%m-%d").to_string(),
                        PlaceholderValue::Image(img) => img.source.clone(),
                    };
                    data.content = StyledText::plain(&text);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_model::shape::Shape;
    use repub_model::{PageOrientation, PageSize, Rect};

    fn create_test_template() -> Template {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);

        let name_frame = Shape::text_frame(Rect::new(72.0, 72.0, 200.0, 24.0));
        let name_id = name_frame.common.id;
        doc.pages_mut()[0].add_shape(name_frame);

        let mut template = Template::new("test-template", "Test Template", TemplateCategory::Business, doc);

        template.add_placeholder(
            Placeholder::text("name", "Full Name", name_id).with_default("John Doe").required(),
        );

        template
    }

    #[test]
    fn test_template_creation() {
        let template = create_test_template();
        assert_eq!(template.metadata.id, "test-template");
        assert_eq!(template.placeholders.len(), 1);
    }

    #[test]
    fn test_template_apply() {
        let template = create_test_template();

        let mut values = PlaceholderValues::new();
        values.set("name", "Jane Smith");

        let doc = template.apply(&values).unwrap();
        assert_eq!(doc.pages().len(), 1);
    }

    #[test]
    fn test_template_apply_sets_text() {
        let template = create_test_template();
        let name_id = template.placeholders[0].object_id;
        let mut values = PlaceholderValues::new();
        values.set("name", "Jane Smith");
        let doc = template.apply(&values).unwrap();
        let shape = doc.page(0).unwrap().get_shape(name_id).unwrap();
        match &shape.kind {
            ShapeKind::TextFrame(data) => assert_eq!(data.content.plain_text(), "Jane Smith"),
            _ => panic!("expected text frame"),
        }
    }

    #[test]
    fn test_template_validation() {
        let template = create_test_template();

        // Should pass with empty values because the required placeholder has a default
        let empty_values = PlaceholderValues::new();
        assert!(template.validate(&empty_values).is_ok());
    }
}
