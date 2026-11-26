//! Vector paths built from line and Bézier segments.

use crate::primitives::{Point, Rect};

/// A single segment of a [`Path`].
///
/// Coordinates are absolute. The starting point of each segment is the
/// current point established by the preceding segment (or the most recent
/// [`PathSegment::MoveTo`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathSegment {
    /// Begin a new subpath at the given point.
    MoveTo(Point),
    /// Straight line to the given point.
    LineTo(Point),
    /// Quadratic Bézier with one control point, ending at the second point.
    QuadTo(Point, Point),
    /// Cubic Bézier with two control points, ending at the third point.
    CubicTo(Point, Point, Point),
    /// Close the current subpath back to its start.
    Close,
}

/// A sequence of path segments describing one or more subpaths.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Path {
    /// The ordered segments making up the path.
    pub segments: Vec<PathSegment>,
}

impl Path {
    /// Create an empty path.
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Whether the path has no segments.
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Begin a new subpath at `p`.
    pub fn move_to(&mut self, p: Point) -> &mut Self {
        self.segments.push(PathSegment::MoveTo(p));
        self
    }

    /// Add a straight line to `p`.
    pub fn line_to(&mut self, p: Point) -> &mut Self {
        self.segments.push(PathSegment::LineTo(p));
        self
    }

    /// Add a quadratic Bézier with control `c` ending at `end`.
    pub fn quad_to(&mut self, c: Point, end: Point) -> &mut Self {
        self.segments.push(PathSegment::QuadTo(c, end));
        self
    }

    /// Add a cubic Bézier with controls `c1`, `c2` ending at `end`.
    pub fn cubic_to(&mut self, c1: Point, c2: Point, end: Point) -> &mut Self {
        self.segments.push(PathSegment::CubicTo(c1, c2, end));
        self
    }

    /// Close the current subpath.
    pub fn close(&mut self) -> &mut Self {
        self.segments.push(PathSegment::Close);
        self
    }

    /// Build a rectangular path.
    pub fn from_rect(r: Rect) -> Self {
        let mut path = Path::new();
        path.move_to(Point::new(r.x(), r.y()))
            .line_to(Point::new(r.right(), r.y()))
            .line_to(Point::new(r.right(), r.bottom()))
            .line_to(Point::new(r.x(), r.bottom()))
            .close();
        path
    }

    /// Flatten all curves into connected polylines, one per subpath.
    ///
    /// `tolerance` is the maximum allowed deviation, in points, between a
    /// curve and its polyline approximation. A `Close` segment appends the
    /// subpath's start point to that subpath's polyline.
    pub fn flatten(&self, tolerance: f64) -> Vec<Vec<Point>> {
        let tol = tolerance.max(1e-6);
        let mut contours: Vec<Vec<Point>> = Vec::new();
        let mut current: Vec<Point> = Vec::new();
        let mut start = Point::ZERO;
        let mut cursor = Point::ZERO;

        for seg in &self.segments {
            match *seg {
                PathSegment::MoveTo(p) => {
                    if !current.is_empty() {
                        contours.push(std::mem::take(&mut current));
                    }
                    start = p;
                    cursor = p;
                    current.push(p);
                }
                PathSegment::LineTo(p) => {
                    current.push(p);
                    cursor = p;
                }
                PathSegment::QuadTo(c, end) => {
                    flatten_quad(cursor, c, end, tol, &mut current);
                    cursor = end;
                }
                PathSegment::CubicTo(c1, c2, end) => {
                    flatten_cubic(cursor, c1, c2, end, tol, &mut current);
                    cursor = end;
                }
                PathSegment::Close => {
                    current.push(start);
                    cursor = start;
                }
            }
        }

        if !current.is_empty() {
            contours.push(current);
        }
        contours
    }

    /// The axis-aligned bounding box of the flattened path, or `None` if empty.
    pub fn bounds(&self, tolerance: f64) -> Option<Rect> {
        let mut min_x = f64::INFINITY;
        let mut min_y = f64::INFINITY;
        let mut max_x = f64::NEG_INFINITY;
        let mut max_y = f64::NEG_INFINITY;
        let mut any = false;

        for contour in self.flatten(tolerance) {
            for p in contour {
                any = true;
                min_x = min_x.min(p.x);
                min_y = min_y.min(p.y);
                max_x = max_x.max(p.x);
                max_y = max_y.max(p.y);
            }
        }

        if any {
            Some(Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
        } else {
            None
        }
    }
}

/// Evaluate a quadratic Bézier at parameter `t` in [0, 1].
pub fn quad_point(p0: Point, c: Point, p1: Point, t: f64) -> Point {
    let mt = 1.0 - t;
    let x = mt * mt * p0.x + 2.0 * mt * t * c.x + t * t * p1.x;
    let y = mt * mt * p0.y + 2.0 * mt * t * c.y + t * t * p1.y;
    Point::new(x, y)
}

/// Evaluate a cubic Bézier at parameter `t` in [0, 1].
pub fn cubic_point(p0: Point, c1: Point, c2: Point, p1: Point, t: f64) -> Point {
    let mt = 1.0 - t;
    let a = mt * mt * mt;
    let b = 3.0 * mt * mt * t;
    let c = 3.0 * mt * t * t;
    let d = t * t * t;
    Point::new(
        a * p0.x + b * c1.x + c * c2.x + d * p1.x,
        a * p0.y + b * c1.y + c * c2.y + d * p1.y,
    )
}

fn flatten_quad(p0: Point, c: Point, p1: Point, tol: f64, out: &mut Vec<Point>) {
    // Distance from the control point to the chord bounds the flatness.
    if quad_is_flat(p0, c, p1, tol) {
        out.push(p1);
        return;
    }
    // De Casteljau subdivision at t = 0.5.
    let p01 = midpoint(p0, c);
    let p12 = midpoint(c, p1);
    let mid = midpoint(p01, p12);
    flatten_quad(p0, p01, mid, tol, out);
    flatten_quad(mid, p12, p1, tol, out);
}

fn flatten_cubic(p0: Point, c1: Point, c2: Point, p1: Point, tol: f64, out: &mut Vec<Point>) {
    if cubic_is_flat(p0, c1, c2, p1, tol) {
        out.push(p1);
        return;
    }
    let p01 = midpoint(p0, c1);
    let p12 = midpoint(c1, c2);
    let p23 = midpoint(c2, p1);
    let p012 = midpoint(p01, p12);
    let p123 = midpoint(p12, p23);
    let mid = midpoint(p012, p123);
    flatten_cubic(p0, p01, p012, mid, tol, out);
    flatten_cubic(mid, p123, p23, p1, tol, out);
}

fn midpoint(a: Point, b: Point) -> Point {
    Point::new((a.x + b.x) * 0.5, (a.y + b.y) * 0.5)
}

fn quad_is_flat(p0: Point, c: Point, p1: Point, tol: f64) -> bool {
    point_line_distance(c, p0, p1) <= tol
}

fn cubic_is_flat(p0: Point, c1: Point, c2: Point, p1: Point, tol: f64) -> bool {
    point_line_distance(c1, p0, p1) <= tol && point_line_distance(c2, p0, p1) <= tol
}

/// Perpendicular distance from `p` to the segment `a`–`b`.
fn point_line_distance(p: Point, a: Point, b: Point) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 {
        return a.to(p).length();
    }
    let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq;
    let t = t.clamp(0.0, 1.0);
    let proj = Point::new(a.x + dx * t, a.y + dy * t);
    proj.to(p).length()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_path_flattens_to_closed_polygon() {
        let path = Path::from_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        let contours = path.flatten(0.1);
        assert_eq!(contours.len(), 1);
        // 4 corners + closing point back to start.
        assert_eq!(contours[0].len(), 5);
        assert_eq!(contours[0].first(), contours[0].last());
    }

    #[test]
    fn straight_line_quad_stays_flat() {
        // A quad whose control point is on the line should not subdivide.
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0))
            .quad_to(Point::new(5.0, 0.0), Point::new(10.0, 0.0));
        let contours = path.flatten(0.01);
        assert_eq!(contours[0].len(), 2);
    }

    #[test]
    fn curved_cubic_subdivides() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0)).cubic_to(
            Point::new(0.0, 10.0),
            Point::new(10.0, 10.0),
            Point::new(10.0, 0.0),
        );
        let contours = path.flatten(0.1);
        assert!(contours[0].len() > 2);
    }

    #[test]
    fn bounds_of_curve() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(10.0, 0.0))
            .line_to(Point::new(10.0, 5.0));
        let b = path.bounds(0.1).unwrap();
        assert_eq!(b, Rect::new(0.0, 0.0, 10.0, 5.0));
    }

    #[test]
    fn bezier_evaluation_endpoints() {
        let p0 = Point::new(1.0, 2.0);
        let p1 = Point::new(9.0, 8.0);
        assert_eq!(
            cubic_point(p0, Point::new(3.0, 3.0), Point::new(7.0, 7.0), p1, 0.0),
            p0
        );
        assert_eq!(
            cubic_point(p0, Point::new(3.0, 3.0), Point::new(7.0, 7.0), p1, 1.0),
            p1
        );
        assert_eq!(quad_point(p0, Point::new(5.0, 5.0), p1, 0.0), p0);
    }
}
