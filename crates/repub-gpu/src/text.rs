//! Glyph-run rendering by outline tessellation.
//!
//! **SDF atlas is deferred.** The GUI's multi-channel SDF text pipeline
//! (`repub-gui`'s `sdf_text`) depends on `fdsm`/`fdsm-ttf-parser` and a bespoke
//! WGSL atlas pipeline that does not port cleanly into this windowless crate in
//! one pass. Instead glyphs are rendered the correct-but-simpler way: each
//! glyph's outline is read with [`ttf_parser`]'s [`OutlineBuilder`], converted
//! into a [`repub_geometry::Path`], and filled through the same lyon
//! tessellation path as [`Primitive::FillPath`](repub_scene::Primitive::FillPath).
//! This is resolution-independent and exact; only the SDF atlas caching and its
//! sub-pixel antialiasing are absent.
//!
//! Fonts are resolved from the installed-font
//! [`registry`](repub_fonts::registry) by family/weight/italic, falling back to
//! the bundled font when a family is missing or the registry scan has not run.

use std::collections::HashMap;
use std::sync::Arc;

use repub_geometry::{Path, Point, Transform2D};
use repub_scene::{FontRef, Paint, PositionedGlyph};

use crate::error::GpuResult;
use crate::paint::color_fn;
use crate::tessellation::{self, Tessellation};

/// The bundled fallback font, compiled into the binary.
const FALLBACK_FONT: &[u8] = include_bytes!("../../../assets/fonts/fallback.ttf");

/// A loaded font face's raw bytes plus the collection index to parse.
struct LoadedFace {
    data: Arc<Vec<u8>>,
    index: u32,
}

impl LoadedFace {
    fn face(&self) -> Option<ttf_parser::Face<'_>> {
        ttf_parser::Face::parse(&self.data, self.index).ok()
    }
}

/// Resolves [`FontRef`]s to font bytes and tessellates glyph runs.
pub struct TextRenderer {
    cache: HashMap<String, LoadedFace>,
    fallback: Arc<Vec<u8>>,
}

impl TextRenderer {
    /// Create a text renderer holding the bundled fallback font.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            fallback: Arc::new(FALLBACK_FONT.to_vec()),
        }
    }

    fn resolve(&mut self, font_ref: &FontRef) -> &LoadedFace {
        let bold = font_ref.weight >= 600;
        let key = format!(
            "{}:{}:{}",
            font_ref.family.to_lowercase(),
            font_ref.weight,
            font_ref.italic
        );
        if !self.cache.contains_key(&key) {
            let loaded = repub_fonts::registry::try_registry()
                .and_then(|reg| reg.find(&font_ref.family, bold, font_ref.italic))
                .and_then(|file| {
                    let data = std::fs::read(&file.path).ok()?;
                    Some(LoadedFace {
                        data: Arc::new(data),
                        index: file.index,
                    })
                })
                .unwrap_or_else(|| LoadedFace {
                    data: self.fallback.clone(),
                    index: 0,
                });
            self.cache.insert(key.clone(), loaded);
        }
        self.cache.get(&key).unwrap()
    }

    /// Tessellate a glyph run into device-space triangles.
    ///
    /// `transform` is the primitive's page transform and `scale` the scene
    /// device scale. Each glyph's outline (in font units) is scaled by
    /// `size / units_per_em`, positioned at the glyph's baseline, flipped from
    /// the font's y-up to the page's y-down, then tessellated and colored.
    pub fn tessellate_run(
        &mut self,
        font_ref: &FontRef,
        size: f64,
        glyphs: &[PositionedGlyph],
        paint: &Paint,
        transform: Transform2D,
        scale: f64,
    ) -> GpuResult<Tessellation> {
        let loaded = self.resolve(font_ref);
        let Some(face) = loaded.face() else {
            return Ok(Tessellation::default());
        };
        let units_per_em = face.units_per_em() as f64;
        if units_per_em <= 0.0 {
            return Ok(Tessellation::default());
        }
        let font_scale = size / units_per_em;

        let color = color_fn(paint, scale, 1.0);
        let map = tessellation::device_mapper(transform, scale);

        let mut combined = Tessellation::default();
        for g in glyphs {
            if g.ch.is_whitespace() {
                continue;
            }
            let gid = resolve_glyph_id(&face, g);
            let Some(gid) = gid else { continue };

            let mut collector = OutlineCollector::new(font_scale, g.x, g.y);
            if face.outline_glyph(gid, &mut collector).is_none() {
                continue;
            }
            collector.finish();
            if collector.path.is_empty() {
                continue;
            }

            let lyon = tessellation::to_lyon(&collector.path, &map);
            let geo = tessellation::fill(&lyon, repub_scene::FillRule::NonZero, &color)?;
            merge(&mut combined, geo);
        }
        Ok(combined)
    }
}

impl Default for TextRenderer {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_glyph_id(face: &ttf_parser::Face, g: &PositionedGlyph) -> Option<ttf_parser::GlyphId> {
    if g.glyph_id != 0 {
        return Some(ttf_parser::GlyphId(g.glyph_id));
    }
    face.glyph_index(g.ch)
}

fn merge(into: &mut Tessellation, other: Tessellation) {
    let base = into.vertices.len() as u32;
    into.vertices.extend(other.vertices);
    into.indices.extend(other.indices.into_iter().map(|i| i + base));
}

/// Builds a [`Path`] from a glyph outline, scaling font units to points and
/// flipping the font's y-up axis to the page's y-down, offset to the glyph's
/// baseline origin `(ox, oy)`.
struct OutlineCollector {
    path: Path,
    scale: f64,
    ox: f64,
    oy: f64,
    open: bool,
}

impl OutlineCollector {
    fn new(scale: f64, ox: f64, oy: f64) -> Self {
        Self {
            path: Path::new(),
            scale,
            ox,
            oy,
            open: false,
        }
    }

    fn pt(&self, x: f32, y: f32) -> Point {
        Point::new(
            self.ox + x as f64 * self.scale,
            self.oy - y as f64 * self.scale,
        )
    }

    fn finish(&mut self) {
        if self.open {
            self.path.close();
            self.open = false;
        }
    }
}

impl ttf_parser::OutlineBuilder for OutlineCollector {
    fn move_to(&mut self, x: f32, y: f32) {
        if self.open {
            self.path.close();
        }
        let p = self.pt(x, y);
        self.path.move_to(p);
        self.open = true;
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.pt(x, y);
        self.path.line_to(p);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let c = self.pt(x1, y1);
        let e = self.pt(x, y);
        self.path.quad_to(c, e);
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let c1 = self.pt(x1, y1);
        let c2 = self.pt(x2, y2);
        let e = self.pt(x, y);
        self.path.cubic_to(c1, c2, e);
    }

    fn close(&mut self) {
        self.path.close();
        self.open = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_color::Color;

    #[test]
    fn fallback_loads_and_outlines() {
        let mut tr = TextRenderer::new();
        let glyphs = vec![PositionedGlyph {
            ch: 'R',
            glyph_id: 0,
            x: 0.0,
            y: 40.0,
            advance: 20.0,
        }];
        let geo = tr
            .tessellate_run(
                &FontRef::new("Selawik", 400, false),
                48.0,
                &glyphs,
                &Paint::Solid(Color::BLACK),
                Transform2D::IDENTITY,
                1.0,
            )
            .unwrap();
        assert!(!geo.is_empty(), "glyph 'R' should produce triangles");
    }

    #[test]
    fn whitespace_produces_nothing() {
        let mut tr = TextRenderer::new();
        let glyphs = vec![PositionedGlyph {
            ch: ' ',
            glyph_id: 0,
            x: 0.0,
            y: 40.0,
            advance: 10.0,
        }];
        let geo = tr
            .tessellate_run(
                &FontRef::new("Selawik", 400, false),
                48.0,
                &glyphs,
                &Paint::Solid(Color::BLACK),
                Transform2D::IDENTITY,
                1.0,
            )
            .unwrap();
        assert!(geo.is_empty());
    }
}
