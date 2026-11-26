//! 2D affine transformations.

use crate::primitives::{Point, Vector};

/// A 2D affine transformation matrix.
///
/// Represented as a 3x2 matrix:
///
/// ```text
/// | a  c  tx |
/// | b  d  ty |
/// | 0  0  1  |
/// ```
///
/// A point `(x, y)` maps to `(a*x + c*y + tx, b*x + d*y + ty)`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    /// Row 0, col 0 (x scale / rotation).
    pub a: f64,
    /// Row 1, col 0 (y shear / rotation).
    pub b: f64,
    /// Row 0, col 1 (x shear / rotation).
    pub c: f64,
    /// Row 1, col 1 (y scale / rotation).
    pub d: f64,
    /// Translation x.
    pub tx: f64,
    /// Translation y.
    pub ty: f64,
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Transform2D {
    /// Identity transform (no change).
    pub const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        tx: 0.0,
        ty: 0.0,
    };

    /// Create a transform from its six components.
    pub const fn new(a: f64, b: f64, c: f64, d: f64, tx: f64, ty: f64) -> Self {
        Self { a, b, c, d, tx, ty }
    }

    /// Create a translation transform.
    pub fn translate(tx: f64, ty: f64) -> Self {
        Self {
            a: 1.0,
            b: 0.0,
            c: 0.0,
            d: 1.0,
            tx,
            ty,
        }
    }

    /// Create a scale transform.
    pub fn scale(sx: f64, sy: f64) -> Self {
        Self {
            a: sx,
            b: 0.0,
            c: 0.0,
            d: sy,
            tx: 0.0,
            ty: 0.0,
        }
    }

    /// Create a rotation transform (angle in radians).
    pub fn rotate(angle: f64) -> Self {
        let cos = angle.cos();
        let sin = angle.sin();
        Self {
            a: cos,
            b: sin,
            c: -sin,
            d: cos,
            tx: 0.0,
            ty: 0.0,
        }
    }

    /// Transform a point.
    pub fn transform_point(&self, point: Point) -> Point {
        Point::new(
            self.a * point.x + self.c * point.y + self.tx,
            self.b * point.x + self.d * point.y + self.ty,
        )
    }

    /// Transform a vector (ignores translation).
    pub fn transform_vector(&self, v: Vector) -> Vector {
        Vector::new(self.a * v.dx + self.c * v.dy, self.b * v.dx + self.d * v.dy)
    }

    /// Alias for [`Transform2D::transform_point`].
    pub fn apply(&self, point: Point) -> Point {
        self.transform_point(point)
    }

    /// Compose with another transform, applying `other` first then `self`.
    pub fn then(&self, outer: &Self) -> Self {
        outer.concat(self)
    }

    /// Concatenate two transforms: `self` applied after `other`
    /// (`self * other`, so `other` is applied to a point first).
    pub fn concat(&self, other: &Self) -> Self {
        Self {
            a: self.a * other.a + self.c * other.b,
            b: self.b * other.a + self.d * other.b,
            c: self.a * other.c + self.c * other.d,
            d: self.b * other.c + self.d * other.d,
            tx: self.a * other.tx + self.c * other.ty + self.tx,
            ty: self.b * other.tx + self.d * other.ty + self.ty,
        }
    }

    /// The determinant of the linear part.
    pub fn determinant(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// The inverse transform, or `None` if the matrix is singular.
    pub fn inverse(&self) -> Option<Self> {
        let det = self.determinant();
        if det.abs() < f64::EPSILON {
            return None;
        }
        let inv_det = 1.0 / det;
        let a = self.d * inv_det;
        let b = -self.b * inv_det;
        let c = -self.c * inv_det;
        let d = self.a * inv_det;
        Some(Self {
            a,
            b,
            c,
            d,
            tx: -(a * self.tx + c * self.ty),
            ty: -(b * self.tx + d * self.ty),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_is_noop() {
        let p = Point::new(10.0, 20.0);
        assert_eq!(Transform2D::IDENTITY.transform_point(p), p);
    }

    #[test]
    fn translate_moves_point() {
        let t = Transform2D::translate(5.0, 10.0);
        assert_eq!(t.transform_point(Point::new(10.0, 20.0)), Point::new(15.0, 30.0));
    }

    #[test]
    fn concat_applies_other_first() {
        // Scale by 2 then translate by (1, 1).
        let scale = Transform2D::scale(2.0, 2.0);
        let translate = Transform2D::translate(1.0, 1.0);
        let combined = translate.concat(&scale);
        assert_eq!(combined.transform_point(Point::new(3.0, 4.0)), Point::new(7.0, 9.0));
    }

    #[test]
    fn inverse_round_trips() {
        let t = Transform2D::translate(5.0, -3.0).concat(&Transform2D::scale(2.0, 4.0));
        let inv = t.inverse().unwrap();
        let p = Point::new(7.0, 11.0);
        let back = inv.transform_point(t.transform_point(p));
        assert!((back.x - p.x).abs() < 1e-9);
        assert!((back.y - p.y).abs() < 1e-9);
    }

    #[test]
    fn singular_has_no_inverse() {
        assert!(Transform2D::scale(0.0, 1.0).inverse().is_none());
    }
}
