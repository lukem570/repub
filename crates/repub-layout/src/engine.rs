//! Main layout engine.

use std::sync::Arc;

use repub_model::document::{Document, Page};
use repub_model::shape::{Shape, ShapeKind, TableData, TextFrameData};
use repub_model::text::StyledText;
use repub_model::{PageOrientation, PageSize};

use repub_fonts::FontProvider;

use crate::cache::{DirtyTracker, LayoutCache};
use crate::object::ObjectLayout;
use crate::page::PageLayout;
use crate::table::{TableLayout, TableLayouter};
use crate::text::{TextFrameLayout, TextLayouter};

/// The main layout engine.
pub struct LayoutEngine<F: FontProvider> {
    /// Font provider
    font_provider: Arc<F>,
    /// Layout cache
    cache: LayoutCache,
    /// Dirty tracker
    dirty: DirtyTracker,
    /// Table layouter
    table_layouter: TableLayouter,
}

impl<F: FontProvider> LayoutEngine<F> {
    /// Create a new layout engine.
    pub fn new(font_provider: Arc<F>) -> Self {
        Self {
            font_provider,
            cache: LayoutCache::new(1000),
            dirty: DirtyTracker::new(),
            table_layouter: TableLayouter::new(),
        }
    }

    /// Create with a custom cache size.
    pub fn with_cache_size(font_provider: Arc<F>, cache_size: usize) -> Self {
        Self {
            font_provider,
            cache: LayoutCache::new(cache_size),
            dirty: DirtyTracker::new(),
            table_layouter: TableLayouter::new(),
        }
    }

    /// Get the font provider.
    pub fn font_provider(&self) -> &F {
        &self.font_provider
    }

    /// Get the cache.
    pub fn cache(&self) -> &LayoutCache {
        &self.cache
    }

    /// Get mutable access to the cache.
    pub fn cache_mut(&mut self) -> &mut LayoutCache {
        &mut self.cache
    }

    /// Get the dirty tracker.
    pub fn dirty_tracker(&self) -> &DirtyTracker {
        &self.dirty
    }

    /// Get mutable access to dirty tracker.
    pub fn dirty_tracker_mut(&mut self) -> &mut DirtyTracker {
        &mut self.dirty
    }

    /// Layout an entire document.
    pub fn layout_document(&mut self, document: &Document) -> Vec<PageLayout> {
        document
            .pages()
            .iter()
            .map(|page| {
                self.layout_page(
                    page,
                    document.default_page_size,
                    document.default_orientation,
                )
            })
            .collect()
    }

    /// Layout a single page.
    pub fn layout_page(
        &mut self,
        page: &Page,
        size: PageSize,
        orientation: PageOrientation,
    ) -> PageLayout {
        let layout = PageLayout::from_page(page, size, orientation);

        // Layout shapes on each layer
        for layer in page.layers() {
            if !layer.visible {
                continue;
            }

            for shape in layer.shapes() {
                self.layout_shape(shape);
            }
        }

        layout
    }

    /// Layout a single shape, caching text-frame and table layouts.
    pub fn layout_shape(&mut self, shape: &Shape) -> ObjectLayout {
        match &shape.kind {
            ShapeKind::TextFrame(data) => {
                let _ = self.layout_text_frame(shape, data);
            }
            ShapeKind::Table(data) => {
                let _ = self.layout_table(shape, data);
            }
            _ => {}
        }

        ObjectLayout::from_shape(shape)
    }

    /// Layout a text frame given its shape (for bounds and id) and its data.
    pub fn layout_text_frame(
        &mut self,
        shape: &Shape,
        data: &TextFrameData,
    ) -> TextFrameLayout {
        let object_id = shape.common.id;
        let version = self.cache.version();

        // Check cache
        if let Some(cached) = self.cache.get_text_frame(object_id, version) {
            return cached.clone();
        }

        // Compute layout
        let layouter = TextLayouter::new(self.font_provider.as_ref());
        let layout = layouter.layout_frame(shape, data);

        // Cache result
        self.cache
            .insert_text_frame(object_id, version, layout.clone());

        layout
    }

    /// Layout a table given its shape (for bounds and id) and its data.
    pub fn layout_table(&mut self, shape: &Shape, data: &TableData) -> TableLayout {
        let object_id = shape.common.id;
        let version = self.cache.version();

        // Check cache
        if let Some(cached) = self.cache.get_table(object_id, version) {
            return cached.clone();
        }

        // Compute layout
        let layout = self.table_layouter.layout(shape.common.bounds, data);

        // Cache result
        self.cache.insert_table(object_id, version, layout.clone());

        layout
    }

    /// Invalidate layout for a shape.
    pub fn invalidate_object(&mut self, shape: &Shape) {
        self.cache.invalidate(shape.id());
        self.dirty.mark_object_dirty(shape.id());
    }

    /// Invalidate all layouts.
    pub fn invalidate_all(&mut self) {
        self.cache.invalidate_all();
        self.dirty.mark_all_dirty();
    }

    /// Perform incremental layout (only dirty objects).
    pub fn layout_incremental(&mut self, document: &Document) {
        if !self.dirty.has_dirty() {
            return;
        }

        for page in document.pages() {
            if self.dirty.is_page_dirty(page.id) {
                // Relayout entire page
                self.layout_page(
                    page,
                    document.default_page_size,
                    document.default_orientation,
                );
            } else {
                // Check individual shapes
                for layer in page.layers() {
                    for shape in layer.shapes() {
                        if self.dirty.is_object_dirty(shape.id()) {
                            self.layout_shape(shape);
                        }
                    }
                }
            }
        }

        self.dirty.clear();
    }

    /// Get cache statistics.
    pub fn cache_stats(&self) -> crate::cache::CacheStats {
        self.cache.stats()
    }
}

/// Result of laying out linked text frames.
#[derive(Debug, Clone)]
pub struct LinkedFrameResult {
    /// Layout for each frame
    pub layouts: Vec<TextFrameLayout>,
    /// Indices of frames that have content
    pub frames_with_content: Vec<usize>,
    /// Whether all text was laid out (no overflow from last frame)
    pub complete: bool,
    /// Total line count across all frames
    pub total_lines: usize,
}

/// Layout linked text frames with cross-page flow tracking.
///
/// Each frame is a text-frame [`Shape`]; non-text-frame shapes are skipped. A
/// frame that receives overflow from the previous one is laid out with the
/// overflow text and marked as a continuation.
pub fn layout_linked_frames<F: FontProvider>(
    engine: &mut LayoutEngine<F>,
    frames: &[&Shape],
) -> LinkedFrameResult {
    if frames.is_empty() {
        return LinkedFrameResult {
            layouts: vec![],
            frames_with_content: vec![],
            complete: true,
            total_lines: 0,
        };
    }

    let mut layouts = Vec::new();
    let mut frames_with_content = Vec::new();
    let mut overflow_text = String::new();
    let mut total_lines = 0;

    for (frame_idx, frame) in frames.iter().enumerate() {
        // Only text-frame shapes carry text-frame data
        let ShapeKind::TextFrame(data) = &frame.kind else {
            continue;
        };

        // Build the data to lay out: inject overflow text for continuations
        let data_to_layout = if !overflow_text.is_empty() && frame_idx > 0 {
            let mut modified = data.clone();
            modified.content = StyledText::plain(&overflow_text);
            modified.is_continuation = true;
            modified
        } else {
            data.clone()
        };

        let mut layout = engine.layout_text_frame(frame, &data_to_layout);

        // Set frame index for tracking
        layout.frame_index = frame_idx;

        // Update line frame indices
        for column in &mut layout.columns {
            for line in &mut column.lines {
                line.frame_index = frame_idx;
            }
        }

        // Track if this frame has content
        if layout.has_content {
            frames_with_content.push(frame_idx);
        }

        // Count lines
        total_lines += layout.line_count();

        if layout.has_overflow {
            if let Some(overflow_start) = layout.overflow_start {
                // Get overflow text for next frame
                let full_text = data_to_layout.content.plain_text();
                if overflow_start < full_text.len() {
                    overflow_text = full_text[overflow_start..].to_string();
                } else {
                    overflow_text.clear();
                }
            }
        } else {
            overflow_text.clear();
        }

        layouts.push(layout);
    }

    let complete = overflow_text.is_empty();

    LinkedFrameResult {
        layouts,
        frames_with_content,
        complete,
        total_lines,
    }
}

/// Layout linked text frames (simple version returning just layouts).
pub fn layout_linked_frames_simple<F: FontProvider>(
    engine: &mut LayoutEngine<F>,
    frames: &[&Shape],
) -> Vec<TextFrameLayout> {
    layout_linked_frames(engine, frames).layouts
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_fonts::SystemFontProvider;
    use repub_geometry::Rect;
    use repub_model::shape::Shape;

    #[test]
    fn test_cache_stats() {
        let provider = SystemFontProvider::default();
        let engine = LayoutEngine::new(Arc::new(provider));
        let stats = engine.cache_stats();

        assert_eq!(stats.size, 0);
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
    }

    #[test]
    fn test_linked_frame_result_empty() {
        let provider = SystemFontProvider::default();
        let mut engine = LayoutEngine::new(Arc::new(provider));

        let result = layout_linked_frames(&mut engine, &[]);

        assert!(result.layouts.is_empty());
        assert!(result.frames_with_content.is_empty());
        assert!(result.complete);
        assert_eq!(result.total_lines, 0);
    }

    #[test]
    fn test_linked_frame_result_single_frame() {
        let provider = SystemFontProvider::default();
        let mut engine = LayoutEngine::new(Arc::new(provider));

        let frame = Shape::text_frame_with_text(Rect::new(0.0, 0.0, 200.0, 400.0), "Hello World");
        let frames: Vec<&Shape> = vec![&frame];

        let result = layout_linked_frames(&mut engine, &frames);

        assert_eq!(result.layouts.len(), 1);
        assert!(result.complete);
        assert_eq!(result.layouts[0].frame_index, 0);
    }

    #[test]
    fn test_linked_frame_result_multiple_frames() {
        let provider = SystemFontProvider::default();
        let mut engine = LayoutEngine::new(Arc::new(provider));

        let frame1 =
            Shape::text_frame_with_text(Rect::new(0.0, 0.0, 200.0, 50.0), "Line 1\nLine 2\nLine 3");
        let frame2 = Shape::text_frame(Rect::new(0.0, 0.0, 200.0, 400.0));
        let frames: Vec<&Shape> = vec![&frame1, &frame2];

        let result = layout_linked_frames(&mut engine, &frames);

        assert_eq!(result.layouts.len(), 2);
        assert_eq!(result.layouts[0].frame_index, 0);
        assert_eq!(result.layouts[1].frame_index, 1);
    }

    #[test]
    fn test_linked_frames_simple() {
        let provider = SystemFontProvider::default();
        let mut engine = LayoutEngine::new(Arc::new(provider));

        let frame = Shape::text_frame_with_text(Rect::new(0.0, 0.0, 200.0, 400.0), "Test");
        let frames: Vec<&Shape> = vec![&frame];

        let layouts = layout_linked_frames_simple(&mut engine, &frames);

        assert_eq!(layouts.len(), 1);
    }

    #[test]
    fn test_linked_frame_content_tracking() {
        let provider = SystemFontProvider::default();
        let mut engine = LayoutEngine::new(Arc::new(provider));

        let frame1 = Shape::text_frame_with_text(Rect::new(0.0, 0.0, 200.0, 400.0), "Content");
        let frame2 = Shape::text_frame(Rect::new(0.0, 0.0, 200.0, 400.0)); // Empty frame
        let frames: Vec<&Shape> = vec![&frame1, &frame2];

        let result = layout_linked_frames(&mut engine, &frames);

        // First frame has content
        assert!(result.frames_with_content.contains(&0));
    }
}
