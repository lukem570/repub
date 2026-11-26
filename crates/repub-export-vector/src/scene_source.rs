//! Shared helper: compose document pages into scenes.

use repub_fonts::FontProvider;
use repub_model::Document;
use repub_scene::{compose, Scene};

/// Compose the given page indices of `doc` into scenes, in order.
pub fn compose_pages(doc: &Document, pages: &[usize], fonts: &dyn FontProvider) -> Vec<Scene> {
    pages.iter().map(|&p| compose(doc, p, fonts)).collect()
}
