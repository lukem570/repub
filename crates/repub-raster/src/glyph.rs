//! Font resolution and glyph-run rasterization.
//!
//! Fonts are resolved from a [`FontRef`] (family, weight, italic) by looking up
//! the installed-font [`registry`](repub_fonts::registry). When the family is
//! not installed, or the registry scan has not completed, a bundled fallback
//! font is used. Each glyph is rasterized with [`fontdue`] into an 8-bit alpha
//! coverage bitmap, cached, and blitted onto the target pixmap tinted by the
//! run's paint primary color.

use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use fontdue::{Font, FontSettings};
use repub_scene::{FontRef, Paint, PositionedGlyph};
use tiny_skia::{Pixmap, Transform};

use crate::glyph_cache::{CachedGlyph, GlyphCache, GlyphKey};
use crate::paint::convert_color;

/// The bundled fallback font, compiled into the binary.
const FALLBACK_FONT: &[u8] = include_bytes!("../../../assets/fonts/fallback.ttf");

/// Resolves and rasterizes glyph runs, caching loaded fonts and glyph bitmaps.
pub struct GlyphRenderer {
    glyphs: GlyphCache,
    fonts: HashMap<String, (Arc<Font>, u64)>,
    fallback: Arc<Font>,
}

impl GlyphRenderer {
    /// Create a glyph renderer with a loaded bundled fallback font.
    pub fn new() -> Self {
        let fallback = Font::from_bytes(FALLBACK_FONT, FontSettings::default())
            .expect("bundled fallback font should parse");
        Self {
            glyphs: GlyphCache::new(2048),
            fonts: HashMap::new(),
            fallback: Arc::new(fallback),
        }
    }

    /// Resolve a [`FontRef`] to a loaded font and a stable identity hash.
    ///
    /// Looks the family up in the installed-font registry; falls back to the
    /// bundled font when the family is missing or the registry is unavailable.
    fn resolve(&mut self, font_ref: &FontRef) -> (Arc<Font>, u64) {
        let bold = font_ref.weight >= 600;
        let key = format!(
            "{}:{}:{}",
            font_ref.family.to_lowercase(),
            font_ref.weight,
            font_ref.italic
        );
        if let Some(entry) = self.fonts.get(&key) {
            return entry.clone();
        }

        let loaded = repub_fonts::registry::try_registry()
            .and_then(|reg| reg.find(&font_ref.family, bold, font_ref.italic))
            .and_then(|file| {
                let data = std::fs::read(&file.path).ok()?;
                Font::from_bytes(
                    data,
                    FontSettings {
                        collection_index: file.index,
                        ..FontSettings::default()
                    },
                )
                .ok()
            });

        let font = loaded
            .map(Arc::new)
            .unwrap_or_else(|| self.fallback.clone());
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        let entry = (font, hasher.finish());
        self.fonts.insert(key, entry.clone());
        entry
    }

    /// Rasterize and blit a glyph run onto `pixmap`.
    ///
    /// `transform` is the full device transform for the run (scene scale
    /// composed with the primitive transform). Glyphs are rasterized at the
    /// transform's scale so they stay crisp, then positioned by their baseline
    /// coordinates transformed into device space.
    pub fn draw_run(
        &mut self,
        pixmap: &mut Pixmap,
        font_ref: &FontRef,
        size: f64,
        glyphs: &[PositionedGlyph],
        paint: &Paint,
        transform: &Transform,
    ) {
        let (font, font_hash) = self.resolve(font_ref);
        let scale = crate::path::transform_scale(transform);
        let render_px = (size as f32 * scale).max(1.0);
        let (tint_r, tint_g, tint_b, tint_a) = {
            let c = convert_color(&paint.primary_color());
            (c.red(), c.green(), c.blue(), c.alpha())
        };

        for g in glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            let key = GlyphKey::new(g.ch, render_px, font_hash);
            if self.glyphs.get(&key).is_none() {
                let (metrics, bitmap) = font.rasterize(g.ch, render_px);
                let cached = CachedGlyph {
                    bitmap,
                    width: metrics.width,
                    height: metrics.height,
                    xmin: metrics.xmin,
                    ymin: metrics.ymin,
                };
                self.glyphs.insert(key.clone(), cached);
            }
            let glyph = self.glyphs.get(&key).unwrap();
            if glyph.width == 0 || glyph.height == 0 {
                continue;
            }

            // Glyph top-left in the run's coordinate space (points).
            let inv = 1.0 / scale;
            let origin_x = g.x as f32 + glyph.xmin as f32 * inv;
            let origin_y = g.y as f32 - glyph.ymin as f32 * inv - glyph.height as f32 * inv;

            blit_coverage(
                pixmap,
                glyph,
                origin_x,
                origin_y,
                inv,
                transform,
                (tint_r, tint_g, tint_b, tint_a),
            );
        }
    }
}

impl Default for GlyphRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Blit one glyph's coverage bitmap into `pixmap`.
///
/// The bitmap is placed into a per-glyph pixmap at the rasterized pixel size
/// and drawn with a transform that maps the run coordinate origin into device
/// space while cancelling the glyph's own supersampling scale.
fn blit_coverage(
    pixmap: &mut Pixmap,
    glyph: &CachedGlyph,
    origin_x: f32,
    origin_y: f32,
    inv_scale: f32,
    transform: &Transform,
    tint: (f32, f32, f32, f32),
) {
    let Some(mut glyph_pixmap) = Pixmap::new(glyph.width as u32, glyph.height as u32) else {
        return;
    };
    let (tr, tg, tb, ta) = tint;
    let data = glyph_pixmap.data_mut();
    for (i, &coverage) in glyph.bitmap.iter().enumerate() {
        let a = (coverage as f32 / 255.0) * ta;
        let idx = i * 4;
        // Premultiplied alpha, as tiny-skia expects.
        data[idx] = (tr * a * 255.0) as u8;
        data[idx + 1] = (tg * a * 255.0) as u8;
        data[idx + 2] = (tb * a * 255.0) as u8;
        data[idx + 3] = (a * 255.0) as u8;
    }

    // The glyph bitmap is rasterized at `1/inv_scale` device pixels per point,
    // so scale it back down by `inv_scale` into run coordinates, then place it
    // at the glyph origin and apply the run's device transform.
    let placement = transform
        .pre_translate(origin_x, origin_y)
        .pre_scale(inv_scale, inv_scale);

    pixmap.draw_pixmap(
        0,
        0,
        glyph_pixmap.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        placement,
        None,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback_font_loads() {
        let renderer = GlyphRenderer::new();
        assert!(renderer.fallback.glyph_count() > 0);
    }

    #[test]
    fn unknown_family_resolves_to_fallback() {
        let mut renderer = GlyphRenderer::new();
        let (_font, hash) = renderer.resolve(&FontRef::new("NoSuchFamily12345", 400, false));
        // Resolving again returns the cached entry with the same hash.
        let (_font2, hash2) = renderer.resolve(&FontRef::new("NoSuchFamily12345", 400, false));
        assert_eq!(hash, hash2);
    }
}
