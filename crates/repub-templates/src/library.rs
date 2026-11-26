//! Template library management.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::builtin;
use crate::template::Template;

/// Template categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateCategory {
    /// Business documents (cards, letterhead, envelopes)
    Business,
    /// Marketing materials (flyers, brochures, posters)
    Marketing,
    /// Publications (newsletters, booklets)
    Publications,
    /// Personal (invitations, cards, calendars)
    Personal,
    /// Office (certificates, labels, signs)
    Office,
    /// Custom/user-created
    Custom,
}

impl TemplateCategory {
    /// Get the display name.
    pub fn name(&self) -> &'static str {
        match self {
            TemplateCategory::Business => "Business",
            TemplateCategory::Marketing => "Marketing",
            TemplateCategory::Publications => "Publications",
            TemplateCategory::Personal => "Personal",
            TemplateCategory::Office => "Office",
            TemplateCategory::Custom => "Custom",
        }
    }

    /// Get all categories.
    pub fn all() -> &'static [TemplateCategory] {
        &[
            TemplateCategory::Business,
            TemplateCategory::Marketing,
            TemplateCategory::Publications,
            TemplateCategory::Personal,
            TemplateCategory::Office,
            TemplateCategory::Custom,
        ]
    }
}

/// Template library containing all available templates.
#[derive(Debug, Default)]
pub struct TemplateLibrary {
    templates: HashMap<String, Template>,
}

impl TemplateLibrary {
    /// Create a new empty library.
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Create a library with built-in templates.
    pub fn with_builtins() -> Self {
        let mut library = Self::new();
        library.load_builtins();
        library
    }

    /// Load built-in templates.
    pub fn load_builtins(&mut self) {
        // Business templates
        self.add(builtin::business_card_modern());
        self.add(builtin::business_card_classic());
        self.add(builtin::letterhead_corporate());

        // Marketing templates
        self.add(builtin::flyer_event());
        self.add(builtin::brochure_trifold());

        // Personal templates
        self.add(builtin::invitation_party());
        self.add(builtin::greeting_card());

        // Office templates
        self.add(builtin::certificate_achievement());
        self.add(builtin::label_address());
    }

    /// Add a template to the library.
    pub fn add(&mut self, template: Template) {
        self.templates
            .insert(template.metadata.id.clone(), template);
    }

    /// Remove a template from the library.
    pub fn remove(&mut self, id: &str) -> Option<Template> {
        self.templates.remove(id)
    }

    /// Get a template by ID.
    pub fn get(&self, id: &str) -> Option<&Template> {
        self.templates.get(id)
    }

    /// Get all templates.
    pub fn all(&self) -> impl Iterator<Item = &Template> {
        self.templates.values()
    }

    /// Get templates by category.
    pub fn by_category(&self, category: TemplateCategory) -> Vec<&Template> {
        self.templates
            .values()
            .filter(|t| t.metadata.category == category)
            .collect()
    }

    /// Search templates by name or tags.
    pub fn search(&self, query: &str) -> Vec<&Template> {
        let query_lower = query.to_lowercase();
        self.templates
            .values()
            .filter(|t| {
                t.metadata.name.to_lowercase().contains(&query_lower)
                    || t.metadata.description.to_lowercase().contains(&query_lower)
                    || t.metadata
                        .tags
                        .iter()
                        .any(|tag| tag.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Get template count.
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Check if library is empty.
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }

    /// Get category counts.
    pub fn category_counts(&self) -> HashMap<TemplateCategory, usize> {
        let mut counts = HashMap::new();
        for category in TemplateCategory::all() {
            counts.insert(*category, 0);
        }

        for template in self.templates.values() {
            *counts.entry(template.metadata.category).or_insert(0) += 1;
        }

        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_builtins() {
        let library = TemplateLibrary::with_builtins();
        assert!(!library.is_empty());
        assert!(library.len() >= 9); // At least 9 built-in templates
    }

    #[test]
    fn test_library_search() {
        let library = TemplateLibrary::with_builtins();

        let results = library.search("business");
        assert!(!results.is_empty());

        let results = library.search("card");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_library_by_category() {
        let library = TemplateLibrary::with_builtins();

        let business = library.by_category(TemplateCategory::Business);
        assert!(!business.is_empty());
    }

    #[test]
    fn test_category_names() {
        assert_eq!(TemplateCategory::Business.name(), "Business");
        assert_eq!(TemplateCategory::Marketing.name(), "Marketing");
    }
}
