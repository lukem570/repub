//! Layout caching for incremental updates.

use std::collections::HashMap;

use repub_model::id::{ObjectId, PageId};

use crate::table::TableLayout;
use crate::text::TextFrameLayout;

/// Cache key for layout entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LayoutCacheKey {
    /// Object ID
    pub object_id: ObjectId,
    /// Version/generation number
    pub version: u64,
}

impl LayoutCacheKey {
    /// Create a new cache key.
    pub fn new(object_id: ObjectId, version: u64) -> Self {
        Self { object_id, version }
    }
}

/// A cached layout entry.
#[derive(Debug, Clone)]
pub enum CachedLayout {
    /// Text frame layout
    TextFrame(TextFrameLayout),
    /// Table layout
    Table(TableLayout),
}

/// Layout cache for storing computed layouts.
pub struct LayoutCache {
    /// Cached layouts by key
    layouts: HashMap<LayoutCacheKey, CachedLayout>,
    /// Maximum cache size
    max_size: usize,
    /// Cache hits counter
    hits: u64,
    /// Cache misses counter
    misses: u64,
    /// Current version for invalidation
    version: u64,
}

impl Default for LayoutCache {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl LayoutCache {
    /// Create a new layout cache with the given maximum size.
    pub fn new(max_size: usize) -> Self {
        Self {
            layouts: HashMap::new(),
            max_size,
            hits: 0,
            misses: 0,
            version: 0,
        }
    }

    /// Get a cached layout.
    pub fn get(&mut self, key: &LayoutCacheKey) -> Option<&CachedLayout> {
        if self.layouts.contains_key(key) {
            self.hits += 1;
            self.layouts.get(key)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insert a layout into the cache.
    pub fn insert(&mut self, key: LayoutCacheKey, layout: CachedLayout) {
        // Evict if at capacity
        if self.layouts.len() >= self.max_size {
            self.evict_oldest();
        }

        self.layouts.insert(key, layout);
    }

    /// Get a text frame layout.
    pub fn get_text_frame(
        &mut self,
        object_id: ObjectId,
        version: u64,
    ) -> Option<&TextFrameLayout> {
        let key = LayoutCacheKey::new(object_id, version);
        self.get(&key).and_then(|l| {
            if let CachedLayout::TextFrame(tf) = l {
                Some(tf)
            } else {
                None
            }
        })
    }

    /// Insert a text frame layout.
    pub fn insert_text_frame(
        &mut self,
        object_id: ObjectId,
        version: u64,
        layout: TextFrameLayout,
    ) {
        let key = LayoutCacheKey::new(object_id, version);
        self.insert(key, CachedLayout::TextFrame(layout));
    }

    /// Get a table layout.
    pub fn get_table(&mut self, object_id: ObjectId, version: u64) -> Option<&TableLayout> {
        let key = LayoutCacheKey::new(object_id, version);
        self.get(&key).and_then(|l| {
            if let CachedLayout::Table(t) = l {
                Some(t)
            } else {
                None
            }
        })
    }

    /// Insert a table layout.
    pub fn insert_table(&mut self, object_id: ObjectId, version: u64, layout: TableLayout) {
        let key = LayoutCacheKey::new(object_id, version);
        self.insert(key, CachedLayout::Table(layout));
    }

    /// Invalidate a specific object's layout.
    pub fn invalidate(&mut self, object_id: ObjectId) {
        self.layouts.retain(|k, _| k.object_id != object_id);
    }

    /// Invalidate all layouts.
    pub fn invalidate_all(&mut self) {
        self.layouts.clear();
        self.version += 1;
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.layouts.len(),
            max_size: self.max_size,
            hits: self.hits,
            misses: self.misses,
            hit_rate: if self.hits + self.misses > 0 {
                self.hits as f64 / (self.hits + self.misses) as f64
            } else {
                0.0
            },
        }
    }

    /// Evict the oldest entry (simple FIFO for now).
    fn evict_oldest(&mut self) {
        if let Some(key) = self.layouts.keys().next().copied() {
            self.layouts.remove(&key);
        }
    }

    /// Current version number.
    pub fn version(&self) -> u64 {
        self.version
    }
}

/// Cache statistics.
#[derive(Debug, Clone, Copy)]
pub struct CacheStats {
    /// Current cache size
    pub size: usize,
    /// Maximum cache size
    pub max_size: usize,
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Hit rate (0.0 to 1.0)
    pub hit_rate: f64,
}

/// Dirty tracking for incremental layout.
pub struct DirtyTracker {
    /// Objects that need relayout
    dirty_objects: Vec<ObjectId>,
    /// Pages that need relayout
    dirty_pages: Vec<PageId>,
    /// Whether the entire document is dirty
    all_dirty: bool,
}

impl Default for DirtyTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl DirtyTracker {
    /// Create a new dirty tracker.
    pub fn new() -> Self {
        Self {
            dirty_objects: Vec::new(),
            dirty_pages: Vec::new(),
            all_dirty: true, // Start dirty
        }
    }

    /// Mark an object as dirty.
    pub fn mark_object_dirty(&mut self, object_id: ObjectId) {
        if !self.dirty_objects.contains(&object_id) {
            self.dirty_objects.push(object_id);
        }
    }

    /// Mark a page as dirty.
    pub fn mark_page_dirty(&mut self, page_id: PageId) {
        if !self.dirty_pages.contains(&page_id) {
            self.dirty_pages.push(page_id);
        }
    }

    /// Mark everything as dirty.
    pub fn mark_all_dirty(&mut self) {
        self.all_dirty = true;
    }

    /// Check if an object is dirty.
    pub fn is_object_dirty(&self, object_id: ObjectId) -> bool {
        self.all_dirty || self.dirty_objects.contains(&object_id)
    }

    /// Check if a page is dirty.
    pub fn is_page_dirty(&self, page_id: PageId) -> bool {
        self.all_dirty || self.dirty_pages.contains(&page_id)
    }

    /// Check if anything is dirty.
    pub fn has_dirty(&self) -> bool {
        self.all_dirty || !self.dirty_objects.is_empty() || !self.dirty_pages.is_empty()
    }

    /// Get dirty objects.
    pub fn dirty_objects(&self) -> &[ObjectId] {
        &self.dirty_objects
    }

    /// Get dirty pages.
    pub fn dirty_pages(&self) -> &[PageId] {
        &self.dirty_pages
    }

    /// Clear all dirty flags.
    pub fn clear(&mut self) {
        self.dirty_objects.clear();
        self.dirty_pages.clear();
        self.all_dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_geometry::Rect;

    #[test]
    fn test_cache_insert_get() {
        let mut cache = LayoutCache::new(10);
        let object_id = ObjectId::new();

        let layout = TextFrameLayout::new(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Rect::new(0.0, 0.0, 100.0, 100.0),
        );

        cache.insert_text_frame(object_id, 1, layout.clone());

        let retrieved = cache.get_text_frame(object_id, 1);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_cache_invalidate() {
        let mut cache = LayoutCache::new(10);
        let object_id = ObjectId::new();

        let layout = TextFrameLayout::new(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            Rect::new(0.0, 0.0, 100.0, 100.0),
        );

        cache.insert_text_frame(object_id, 1, layout);
        cache.invalidate(object_id);

        let retrieved = cache.get_text_frame(object_id, 1);
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_dirty_tracker() {
        let mut tracker = DirtyTracker::new();
        tracker.clear(); // Start clean

        let object_id = ObjectId::new();
        assert!(!tracker.is_object_dirty(object_id));

        tracker.mark_object_dirty(object_id);
        assert!(tracker.is_object_dirty(object_id));
        assert!(tracker.has_dirty());

        tracker.clear();
        assert!(!tracker.has_dirty());
    }
}
