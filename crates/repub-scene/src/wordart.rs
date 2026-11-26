//! WordArt and text-on-path resolution into scene primitives.
//!
//! This ports the warp-envelope math from the GUI's `wordart_warp.rs` and the
//! glyph-outline transform from `wordart_path.rs`, but produces scene
//! [`Primitive`](crate::Primitive)s (filled and stroked
//! [`Path`](repub_geometry::Path)s) instead of egui meshes.
//!
//! A text effect is resolved in three passes:
//! 1. Each glyph outline is extracted (via [`crate::glyph`]) in em units.
//! 2. The outline is mapped into the shape's bounds, either through a warp
//!    envelope ([`TextTransform::Preset`]) or along a path
//!    ([`TextTransform::Path`]).
//! 3. Optional 3D extrusion emits offset copies of the front face behind it,
//!    lightly shaded; then the front fill and outline are emitted.
//!
//! See the crate docs for a note on what is simplified relative to the GUI.

use repub_color::Color;
use repub_geometry::{Path as GeomPath, Point, Rect, Transform2D};
use repub_model::geometry::Path as ModelPath;
use repub_model::shape::{Extrusion3D, TextEffectData, TextTransform, WordArtPreset};
use repub_model::styles::{Fill, Stroke};

use crate::convert::{fill_to_paint, model_path_to_geom, stroke_paint};
use crate::glyph::{GlyphOutline, OutlineFace, OutlineSegment};
use crate::primitive::{FillRule, LineCap, LineJoin, Primitive, SceneEffect};

/// The warp preset envelopes, ported from the GUI's `WarpPreset`.
///
/// Each preset defines a top and bottom envelope as a function of the
/// horizontal position `x` (0–1) and the box `height`. Text is mapped between
/// the two envelopes per the ECMA-376 warp algorithm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WarpPreset {
    /// No warp (plain, linearly scaled text).
    #[default]
    Plain,
    /// Single sine wave.
    Wave1,
    /// Single sine wave, opposite phase.
    Wave2,
    /// Double wave.
    Wave4,
    /// Double wave (alias of [`WarpPreset::Wave4`]).
    DoubleWave1,
    /// Arch curving upward.
    ArchUp,
    /// Arch curving downward.
    ArchDown,
    /// Arch up, pour variant.
    ArchUpPour,
    /// Arch down, pour variant.
    ArchDownPour,
    /// Both edges curve up (circle).
    Circle,
    /// Circle, pour variant.
    CirclePour,
    /// Button shape.
    Button,
    /// Button, pour variant.
    ButtonPour,
    /// Top forms a triangle peak.
    Triangle,
    /// Bottom forms a triangle.
    TriangleInverted,
    /// Both edges form a V pointing up.
    Chevron,
    /// Both edges form a V pointing down.
    ChevronInverted,
    /// Shrinks toward the right.
    FadeRight,
    /// Shrinks toward the left.
    FadeLeft,
    /// Fades up (approximated as plain).
    FadeUp,
    /// Fades down (approximated as plain).
    FadeDown,
    /// Slants upward.
    SlantUp,
    /// Slants downward.
    SlantDown,
    /// Cascades up (smaller to the right, tilting up).
    CascadeUp,
    /// Cascades down (smaller to the right, tilting down).
    CascadeDown,
    /// Curve up.
    CurveUp,
    /// Curve down.
    CurveDown,
    /// Cylindrical can curving up.
    CanUp,
    /// Cylindrical can curving down.
    CanDown,
    /// Bulges outward in the middle.
    Inflate,
    /// Pinches inward in the middle.
    Deflate,
    /// Inflates the bottom edge.
    InflateBottom,
    /// Inflates the top edge.
    InflateTop,
    /// Deflates the bottom edge.
    DeflateBottom,
    /// Deflates the top edge.
    DeflateTop,
    /// Mixed deflate/inflate wave.
    DeflateInflate,
    /// Mixed deflate/inflate/deflate wave.
    DeflateInflateDeflate,
    /// Ring (inside).
    RingInside,
    /// Ring (outside).
    RingOutside,
    /// Octagonal stop-sign shape.
    Stop,
}

impl WarpPreset {
    /// Map a model [`WordArtPreset`] to the closest warp envelope.
    pub fn from_model(preset: WordArtPreset) -> Self {
        match preset {
            WordArtPreset::Plain | WordArtPreset::Outline | WordArtPreset::Shadow => {
                WarpPreset::Plain
            }
            WordArtPreset::ThreeD => WarpPreset::Plain,
            WordArtPreset::ArchUp => WarpPreset::ArchUp,
            WordArtPreset::ArchDown => WarpPreset::ArchDown,
            WordArtPreset::Circle => WarpPreset::Circle,
            WordArtPreset::Wave => WarpPreset::Wave1,
            WordArtPreset::Inflate => WarpPreset::Inflate,
            WordArtPreset::Deflate => WarpPreset::Deflate,
            WordArtPreset::SlantUp => WarpPreset::SlantUp,
            WordArtPreset::SlantDown => WarpPreset::SlantDown,
            WordArtPreset::FadeRight => WarpPreset::FadeRight,
            WordArtPreset::FadeLeft => WarpPreset::FadeLeft,
            WordArtPreset::ChevronUp => WarpPreset::Chevron,
            WordArtPreset::ChevronDown => WarpPreset::ChevronInverted,
            WordArtPreset::TriangleUp => WarpPreset::Triangle,
            WordArtPreset::TriangleDown => WarpPreset::TriangleInverted,
            WordArtPreset::CanUp => WarpPreset::CanUp,
            WordArtPreset::CanDown => WarpPreset::CanDown,
            WordArtPreset::CascadeUp => WarpPreset::CascadeUp,
            WordArtPreset::CascadeDown => WarpPreset::CascadeDown,
            WordArtPreset::Stop => WarpPreset::Stop,
            WordArtPreset::RingInside => WarpPreset::RingInside,
            WordArtPreset::RingOutside => WarpPreset::RingOutside,
            WordArtPreset::Button => WarpPreset::Button,
        }
    }

    /// The top and bottom envelope y positions at `x` (0–1) for a box of the
    /// given `height`. Ported from the GUI's `get_warp_envelope_internal`.
    pub fn envelope(self, x: f64, h: f64) -> (f64, f64) {
        use std::f64::consts::PI;
        match self {
            WarpPreset::Plain => (0.0, h),
            WarpPreset::Wave1 => {
                let amp = h * 0.2;
                let wave = (x * PI * 2.0).sin() * amp;
                (amp + wave, h - amp + wave)
            }
            WarpPreset::Wave2 => {
                let amp = h * 0.2;
                let wave = (x * PI * 2.0 + PI).sin() * amp;
                (amp + wave, h - amp + wave)
            }
            WarpPreset::Wave4 | WarpPreset::DoubleWave1 => {
                let amp = h * 0.12;
                let wave = (x * PI * 4.0).sin() * amp;
                (amp + wave, h - amp + wave)
            }
            WarpPreset::ArchUp | WarpPreset::ArchUpPour => {
                let curve = h * 0.4;
                let parabola = 4.0 * curve * x * (1.0 - x);
                (-parabola * 0.3, h - parabola)
            }
            WarpPreset::ArchDown | WarpPreset::ArchDownPour => {
                let curve = h * 0.4;
                let parabola = 4.0 * curve * x * (1.0 - x);
                (parabola, h + parabola * 0.3)
            }
            WarpPreset::Circle
            | WarpPreset::CirclePour
            | WarpPreset::RingInside
            | WarpPreset::RingOutside => {
                let curve = h * 0.3;
                let arc = 4.0 * curve * x * (1.0 - x);
                (-arc * 0.5, h - arc)
            }
            WarpPreset::Button | WarpPreset::ButtonPour => {
                let curve = h * 0.25;
                let arc = 4.0 * curve * x * (1.0 - x);
                (-arc * 0.3, h - arc * 0.7)
            }
            WarpPreset::Inflate => {
                let bulge = h * 0.3;
                let arc = 4.0 * bulge * x * (1.0 - x);
                (-arc, h + arc)
            }
            WarpPreset::Deflate => {
                let pinch = h * 0.25;
                let arc = 4.0 * pinch * x * (1.0 - x);
                (arc, h - arc)
            }
            WarpPreset::InflateBottom => {
                let bulge = h * 0.35;
                let arc = 4.0 * bulge * x * (1.0 - x);
                (0.0, h + arc)
            }
            WarpPreset::InflateTop => {
                let bulge = h * 0.35;
                let arc = 4.0 * bulge * x * (1.0 - x);
                (-arc, h)
            }
            WarpPreset::DeflateBottom => {
                let pinch = h * 0.3;
                let arc = 4.0 * pinch * x * (1.0 - x);
                (0.0, h - arc)
            }
            WarpPreset::DeflateTop => {
                let pinch = h * 0.3;
                let arc = 4.0 * pinch * x * (1.0 - x);
                (arc, h)
            }
            WarpPreset::Triangle => {
                let peak = h * 0.45;
                let top = if x < 0.5 {
                    peak * (1.0 - x * 2.0)
                } else {
                    peak * ((x - 0.5) * 2.0)
                };
                (top, h)
            }
            WarpPreset::TriangleInverted => {
                let peak = h * 0.45;
                let bottom = if x < 0.5 {
                    h - peak + peak * x * 2.0
                } else {
                    h - peak * (1.0 - (x - 0.5) * 2.0)
                };
                (0.0, bottom)
            }
            WarpPreset::Chevron => {
                let depth = h * 0.3;
                let v = if x < 0.5 {
                    depth * (1.0 - x * 2.0)
                } else {
                    depth * ((x - 0.5) * 2.0)
                };
                (v, h - depth + v)
            }
            WarpPreset::ChevronInverted => {
                let depth = h * 0.3;
                let v = if x < 0.5 {
                    depth * x * 2.0
                } else {
                    depth * (1.0 - (x - 0.5) * 2.0)
                };
                (v, h - depth + v)
            }
            WarpPreset::FadeRight => {
                let shrink = h * 0.45;
                (shrink * x, h - shrink * x)
            }
            WarpPreset::FadeLeft => {
                let shrink = h * 0.45;
                (shrink * (1.0 - x), h - shrink * (1.0 - x))
            }
            WarpPreset::FadeUp | WarpPreset::FadeDown => (0.0, h),
            WarpPreset::SlantUp => {
                let slant = h * 0.35;
                (slant * (1.0 - x), h - slant * x)
            }
            WarpPreset::SlantDown => {
                let slant = h * 0.35;
                (slant * x, h - slant * (1.0 - x))
            }
            WarpPreset::CascadeUp => {
                let shrink = h * 0.4;
                (shrink * (1.0 - x), h - shrink * x)
            }
            WarpPreset::CascadeDown => {
                let shrink = h * 0.4;
                (shrink * x, h - shrink * (1.0 - x))
            }
            WarpPreset::CurveUp => {
                let curve = h * 0.25;
                let arc = 4.0 * curve * x * (1.0 - x);
                (-arc * 0.4, h - arc)
            }
            WarpPreset::CurveDown => {
                let curve = h * 0.25;
                let arc = 4.0 * curve * x * (1.0 - x);
                (arc, h + arc * 0.4)
            }
            WarpPreset::CanUp => {
                let dy = h * 0.143;
                let arc = 4.0 * dy * x * (1.0 - x);
                (-arc, h - arc)
            }
            WarpPreset::CanDown => {
                let dy = h * 0.143;
                let arc = 4.0 * dy * x * (1.0 - x);
                (arc, h + arc)
            }
            WarpPreset::Stop => {
                let inset = h * 0.2;
                let third = 1.0 / 3.0;
                if x < third {
                    let t = x / third;
                    (inset * (1.0 - t), h - inset * (1.0 - t))
                } else if x > 1.0 - third {
                    let t = (x - (1.0 - third)) / third;
                    (inset * t, h - inset * t)
                } else {
                    (0.0, h)
                }
            }
            WarpPreset::DeflateInflate | WarpPreset::DeflateInflateDeflate => {
                let amp = h * 0.15;
                let wave = (x * PI * 3.0).sin() * amp;
                (amp + wave, h - amp + wave)
            }
        }
    }
}

/// The em-space extent of the whole text run (used to normalize each point).
#[derive(Debug, Clone, Copy)]
struct TextExtent {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
}

/// Map a point from text coordinates (em units, y up, baseline at 0) through
/// the warp envelope into the box `bounds` (page coordinates, y down).
///
/// Ported from the GUI's `warp_point`. The point's normalized `(x%, y%)` within
/// `extent` drives the envelope lookup.
fn warp_point(p: Point, extent: TextExtent, bounds: Rect, preset: WarpPreset) -> Point {
    let text_width = extent.max_x - extent.min_x;
    let text_height = extent.max_y - extent.min_y;

    let x_pct = if text_width > 0.0 {
        (p.x - extent.min_x) / text_width
    } else {
        0.5
    };
    let y_pct_font = if text_height > 0.0 {
        (p.y - extent.min_y) / text_height
    } else {
        0.5
    };
    let y_pct = 1.0 - y_pct_font;

    let (env_top, env_bottom) = preset.envelope(x_pct.clamp(0.0, 1.0), bounds.height());
    let warped_y = bounds.y() + env_top + y_pct.clamp(0.0, 1.0) * (env_bottom - env_top);
    let warped_x = bounds.x() + x_pct * bounds.width();
    Point::new(warped_x, warped_y)
}

/// Linearly map a point from text coordinates into `bounds`, flipping y
/// (used for [`WarpPreset::Plain`]). Ported from the GUI's plain branch.
fn plain_point(p: Point, extent: TextExtent, bounds: Rect) -> Point {
    let text_width = extent.max_x - extent.min_x;
    let text_height = extent.max_y - extent.min_y;
    let scale_x = if text_width > 0.0 {
        bounds.width() / text_width
    } else {
        1.0
    };
    let scale_y = if text_height > 0.0 {
        bounds.height() / text_height
    } else {
        1.0
    };
    Point::new(
        bounds.x() + (p.x - extent.min_x) * scale_x,
        bounds.y() + (extent.max_y - p.y) * scale_y,
    )
}

/// Warp (or linearly map) a glyph outline into a geometry path in page space.
fn warp_outline_to_path(
    outline: &GlyphOutline,
    glyph_offset_x: f64,
    extent: TextExtent,
    bounds: Rect,
    preset: WarpPreset,
) -> GeomPath {
    let tf = |p: Point| -> Point {
        let tp = Point::new(glyph_offset_x + p.x, p.y);
        if preset == WarpPreset::Plain {
            plain_point(tp, extent, bounds)
        } else {
            warp_point(tp, extent, bounds, preset)
        }
    };

    let mut path = GeomPath::new();
    for seg in &outline.segments {
        match *seg {
            OutlineSegment::MoveTo(p) => {
                path.move_to(tf(p));
            }
            OutlineSegment::LineTo(p) => {
                path.line_to(tf(p));
            }
            OutlineSegment::QuadTo(c, e) => {
                path.quad_to(tf(c), tf(e));
            }
            OutlineSegment::CubicTo(c1, c2, e) => {
                path.cubic_to(tf(c1), tf(c2), tf(e));
            }
            OutlineSegment::Close => {
                path.close();
            }
        }
    }
    path
}

/// Translate every point of a geometry path by `(dx, dy)`.
fn offset_path(path: &GeomPath, dx: f64, dy: f64) -> GeomPath {
    use repub_geometry::PathSegment;
    let mut out = GeomPath::new();
    let shift = |p: Point| Point::new(p.x + dx, p.y + dy);
    for seg in &path.segments {
        out.segments.push(match *seg {
            PathSegment::MoveTo(p) => PathSegment::MoveTo(shift(p)),
            PathSegment::LineTo(p) => PathSegment::LineTo(shift(p)),
            PathSegment::QuadTo(c, e) => PathSegment::QuadTo(shift(c), shift(e)),
            PathSegment::CubicTo(c1, c2, e) => PathSegment::CubicTo(shift(c1), shift(c2), shift(e)),
            PathSegment::Close => PathSegment::Close,
        });
    }
    out
}

/// Darken a color toward black by `factor` (0 = black, 1 = unchanged).
fn darken(c: Color, factor: f64) -> Color {
    Color::new(c.r * factor, c.g * factor, c.b * factor, c.a)
}

/// Parameters controlling WordArt resolution.
struct ResolveParams<'a> {
    text: &'a str,
    bounds: Rect,
    transform: Transform2D,
    clip: Option<Rect>,
    fill: &'a Fill,
    line: Option<&'a Stroke>,
    effects: Vec<SceneEffect>,
    extrusion: Option<&'a Extrusion3D>,
    opacity: f64,
}

/// Resolve a text-effect shape into scene primitives, appending them to `out`.
///
/// `font_bytes` supplies the font face for glyph outlines; the caller resolves
/// it from a [`FontProvider`](repub_fonts::FontProvider) registry or a bundled
/// fallback. Returns the number of primitives appended.
#[allow(clippy::too_many_arguments)]
pub fn resolve_text_effect(
    data: &TextEffectData,
    bounds: Rect,
    transform: Transform2D,
    clip: Option<Rect>,
    fill: &Fill,
    line: Option<&Stroke>,
    effects: Vec<SceneEffect>,
    opacity: f64,
    scheme: &repub_color::ColorScheme,
    font_bytes: &[u8],
    out: &mut Vec<Primitive>,
) -> usize {
    let params = ResolveParams {
        text: &data.text,
        bounds,
        transform,
        clip,
        fill,
        line,
        effects,
        extrusion: data.extrusion.as_ref().filter(|e| e.depth > 0.0),
        opacity,
    };
    match &data.transform {
        TextTransform::Preset(p) => {
            resolve_warped(&params, WarpPreset::from_model(*p), scheme, font_bytes, out)
        }
        TextTransform::None => {
            resolve_warped(&params, WarpPreset::Plain, scheme, font_bytes, out)
        }
        TextTransform::Warp { .. } => {
            // Free-form control-point warp is not ported; fall back to plain.
            resolve_warped(&params, WarpPreset::Plain, scheme, font_bytes, out)
        }
        TextTransform::Path(path) => resolve_on_path(&params, path, scheme, font_bytes, out),
    }
}

fn resolve_warped(
    params: &ResolveParams<'_>,
    preset: WarpPreset,
    scheme: &repub_color::ColorScheme,
    font_bytes: &[u8],
    out: &mut Vec<Primitive>,
) -> usize {
    let Some(face) = OutlineFace::parse(font_bytes) else {
        return 0;
    };
    let metrics = face.metrics();

    let mut outlines: Vec<GlyphOutline> = Vec::new();
    let mut total_advance = 0.0;
    for ch in params.text.chars() {
        if let Some(o) = face.outline(ch) {
            total_advance += o.advance;
            outlines.push(o);
        }
    }
    if outlines.is_empty() || total_advance <= 0.0 {
        return 0;
    }

    let extent = TextExtent {
        min_x: 0.0,
        max_x: total_advance,
        min_y: metrics.descent,
        max_y: metrics.ascent,
    };

    // Pad the box a little, as the GUI does.
    let pad = params.bounds.width() * 0.03;
    let box_rect = Rect::new(
        params.bounds.x() + pad,
        params.bounds.y() + pad,
        (params.bounds.width() - pad * 2.0).max(1.0),
        (params.bounds.height() - pad * 2.0).max(1.0),
    );

    let fill_paint = fill_to_paint(params.fill, box_rect, scheme)
        .map(|p| crate::convert::paint_with_opacity(p, params.opacity));

    let start = out.len();
    let mut glyph_x = 0.0;
    for outline in &outlines {
        let path = warp_outline_to_path(outline, glyph_x, extent, box_rect, preset);
        glyph_x += outline.advance;
        if path.is_empty() {
            continue;
        }

        // 3D extrusion: emit offset copies of the front face behind it,
        // shaded darker with depth, before the front fill.
        if let Some(ext) = params.extrusion {
            let ext_color = ext
                .color
                .unwrap_or_else(|| darken(params.fill.primary_color(), 0.5));
            let depth = ext.depth;
            // Direction mirrors the GUI's default down-right extrusion.
            let (dir_x, dir_y) = (0.3, 0.3);
            let layers = ((depth / 0.5).ceil() as usize).clamp(1, 60);
            for i in 0..layers {
                let t = (layers - 1 - i) as f64 / layers as f64;
                let off = depth * t;
                // Shade from dark (back) to the extrusion color (front).
                let shade = 0.55 + 0.45 * (1.0 - t);
                let layer_color = darken(ext_color, shade);
                let layer_path = offset_path(&path, dir_x * off, dir_y * off);
                out.push(Primitive::FillPath {
                    path: layer_path,
                    paint: crate::primitive::Paint::Solid(
                        layer_color.with_alpha(layer_color.a * params.opacity),
                    ),
                    fill_rule: FillRule::NonZero,
                    transform: params.transform,
                    clip: params.clip,
                    effects: Vec::new(),
                });
            }
        }

        if let Some(paint) = &fill_paint {
            out.push(Primitive::FillPath {
                path: path.clone(),
                paint: paint.clone(),
                fill_rule: FillRule::NonZero,
                transform: params.transform,
                clip: params.clip,
                effects: params.effects.clone(),
            });
        }
        if let Some(stroke) = params.line {
            out.push(Primitive::StrokePath {
                path,
                paint: stroke_paint(stroke),
                width: stroke.width,
                cap: LineCap::from(stroke.cap),
                join: LineJoin::from(stroke.join),
                dash: stroke.dash_pattern.clone(),
                transform: params.transform,
                clip: params.clip,
                effects: Vec::new(),
            });
        }
    }
    out.len() - start
}

/// Lay glyphs along a custom path (text-on-path), emitting fill/stroke
/// primitives. The path is flattened and each glyph placed by arc length,
/// rotated to the local tangent.
fn resolve_on_path(
    params: &ResolveParams<'_>,
    model_path: &ModelPath,
    scheme: &repub_color::ColorScheme,
    font_bytes: &[u8],
    out: &mut Vec<Primitive>,
) -> usize {
    let Some(face) = OutlineFace::parse(font_bytes) else {
        return 0;
    };

    let geom = model_path_to_geom(model_path);
    let polyline: Vec<Point> = geom.flatten(0.25).into_iter().flatten().collect();
    if polyline.len() < 2 {
        return 0;
    }

    // Cumulative arc length along the flattened path.
    let mut lengths = vec![0.0];
    for w in polyline.windows(2) {
        let d = w[0].distance(w[1]);
        lengths.push(lengths.last().unwrap() + d);
    }
    let total_len = *lengths.last().unwrap();
    if total_len <= 0.0 {
        return 0;
    }

    // Scale the text so its cap height is a sensible fraction of the box.
    let font_size = params.bounds.height().max(1.0) * 0.6;
    let fill_paint = fill_to_paint(params.fill, params.bounds, scheme)
        .map(|p| crate::convert::paint_with_opacity(p, params.opacity));

    let mut cursor = 0.0;

    let start = out.len();
    for ch in params.text.chars() {
        let Some(outline) = face.outline(ch) else {
            continue;
        };
        let adv = outline.advance * font_size;
        if cursor > total_len {
            break;
        }
        // Find the point and tangent at `cursor` along the polyline.
        let (pos, tangent) = sample_polyline(&polyline, &lengths, cursor);
        let angle = tangent.dy.atan2(tangent.dx);

        if !outline.segments.is_empty() {
            // Build the glyph path in em units scaled to font_size, baseline
            // centered on the path, then rotate+translate onto the path point.
            let scale = font_size;
            let rot = Transform2D::rotate(angle);
            let place = Transform2D::translate(pos.x, pos.y).concat(&rot);
            let path = glyph_to_path_on_path(&outline, scale, &place);
            if let Some(paint) = &fill_paint {
                out.push(Primitive::FillPath {
                    path: path.clone(),
                    paint: paint.clone(),
                    fill_rule: FillRule::NonZero,
                    transform: params.transform,
                    clip: params.clip,
                    effects: params.effects.clone(),
                });
            }
            if let Some(stroke) = params.line {
                out.push(Primitive::StrokePath {
                    path,
                    paint: stroke_paint(stroke),
                    width: stroke.width,
                    cap: LineCap::from(stroke.cap),
                    join: LineJoin::from(stroke.join),
                    dash: stroke.dash_pattern.clone(),
                    transform: params.transform,
                    clip: params.clip,
                    effects: Vec::new(),
                });
            }
        }
        cursor += adv;
    }
    out.len() - start
}

/// Build a glyph path positioned at a point on a path. The glyph's em units
/// are scaled by `scale`, its baseline shifted so the em box is vertically
/// centered, then transformed by `place` (rotation + translation).
fn glyph_to_path_on_path(outline: &GlyphOutline, scale: f64, place: &Transform2D) -> GeomPath {
    // In font space y is up; flip to page space (y down) and place the glyph
    // baseline on the path point.
    let tf = |p: Point| -> Point {
        let local = Point::new(p.x * scale, -(p.y * scale));
        place.transform_point(local)
    };
    let mut path = GeomPath::new();
    for seg in &outline.segments {
        match *seg {
            OutlineSegment::MoveTo(p) => {
                path.move_to(tf(p));
            }
            OutlineSegment::LineTo(p) => {
                path.line_to(tf(p));
            }
            OutlineSegment::QuadTo(c, e) => {
                path.quad_to(tf(c), tf(e));
            }
            OutlineSegment::CubicTo(c1, c2, e) => {
                path.cubic_to(tf(c1), tf(c2), tf(e));
            }
            OutlineSegment::Close => {
                path.close();
            }
        }
    }
    path
}

/// Sample the position and (unit) tangent of a polyline at arc length `d`.
fn sample_polyline(
    points: &[Point],
    lengths: &[f64],
    d: f64,
) -> (Point, repub_geometry::Vector) {
    use repub_geometry::Vector;
    if d <= 0.0 {
        let t = points[0].to(points[1]).normalized();
        return (points[0], t);
    }
    for i in 0..points.len() - 1 {
        if d <= lengths[i + 1] {
            let seg = lengths[i + 1] - lengths[i];
            let local = if seg > 0.0 { (d - lengths[i]) / seg } else { 0.0 };
            let pos = points[i].lerp(points[i + 1], local);
            let t = points[i].to(points[i + 1]).normalized();
            return (pos, t);
        }
    }
    let n = points.len();
    let t = points[n - 2].to(points[n - 1]).normalized();
    (points[n - 1], if t == Vector::ZERO { Vector::new(1.0, 0.0) } else { t })
}
