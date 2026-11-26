//! Placeholder types and values.

use std::collections::HashMap;

use repub_model::id::ObjectId;
use serde::{Deserialize, Serialize};

/// Serialize an [`ObjectId`] as its raw numeric value. The model's IDs are not
/// themselves `serde`-serializable, so the template-description format stores
/// the raw value and restores it with [`ObjectId::from_raw`].
mod object_id_serde {
    use super::ObjectId;
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S: Serializer>(id: &ObjectId, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(id.raw())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<ObjectId, D::Error> {
        let value = u64::deserialize(d)?;
        Ok(ObjectId::from_raw(value))
    }
}

/// A placeholder in a template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Placeholder {
    /// Unique identifier for this placeholder
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Description/help text
    pub description: String,
    /// ID of the shape this placeholder targets
    #[serde(with = "object_id_serde")]
    pub object_id: ObjectId,
    /// Type of placeholder
    pub placeholder_type: PlaceholderType,
    /// Default value
    pub default_value: Option<String>,
    /// Is this placeholder required?
    pub required: bool,
    /// Validation pattern (regex)
    pub validation: Option<String>,
    /// Example value for UI hints
    pub example: Option<String>,
}

impl Placeholder {
    /// Create a new text placeholder.
    pub fn text(id: &str, name: &str, object_id: ObjectId) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            object_id,
            placeholder_type: PlaceholderType::Text,
            default_value: None,
            required: false,
            validation: None,
            example: None,
        }
    }

    /// Create a new image placeholder.
    pub fn image(id: &str, name: &str, object_id: ObjectId) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: String::new(),
            object_id,
            placeholder_type: PlaceholderType::Image,
            default_value: None,
            required: false,
            validation: None,
            example: None,
        }
    }

    /// Set the description.
    pub fn with_description(mut self, desc: &str) -> Self {
        self.description = desc.to_string();
        self
    }

    /// Set as required.
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Set default value.
    pub fn with_default(mut self, value: &str) -> Self {
        self.default_value = Some(value.to_string());
        self
    }

    /// Set example value.
    pub fn with_example(mut self, example: &str) -> Self {
        self.example = Some(example.to_string());
        self
    }

    /// Validate a value against this placeholder's rules.
    pub fn validate(&self, value: &str) -> bool {
        if let Some(ref pattern) = self.validation {
            if let Ok(re) = regex::Regex::new(pattern) {
                return re.is_match(value);
            }
        }
        true
    }
}

/// Types of placeholders.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaceholderType {
    /// Single line text
    Text,
    /// Multi-line text
    MultiLineText,
    /// Image (file path or URL)
    Image,
    /// Date (formatted)
    Date,
    /// Email address
    Email,
    /// Phone number
    Phone,
    /// URL
    Url,
    /// Number
    Number,
    /// Currency
    Currency,
    /// Color (hex)
    Color,
}

impl PlaceholderType {
    /// Get a default validation pattern for this type.
    pub fn default_validation(&self) -> Option<&'static str> {
        match self {
            PlaceholderType::Email => Some(r"^[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}$"),
            PlaceholderType::Phone => Some(r"^[\d\s\-\(\)\+]+$"),
            PlaceholderType::Url => Some(r"^https?://"),
            PlaceholderType::Color => Some(r"^#?[0-9a-fA-F]{6}$"),
            _ => None,
        }
    }
}

/// Collection of placeholder values.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlaceholderValues {
    values: HashMap<String, PlaceholderValue>,
}

/// A single placeholder value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PlaceholderValue {
    /// Text value
    Text(String),
    /// Image data (base64 or path)
    Image(ImageValue),
    /// Date value
    Date(chrono::NaiveDate),
    /// Number value
    Number(f64),
}

/// Image value for placeholders.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageValue {
    /// Image source (path, URL, or base64)
    pub source: String,
    /// Alt text
    pub alt: Option<String>,
}

impl PlaceholderValues {
    /// Create a new empty placeholder values collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a text value.
    pub fn set(&mut self, id: &str, value: &str) {
        self.values.insert(id.to_string(), PlaceholderValue::Text(value.to_string()));
    }

    /// Set an image value.
    pub fn set_image(&mut self, id: &str, source: &str, alt: Option<&str>) {
        self.values.insert(
            id.to_string(),
            PlaceholderValue::Image(ImageValue { source: source.to_string(), alt: alt.map(|s| s.to_string()) }),
        );
    }

    /// Set a date value.
    pub fn set_date(&mut self, id: &str, date: chrono::NaiveDate) {
        self.values.insert(id.to_string(), PlaceholderValue::Date(date));
    }

    /// Set a number value.
    pub fn set_number(&mut self, id: &str, value: f64) {
        self.values.insert(id.to_string(), PlaceholderValue::Number(value));
    }

    /// Get a value.
    pub fn get(&self, id: &str) -> Option<&PlaceholderValue> {
        self.values.get(id)
    }

    /// Get a text value.
    pub fn get_text(&self, id: &str) -> Option<&str> {
        match self.values.get(id) {
            Some(PlaceholderValue::Text(s)) => Some(s),
            _ => None,
        }
    }

    /// Check if a value is set.
    pub fn has(&self, id: &str) -> bool {
        self.values.contains_key(id)
    }

    /// Remove a value.
    pub fn remove(&mut self, id: &str) {
        self.values.remove(id);
    }

    /// Get all keys.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.values.keys().map(|s| s.as_str())
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Get count.
    pub fn len(&self) -> usize {
        self.values.len()
    }
}

impl From<HashMap<String, String>> for PlaceholderValues {
    fn from(map: HashMap<String, String>) -> Self {
        let mut values = PlaceholderValues::new();
        for (k, v) in map {
            values.set(&k, &v);
        }
        values
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder_values() {
        let mut values = PlaceholderValues::new();
        values.set("name", "John");
        values.set("email", "john@example.com");

        assert_eq!(values.get_text("name"), Some("John"));
        assert_eq!(values.get_text("email"), Some("john@example.com"));
        assert_eq!(values.get_text("missing"), None);
        assert_eq!(values.len(), 2);
    }

    #[test]
    fn test_placeholder_validation() {
        let email = Placeholder {
            id: "email".to_string(),
            name: "Email".to_string(),
            description: String::new(),
            object_id: ObjectId::new(),
            placeholder_type: PlaceholderType::Email,
            default_value: None,
            required: true,
            validation: PlaceholderType::Email.default_validation().map(|s| s.to_string()),
            example: None,
        };

        assert!(email.validate("test@example.com"));
        assert!(!email.validate("invalid"));
    }
}
