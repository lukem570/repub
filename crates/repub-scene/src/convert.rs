//! Conversions between model-layer and geometry-layer vocabulary.
//!
//! The model and escher crates describe paths with
//! [`repub_model::geometry::Path`] (a list of [`PathCommand`]s and per-subpath
//! styles), while the scene's [`Primitive`](crate::Primitive)s carry the
//! geometry-layer [`repub_geometry::Path`] (a flat list of [`PathSegment`]s).
//! These helpers bridge the two, and resolve model [`Fill`]/[`Stroke`] styling
//! into concrete scene [`Paint`]s.

use repub_color::{Color, ColorScheme};
use repub_geometry::{Path as GeomPath, PathSegment, Point, Rect};
use repub_model::geometry::{Path as ModelPath, PathCommand};
use repub_model::styles::{Fill, Stroke};

use crate::primitive::{FillRule, Paint};

/// Convert a model path into a geometry path, dropping per-subpath styles.
///
/// `MoveTo`/`LineTo`/`Close` map directly; quadratic and cubic Béziers are
/// preserved as curve segments.
pub fn model_path_to_geom(path: &ModelPath) -> GeomPath {
    let mut out = GeomPath::new();
    for cmd in &path.commands {
        match *cmd {
            PathCommand::MoveTo(p) => {
                out.move_to(p);
            }
            PathCommand::LineTo(p) => {
                out.line_to(p);
            }
            PathCommand::QuadTo { control, end } => {
                out.quad_to(control, end);
            }
            PathCommand::CubicTo {
                control1,
                control2,
                end,
            } => {
                out.cubic_to(control1, control2, end);
            }
            PathCommand::Close => {
                out.close();
            }
        }
    }
    out
}

/// The fill rule a model path uses.
pub fn model_path_fill_rule(path: &ModelPath) -> FillRule {
    if path.even_odd {
        FillRule::EvenOdd
    } else {
        FillRule::NonZero
    }
}

/// A geometry rectangle path (`MoveTo` through four corners, then `Close`).
pub fn rect_path(r: Rect) -> GeomPath {
    GeomPath::from_rect(r)
}

/// Resolve a model [`Fill`] into a concrete scene [`Paint`] for `bounds`.
///
/// Returns `None` for [`Fill::None`]. Gradient and pattern paints carry
/// `bounds` so a backend can map their normalized coordinates. Texture fills,
/// which the scene does not model as a paint, fall back to a solid white.
pub fn fill_to_paint(fill: &Fill, bounds: Rect, _scheme: &ColorScheme) -> Option<Paint> {
    match fill {
        Fill::None => None,
        Fill::Solid(c) => Some(Paint::Solid(*c)),
        Fill::LinearGradient(g) => Some(Paint::LinearGradient {
            gradient: g.clone(),
            bounds,
        }),
        Fill::RadialGradient(g) => Some(Paint::RadialGradient {
            gradient: g.clone(),
            bounds,
        }),
        Fill::Pattern(p) => Some(Paint::Pattern {
            pattern: p.clone(),
            bounds,
        }),
        Fill::Texture(_) => Some(Paint::Solid(Color::WHITE)),
    }
}

/// Resolve a stroke's color into a solid scene [`Paint`].
pub fn stroke_paint(stroke: &Stroke) -> Paint {
    Paint::Solid(stroke.color)
}

/// Apply an opacity factor to a paint's colors in place where it is cheap to
/// do so (solid colors and gradient stops); patterns are returned unchanged.
pub fn paint_with_opacity(paint: Paint, opacity: f64) -> Paint {
    if opacity >= 1.0 {
        return paint;
    }
    match paint {
        Paint::Solid(c) => Paint::Solid(c.with_alpha(c.a * opacity)),
        Paint::LinearGradient { mut gradient, bounds } => {
            for stop in &mut gradient.stops {
                stop.color = stop.color.with_alpha(stop.color.a * opacity);
            }
            Paint::LinearGradient { gradient, bounds }
        }
        Paint::RadialGradient { mut gradient, bounds } => {
            for stop in &mut gradient.stops {
                stop.color = stop.color.with_alpha(stop.color.a * opacity);
            }
            Paint::RadialGradient { gradient, bounds }
        }
        other => other,
    }
}

/// A short straight polyline path through `points`.
pub fn polyline_path(points: &[Point], close: bool) -> GeomPath {
    let mut path = GeomPath::new();
    let mut iter = points.iter();
    if let Some(first) = iter.next() {
        path.move_to(*first);
        for p in iter {
            path.line_to(*p);
        }
        if close {
            path.close();
        }
    }
    path
}

/// The number of raw segments in a geometry path (for assertions and tests).
pub fn segment_count(path: &GeomPath) -> usize {
    path.segments.len()
}

/// Whether a geometry path contains at least one `MoveTo`.
pub fn has_subpath(path: &GeomPath) -> bool {
    path.segments
        .iter()
        .any(|s| matches!(s, PathSegment::MoveTo(_)))
}
