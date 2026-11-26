//! # Repub Hit
//!
//! Hit-testing a composed [`Scene`] for a selection UI.
//!
//! This consolidates the point-in-path, stroke-distance and bounds logic that
//! previously lived in the CPU renderer and the GUI into one implementation
//! that operates directly on scene [`Primitive`]s. Each primitive is flattened
//! into page space (its transform applied), then tested with the correct fill
//! rule for fills, a width-plus-tolerance band for strokes, and bounds for
//! glyph runs and images.
//!
//! Primitives are tested topmost-first (the scene lists them back-to-front), so
//! [`hit_test`] returns the index of the front-most primitive under a point.

#![warn(missing_docs)]
#![warn(clippy::all)]

use repub_geometry::{Path, Point, Rect, Transform2D};
use repub_scene::{FillRule, Primitive, Scene};

/// Flattening tolerance in points for curves during hit testing.
const FLATTEN_TOLERANCE: f64 = 0.5;

/// Options controlling hit testing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitOptions {
    /// Extra slack in points around strokes and edges.
    pub tolerance: f64,
    /// Whether to count the inside of filled paths as a hit.
    pub test_fills: bool,
    /// Whether to count being near a stroke as a hit.
    pub test_strokes: bool,
}

impl Default for HitOptions {
    fn default() -> Self {
        Self {
            tolerance: 3.0,
            test_fills: true,
            test_strokes: true,
        }
    }
}

impl HitOptions {
    /// Options testing fills only.
    pub fn fills_only() -> Self {
        Self {
            test_fills: true,
            test_strokes: false,
            ..Self::default()
        }
    }

    /// Options testing strokes only.
    pub fn strokes_only() -> Self {
        Self {
            test_fills: false,
            test_strokes: true,
            ..Self::default()
        }
    }

    /// Set the tolerance.
    pub fn with_tolerance(mut self, tolerance: f64) -> Self {
        self.tolerance = tolerance;
        self
    }
}

/// The index of the front-most primitive under `point`, if any, using default
/// options.
pub fn hit_test(scene: &Scene, point: Point) -> Option<usize> {
    hit_test_with(scene, point, &HitOptions::default())
}

/// The index of the front-most primitive under `point`, if any, using `options`.
pub fn hit_test_with(scene: &Scene, point: Point, options: &HitOptions) -> Option<usize> {
    for (index, primitive) in scene.primitives.iter().enumerate().rev() {
        if hit_primitive(primitive, point, options) {
            return Some(index);
        }
    }
    None
}

/// Every primitive index under `point`, front-most first.
pub fn hit_all(scene: &Scene, point: Point, options: &HitOptions) -> Vec<usize> {
    let mut out = Vec::new();
    for (index, primitive) in scene.primitives.iter().enumerate().rev() {
        if hit_primitive(primitive, point, options) {
            out.push(index);
        }
    }
    out
}

/// Whether `point` hits `primitive` under `options`.
pub fn hit_primitive(primitive: &Primitive, point: Point, options: &HitOptions) -> bool {
    match primitive {
        Primitive::FillPath {
            path,
            fill_rule,
            transform,
            clip,
            ..
        } => {
            if !within_clip(*clip, point) {
                return false;
            }
            let contours = flatten_in_page(path, *transform);
            options.test_fills && point_in_contours(&contours, point, *fill_rule)
        }
        Primitive::StrokePath {
            path,
            width,
            transform,
            clip,
            ..
        } => {
            if !within_clip(*clip, point) {
                return false;
            }
            if !options.test_strokes {
                return false;
            }
            let contours = flatten_in_page(path, *transform);
            let reach = options.tolerance + width.max(0.0) / 2.0;
            contours
                .iter()
                .any(|c| distance_to_polyline(point, c) <= reach)
        }
        Primitive::Glyphs {
            glyphs,
            size,
            transform,
            clip,
            ..
        } => {
            if !within_clip(*clip, point) {
                return false;
            }
            // Treat each glyph as a band around its baseline, since the scene
            // does not carry glyph ink outlines for a text run.
            glyphs.iter().any(|g| {
                let ascent = size * 0.8;
                let descent = size * 0.25;
                let rect = Rect::new(g.x, g.y - ascent, g.advance.max(1.0), ascent + descent);
                rect_hit(rect, *transform, point, options.tolerance)
            })
        }
        Primitive::Image {
            dest,
            transform,
            clip,
            ..
        } => {
            if !within_clip(*clip, point) {
                return false;
            }
            rect_hit(*dest, *transform, point, options.tolerance)
        }
    }
}

/// The page-space bounding rectangle of a primitive, if it has geometry.
pub fn primitive_bounds(primitive: &Primitive) -> Option<Rect> {
    match primitive {
        Primitive::FillPath {
            path, transform, ..
        }
        | Primitive::StrokePath {
            path, transform, ..
        } => {
            let contours = flatten_in_page(path, *transform);
            bounds_of_contours(&contours)
        }
        Primitive::Glyphs {
            glyphs,
            size,
            transform,
            ..
        } => {
            let mut pts = Vec::new();
            for g in glyphs {
                let ascent = size * 0.8;
                let descent = size * 0.25;
                let r = Rect::new(g.x, g.y - ascent, g.advance.max(1.0), ascent + descent);
                pts.extend(rect_corners(r).map(|p| transform.transform_point(p)));
            }
            bounds_of_points(&pts)
        }
        Primitive::Image {
            dest, transform, ..
        } => {
            let pts: Vec<Point> = rect_corners(*dest)
                .map(|p| transform.transform_point(p))
                .to_vec();
            bounds_of_points(&pts)
        }
    }
}

/// The page-space bounding rectangle enclosing the whole scene's geometry.
pub fn scene_bounds(scene: &Scene) -> Option<Rect> {
    let mut acc: Option<Rect> = None;
    for p in &scene.primitives {
        if let Some(b) = primitive_bounds(p) {
            acc = Some(match acc {
                Some(a) => a.union(&b),
                None => b,
            });
        }
    }
    acc
}

fn within_clip(clip: Option<Rect>, point: Point) -> bool {
    clip.map_or(true, |r| r.contains(point))
}

/// Flatten a path and apply `transform` to every point, yielding page-space
/// contours.
fn flatten_in_page(path: &Path, transform: Transform2D) -> Vec<Vec<Point>> {
    path.flatten(FLATTEN_TOLERANCE)
        .into_iter()
        .map(|contour| {
            contour
                .into_iter()
                .map(|p| transform.transform_point(p))
                .collect()
        })
        .collect()
}

/// Whether `point` is inside the set of `contours` under `rule`.
fn point_in_contours(contours: &[Vec<Point>], point: Point, rule: FillRule) -> bool {
    match rule {
        FillRule::NonZero => {
            contours
                .iter()
                .map(|c| winding_number(point, c))
                .sum::<i32>()
                != 0
        }
        FillRule::EvenOdd => {
            let crossings: usize = contours
                .iter()
                .filter(|c| point_in_polygon(point, c))
                .count();
            crossings % 2 == 1
        }
    }
}

/// Winding number of the closed polygon `points` around `p`.
fn winding_number(p: Point, points: &[Point]) -> i32 {
    if points.len() < 3 {
        return 0;
    }
    let mut wn = 0;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        let cross = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
        if a.y <= p.y {
            if b.y > p.y && cross > 0.0 {
                wn += 1;
            }
        } else if b.y <= p.y && cross < 0.0 {
            wn -= 1;
        }
    }
    wn
}

/// Even-odd ray-cast point-in-polygon test.
fn point_in_polygon(p: Point, points: &[Point]) -> bool {
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let a = points[i];
        let b = points[j];
        if (a.y > p.y) != (b.y > p.y) {
            let denom = b.y - a.y;
            if denom != 0.0 {
                let x_cross = a.x + (p.y - a.y) / denom * (b.x - a.x);
                if p.x < x_cross {
                    inside = !inside;
                }
            }
        }
        j = i;
    }
    inside
}

/// Shortest distance from `p` to a polyline (open).
fn distance_to_polyline(p: Point, points: &[Point]) -> f64 {
    if points.is_empty() {
        return f64::INFINITY;
    }
    if points.len() == 1 {
        return p.distance(points[0]);
    }
    let mut best = f64::INFINITY;
    for seg in points.windows(2) {
        best = best.min(distance_to_segment(p, seg[0], seg[1]));
    }
    best
}

/// Shortest distance from `p` to the segment `a`–`b`.
fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 {
        return p.distance(a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq).clamp(0.0, 1.0);
    let proj = Point::new(a.x + dx * t, a.y + dy * t);
    p.distance(proj)
}

/// Whether `point` hits a rectangle transformed into page space, within
/// `tolerance`. The rectangle's four corners are transformed, then tested as a
/// polygon with a stroke band around its edges.
fn rect_hit(rect: Rect, transform: Transform2D, point: Point, tolerance: f64) -> bool {
    let corners: Vec<Point> = rect_corners(rect)
        .map(|p| transform.transform_point(p))
        .to_vec();
    if point_in_polygon(point, &corners) {
        return true;
    }
    // Edge band (so thin/degenerate rects still register near their outline).
    let mut ring = corners.clone();
    ring.push(corners[0]);
    distance_to_polyline(point, &ring) <= tolerance
}

fn rect_corners(r: Rect) -> [Point; 4] {
    [
        Point::new(r.x(), r.y()),
        Point::new(r.right(), r.y()),
        Point::new(r.right(), r.bottom()),
        Point::new(r.x(), r.bottom()),
    ]
}

fn bounds_of_contours(contours: &[Vec<Point>]) -> Option<Rect> {
    let pts: Vec<Point> = contours.iter().flatten().copied().collect();
    bounds_of_points(&pts)
}

fn bounds_of_points(points: &[Point]) -> Option<Rect> {
    let mut min_x = f64::INFINITY;
    let mut min_y = f64::INFINITY;
    let mut max_x = f64::NEG_INFINITY;
    let mut max_y = f64::NEG_INFINITY;
    for p in points {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }
    if min_x.is_finite() {
        Some(Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_color::Color;
    use repub_geometry::{Path, Size};
    use repub_scene::{FillRule, Paint, Primitive, Scene};

    fn fill_rect(r: Rect) -> Primitive {
        Primitive::FillPath {
            path: Path::from_rect(r),
            paint: Paint::Solid(Color::RED),
            fill_rule: FillRule::NonZero,
            transform: Transform2D::IDENTITY,
            clip: None,
            effects: Vec::new(),
        }
    }

    #[test]
    fn picks_topmost() {
        let mut scene = Scene::new(Size::new(200.0, 200.0));
        scene.push(fill_rect(Rect::new(10.0, 10.0, 100.0, 100.0)));
        scene.push(fill_rect(Rect::new(50.0, 50.0, 100.0, 100.0)));

        // Overlap region: the second (topmost) primitive wins.
        assert_eq!(hit_test(&scene, Point::new(60.0, 60.0)), Some(1));
        // Only the first covers this point.
        assert_eq!(hit_test(&scene, Point::new(20.0, 20.0)), Some(0));
        // Outside both.
        assert_eq!(hit_test(&scene, Point::new(180.0, 180.0)), None);
    }

    #[test]
    fn hit_all_returns_overlaps() {
        let mut scene = Scene::new(Size::new(200.0, 200.0));
        scene.push(fill_rect(Rect::new(10.0, 10.0, 100.0, 100.0)));
        scene.push(fill_rect(Rect::new(50.0, 50.0, 100.0, 100.0)));
        let all = hit_all(&scene, Point::new(60.0, 60.0), &HitOptions::default());
        assert_eq!(all, vec![1, 0]);
    }

    #[test]
    fn stroke_near_edge() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0));
        path.line_to(Point::new(100.0, 0.0));
        let prim = Primitive::StrokePath {
            path,
            paint: Paint::Solid(Color::BLACK),
            width: 2.0,
            cap: repub_scene::LineCap::Butt,
            join: repub_scene::LineJoin::Miter,
            dash: None,
            transform: Transform2D::IDENTITY,
            clip: None,
            effects: Vec::new(),
        };
        let opts = HitOptions::default();
        assert!(hit_primitive(&prim, Point::new(50.0, 1.0), &opts));
        assert!(!hit_primitive(&prim, Point::new(50.0, 20.0), &opts));
    }

    #[test]
    fn transform_is_applied() {
        let mut scene = Scene::new(Size::new(200.0, 200.0));
        scene.push(Primitive::FillPath {
            path: Path::from_rect(Rect::new(0.0, 0.0, 10.0, 10.0)),
            paint: Paint::Solid(Color::RED),
            fill_rule: FillRule::NonZero,
            transform: Transform2D::translate(100.0, 100.0),
            clip: None,
            effects: Vec::new(),
        });
        assert_eq!(hit_test(&scene, Point::new(105.0, 105.0)), Some(0));
        assert_eq!(hit_test(&scene, Point::new(5.0, 5.0)), None);
    }

    #[test]
    fn bounds_cover_geometry() {
        let mut scene = Scene::new(Size::new(200.0, 200.0));
        scene.push(fill_rect(Rect::new(10.0, 20.0, 30.0, 40.0)));
        let b = scene_bounds(&scene).unwrap();
        assert_eq!(b, Rect::new(10.0, 20.0, 30.0, 40.0));
    }
}
