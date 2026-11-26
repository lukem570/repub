//! # Repub Templates
//!
//! Template system for creating documents from pre-designed layouts, targeting
//! the [`repub_model`] document model.
//!
//! ## Features
//!
//! - Template definition with placeholders
//! - Built-in template library
//! - Custom template creation
//! - Placeholder substitution
//! - Category organization
//!
//! The placeholder-description types are a serialized asset format and carry
//! `serde` derives. The model [`repub_model::Document`] a template builds is
//! deliberately not serializable here (serialization lives in
//! `repub-format-native`), so templates are constructed in code — as the
//! built-in library does. Loading a template whose layout is stored as a
//! document file should go through `repub-format-native` to deserialize the
//! document, then wrap it with its placeholder descriptions; that wiring is
//! not implemented yet.
//!
//! ## Usage
//!
//! ```rust
//! use repub_templates::{TemplateLibrary, PlaceholderValues};
//!
//! let library = TemplateLibrary::with_builtins();
//! let template = library.get("business-card-modern").unwrap();
//!
//! let mut values = PlaceholderValues::new();
//! values.set("name", "John Smith");
//! values.set("title", "Software Engineer");
//! values.set("email", "john@example.com");
//!
//! let document = template.apply(&values).unwrap();
//! assert_eq!(document.pages().len(), 1);
//! ```

#![warn(missing_docs)]

mod builtin;
mod error;
mod library;
mod placeholder;
mod template;

pub use builtin::*;
pub use error::{TemplateError, TemplateResult};
pub use library::{TemplateCategory, TemplateLibrary};
pub use placeholder::{
    ImageValue, Placeholder, PlaceholderType, PlaceholderValue, PlaceholderValues,
};
pub use template::{Template, TemplateMetadata};
