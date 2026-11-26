//! Points, vectors, sizes, and rectangles.

/// A 2D point with x and y coordinates, in points.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    /// X coordinate in points.
    pub x: f64,
    /// Y coordinate in points.
    pub y: f64,
}

impl Point {
    /// Origin point (0, 0).
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    /// Create a new point.
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }

    /// The vector from this point to `other`.
    pub fn to(self, other: Point) -> Vector {
        Vector::new(other.x - self.x, other.y - self.y)
    }

    /// Translate this point by a vector.
    pub fn translated(self, v: Vector) -> Point {
        Point::new(self.x + v.dx, self.y + v.dy)
    }

    /// Euclidean distance to another point.
    pub fn distance(self, other: Point) -> f64 {
        self.to(other).length()
    }

    /// Linear interpolation toward `other` by factor `t` (unclamped).
    pub fn lerp(self, other: Point, t: f64) -> Point {
        Point::new(self.x + (other.x - self.x) * t, self.y + (other.y - self.y) * t)
    }
}

/// A 2D displacement.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vector {
    /// X component.
    pub dx: f64,
    /// Y component.
    pub dy: f64,
}

impl Vector {
    /// Zero vector.
    pub const ZERO: Self = Self { dx: 0.0, dy: 0.0 };

    /// Create a new vector.
    pub const fn new(dx: f64, dy: f64) -> Self {
        Self { dx, dy }
    }

    /// Magnitude of the vector.
    pub fn length(self) -> f64 {
        (self.dx * self.dx + self.dy * self.dy).sqrt()
    }

    /// Dot product with another vector.
    pub fn dot(self, other: Vector) -> f64 {
        self.dx * other.dx + self.dy * other.dy
    }

    /// Unit vector in the same direction, or the zero vector if length is zero.
    pub fn normalized(self) -> Vector {
        let len = self.length();
        if len == 0.0 {
            Vector::ZERO
        } else {
            Vector::new(self.dx / len, self.dy / len)
        }
    }

    /// Scale the vector by a scalar.
    pub fn scaled(self, s: f64) -> Vector {
        Vector::new(self.dx * s, self.dy * s)
    }
}

/// A 2D size with width and height, in points.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    /// Width in points.
    pub width: f64,
    /// Height in points.
    pub height: f64,
}

impl Size {
    /// Zero size.
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    /// Create a new size.
    pub const fn new(width: f64, height: f64) -> Self {
        Self { width, height }
    }

    /// Area of the size.
    pub fn area(self) -> f64 {
        self.width * self.height
    }
}

/// A rectangle defined by an origin point and a size.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    /// Top-left origin point.
    pub origin: Point,
    /// Size of the rectangle.
    pub size: Size,
}

impl Rect {
    /// Create a new rectangle from x, y, width, height.
    pub const fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    /// Create from an origin and a size.
    pub const fn from_origin_size(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }

    /// Create from two corner points, normalizing so the result is non-negative.
    pub fn from_corners(a: Point, b: Point) -> Self {
        let x = a.x.min(b.x);
        let y = a.y.min(b.y);
        Self::new(x, y, (a.x - b.x).abs(), (a.y - b.y).abs())
    }

    /// X coordinate of the left edge.
    pub fn x(&self) -> f64 {
        self.origin.x
    }

    /// Y coordinate of the top edge.
    pub fn y(&self) -> f64 {
        self.origin.y
    }

    /// Width of the rectangle.
    pub fn width(&self) -> f64 {
        self.size.width
    }

    /// Height of the rectangle.
    pub fn height(&self) -> f64 {
        self.size.height
    }

    /// X coordinate of the right edge.
    pub fn right(&self) -> f64 {
        self.origin.x + self.size.width
    }

    /// Y coordinate of the bottom edge.
    pub fn bottom(&self) -> f64 {
        self.origin.y + self.size.height
    }

    /// Center point of the rectangle.
    pub fn center(&self) -> Point {
        Point::new(
            self.origin.x + self.size.width / 2.0,
            self.origin.y + self.size.height / 2.0,
        )
    }

    /// Check if the rectangle contains a point (edges inclusive).
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.origin.x
            && point.x <= self.right()
            && point.y >= self.origin.y
            && point.y <= self.bottom()
    }

    /// Check if this rectangle intersects another.
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x() < other.right()
            && self.right() > other.x()
            && self.y() < other.bottom()
            && self.bottom() > other.y()
    }

    /// Check if this rectangle fully contains another.
    pub fn contains_rect(&self, other: &Rect) -> bool {
        other.x() >= self.x()
            && other.right() <= self.right()
            && other.y() >= self.y()
            && other.bottom() <= self.bottom()
    }

    /// The intersection of two rectangles, if any.
    pub fn intersection(&self, other: &Rect) -> Option<Rect> {
        if !self.intersects(other) {
            return None;
        }
        let x = self.x().max(other.x());
        let y = self.y().max(other.y());
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        Some(Rect::new(x, y, right - x, bottom - y))
    }

    /// The smallest rectangle containing both rectangles.
    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x().min(other.x());
        let y = self.y().min(other.y());
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Rect::new(x, y, right - x, bottom - y)
    }

    /// Shrink the rectangle by `amount` on all sides (clamped to non-negative size).
    pub fn inset(&self, amount: f64) -> Rect {
        Rect::new(
            self.x() + amount,
            self.y() + amount,
            (self.width() - 2.0 * amount).max(0.0),
            (self.height() - 2.0 * amount).max(0.0),
        )
    }

    /// Grow the rectangle by `amount` on all sides.
    pub fn outset(&self, amount: f64) -> Rect {
        self.inset(-amount)
    }

    /// Translate the rectangle by a vector.
    pub fn translated(&self, v: Vector) -> Rect {
        Rect::from_origin_size(self.origin.translated(v), self.size)
    }

    /// Whether the rectangle is empty (zero or negative size).
    pub fn is_empty(&self) -> bool {
        self.size.width <= 0.0 || self.size.height <= 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn point_distance_and_lerp() {
        let a = Point::new(0.0, 0.0);
        let b = Point::new(3.0, 4.0);
        assert!((a.distance(b) - 5.0).abs() < 1e-9);
        let mid = a.lerp(b, 0.5);
        assert_eq!(mid, Point::new(1.5, 2.0));
    }

    #[test]
    fn vector_normalize() {
        let v = Vector::new(0.0, 2.0).normalized();
        assert_eq!(v, Vector::new(0.0, 1.0));
        assert_eq!(Vector::ZERO.normalized(), Vector::ZERO);
    }

    #[test]
    fn rect_contains_and_intersection() {
        let r = Rect::new(10.0, 10.0, 100.0, 100.0);
        assert!(r.contains(Point::new(50.0, 50.0)));
        assert!(!r.contains(Point::new(0.0, 0.0)));

        let other = Rect::new(60.0, 60.0, 100.0, 100.0);
        let inter = r.intersection(&other).unwrap();
        assert_eq!(inter, Rect::new(60.0, 60.0, 50.0, 50.0));
    }

    #[test]
    fn rect_from_corners_normalizes() {
        let r = Rect::from_corners(Point::new(30.0, 40.0), Point::new(10.0, 10.0));
        assert_eq!(r, Rect::new(10.0, 10.0, 20.0, 30.0));
    }

    #[test]
    fn rect_union() {
        let a = Rect::new(0.0, 0.0, 10.0, 10.0);
        let b = Rect::new(20.0, 20.0, 10.0, 10.0);
        assert_eq!(a.union(&b), Rect::new(0.0, 0.0, 30.0, 30.0));
    }
}
