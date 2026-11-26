//! Glyph outline extraction via ttf-parser.
//!
//! WordArt resolves text to concrete filled/stroked paths, so it needs each
//! glyph's outline as Bézier curves. This module wraps `ttf-parser`'s
//! [`OutlineBuilder`](ttf_parser::OutlineBuilder) and produces outlines in
//! font em units (y up, baseline at 0), which the WordArt resolver then warps
//! and flips into page coordinates.

use repub_geometry::{Point, Rect};

/// A single glyph outline in em units (y up, origin at the glyph's left
/// baseline).
#[derive(Debug, Clone)]
pub struct GlyphOutline {
    /// The source character.
    pub ch: char,
    /// Outline segments in em units.
    pub segments: Vec<OutlineSegment>,
    /// Advance width in em units.
    pub advance: f64,
    /// Tight bounding box of the outline in em units, if non-empty.
    pub bounds: Option<Rect>,
}

/// A single segment of a [`GlyphOutline`], in em units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OutlineSegment {
    /// Start a new contour at this point.
    MoveTo(Point),
    /// Straight line to this point.
    LineTo(Point),
    /// Quadratic Bézier (control, end).
    QuadTo(Point, Point),
    /// Cubic Bézier (control1, control2, end).
    CubicTo(Point, Point, Point),
    /// Close the current contour.
    Close,
}

struct Builder {
    scale: f64,
    segments: Vec<OutlineSegment>,
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
}

impl Builder {
    fn new(scale: f64) -> Self {
        Self {
            scale,
            segments: Vec::new(),
            min_x: f64::MAX,
            min_y: f64::MAX,
            max_x: f64::MIN,
            max_y: f64::MIN,
        }
    }

    fn track(&mut self, x: f64, y: f64) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    fn bounds(&self) -> Option<Rect> {
        if self.min_x > self.max_x {
            None
        } else {
            Some(Rect::new(
                self.min_x,
                self.min_y,
                self.max_x - self.min_x,
                self.max_y - self.min_y,
            ))
        }
    }
}

impl ttf_parser::OutlineBuilder for Builder {
    fn move_to(&mut self, x: f32, y: f32) {
        let (x, y) = (x as f64 * self.scale, y as f64 * self.scale);
        self.track(x, y);
        self.segments.push(OutlineSegment::MoveTo(Point::new(x, y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let (x, y) = (x as f64 * self.scale, y as f64 * self.scale);
        self.track(x, y);
        self.segments.push(OutlineSegment::LineTo(Point::new(x, y)));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (cx, cy) = (x1 as f64 * self.scale, y1 as f64 * self.scale);
        let (ex, ey) = (x as f64 * self.scale, y as f64 * self.scale);
        self.track(cx, cy);
        self.track(ex, ey);
        self.segments
            .push(OutlineSegment::QuadTo(Point::new(cx, cy), Point::new(ex, ey)));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (c1x, c1y) = (x1 as f64 * self.scale, y1 as f64 * self.scale);
        let (c2x, c2y) = (x2 as f64 * self.scale, y2 as f64 * self.scale);
        let (ex, ey) = (x as f64 * self.scale, y as f64 * self.scale);
        self.track(c1x, c1y);
        self.track(c2x, c2y);
        self.track(ex, ey);
        self.segments.push(OutlineSegment::CubicTo(
            Point::new(c1x, c1y),
            Point::new(c2x, c2y),
            Point::new(ex, ey),
        ));
    }

    fn close(&mut self) {
        self.segments.push(OutlineSegment::Close);
    }
}

/// Vertical font metrics in em units.
#[derive(Debug, Clone, Copy)]
pub struct FontEmMetrics {
    /// Ascender in em units (positive).
    pub ascent: f64,
    /// Descender in em units (negative).
    pub descent: f64,
}

/// A parsed font face, with helpers to extract glyph outlines.
pub struct OutlineFace<'a> {
    face: ttf_parser::Face<'a>,
    units_per_em: f64,
}

impl<'a> OutlineFace<'a> {
    /// Parse the first face from font file bytes.
    pub fn parse(data: &'a [u8]) -> Option<Self> {
        let face = ttf_parser::Face::parse(data, 0).ok()?;
        let units_per_em = face.units_per_em() as f64;
        if units_per_em <= 0.0 {
            return None;
        }
        Some(Self { face, units_per_em })
    }

    /// The face's vertical metrics in em units.
    pub fn metrics(&self) -> FontEmMetrics {
        FontEmMetrics {
            ascent: self.face.ascender() as f64 / self.units_per_em,
            descent: self.face.descender() as f64 / self.units_per_em,
        }
    }

    /// Extract the outline for `ch` in em units, or `None` if the font has no
    /// glyph for the character.
    pub fn outline(&self, ch: char) -> Option<GlyphOutline> {
        let glyph_id = self.face.glyph_index(ch)?;
        let advance =
            self.face.glyph_hor_advance(glyph_id).unwrap_or(0) as f64 / self.units_per_em;
        let mut builder = Builder::new(1.0 / self.units_per_em);
        // A space (or any whitespace with no contour) has no outline but still
        // advances; return an empty outline rather than None so it reserves width.
        let bounds = match self.face.outline_glyph(glyph_id, &mut builder) {
            Some(_) => builder.bounds(),
            None => None,
        };
        Some(GlyphOutline {
            ch,
            segments: builder.segments,
            advance,
            bounds,
        })
    }
}
