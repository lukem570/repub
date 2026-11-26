//! A cache of rasterized glyph bitmaps.
//!
//! Ported from the original `repub-render-cpu` glyph cache: an alpha-coverage
//! bitmap keyed by character, size, font identity and style, with a simple
//! capacity-bounded eviction.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

/// Key identifying a rasterized glyph.
#[derive(Debug, Clone)]
pub struct GlyphKey {
    /// Source character.
    pub ch: char,
    /// Font size in tenths of a point (rounded to avoid float keys).
    pub size_tenths: u32,
    /// Hash of the resolved font identity.
    pub font_hash: u64,
}

impl GlyphKey {
    /// Create a glyph key for a character at a pixel size and font hash.
    pub fn new(ch: char, size: f32, font_hash: u64) -> Self {
        Self {
            ch,
            size_tenths: (size * 10.0).round() as u32,
            font_hash,
        }
    }
}

impl Hash for GlyphKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ch.hash(state);
        self.size_tenths.hash(state);
        self.font_hash.hash(state);
    }
}

impl PartialEq for GlyphKey {
    fn eq(&self, other: &Self) -> bool {
        self.ch == other.ch
            && self.size_tenths == other.size_tenths
            && self.font_hash == other.font_hash
    }
}

impl Eq for GlyphKey {}

/// A rasterized glyph: an 8-bit alpha coverage bitmap plus placement metrics.
#[derive(Debug, Clone)]
pub struct CachedGlyph {
    /// Coverage bitmap (one byte of alpha per pixel).
    pub bitmap: Vec<u8>,
    /// Bitmap width in pixels.
    pub width: usize,
    /// Bitmap height in pixels.
    pub height: usize,
    /// X offset of the bitmap from the pen origin, in pixels.
    pub xmin: i32,
    /// Y offset of the bitmap top from the baseline, in pixels.
    pub ymin: i32,
}

/// A capacity-bounded cache of rasterized glyphs.
pub struct GlyphCache {
    cache: HashMap<GlyphKey, CachedGlyph>,
    max_size: usize,
}

impl GlyphCache {
    /// Create a cache holding at most `max_size` glyphs.
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: HashMap::with_capacity(max_size.min(1024)),
            max_size: max_size.max(1),
        }
    }

    /// Fetch a cached glyph, if present.
    pub fn get(&self, key: &GlyphKey) -> Option<&CachedGlyph> {
        self.cache.get(key)
    }

    /// Insert a glyph, evicting entries if at capacity.
    pub fn insert(&mut self, key: GlyphKey, glyph: CachedGlyph) {
        if self.cache.len() >= self.max_size {
            self.evict();
        }
        self.cache.insert(key, glyph);
    }

    fn evict(&mut self) {
        let to_remove = (self.cache.len() / 4).max(1);
        let keys: Vec<GlyphKey> = self.cache.keys().take(to_remove).cloned().collect();
        for key in keys {
            self.cache.remove(&key);
        }
    }

    /// Number of cached glyphs.
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let mut cache = GlyphCache::new(16);
        let key = GlyphKey::new('A', 12.0, 7);
        cache.insert(
            key.clone(),
            CachedGlyph {
                bitmap: vec![255; 4],
                width: 2,
                height: 2,
                xmin: 0,
                ymin: 0,
            },
        );
        assert!(cache.get(&key).is_some());
    }

    #[test]
    fn eviction_bounds_size() {
        let mut cache = GlyphCache::new(4);
        for i in 0..20u32 {
            let ch = char::from_u32(65 + i).unwrap();
            cache.insert(
                GlyphKey::new(ch, 12.0, 1),
                CachedGlyph {
                    bitmap: vec![],
                    width: 0,
                    height: 0,
                    xmin: 0,
                    ymin: 0,
                },
            );
        }
        assert!(cache.len() <= 4);
    }
}
