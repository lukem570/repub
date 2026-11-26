//! Conversion from scene geometry to tiny-skia types.

use repub_geometry::{Path as ScenePath, PathSegment, Rect, Transform2D};
use tiny_skia::{Path, PathBuilder, Transform};

/// Convert a [`ScenePath`] into a tiny-skia [`Path`].
///
/// Returns `None` when the path is empty or cannot form a valid tiny-skia
/// path (e.g. no drawable segments).
pub fn convert_path(path: &ScenePath) -> Option<Path> {
    let mut builder = PathBuilder::new();
    let mut has_current = false;

    for seg in &path.segments {
        match *seg {
            PathSegment::MoveTo(p) => {
                builder.move_to(p.x as f32, p.y as f32);
                has_current = true;
            }
            PathSegment::LineTo(p) => {
                if !has_current {
                    builder.move_to(p.x as f32, p.y as f32);
                    has_current = true;
                } else {
                    builder.line_to(p.x as f32, p.y as f32);
                }
            }
            PathSegment::QuadTo(c, end) => {
                if !has_current {
                    builder.move_to(c.x as f32, c.y as f32);
                    has_current = true;
                }
                builder.quad_to(c.x as f32, c.y as f32, end.x as f32, end.y as f32);
            }
            PathSegment::CubicTo(c1, c2, end) => {
                if !has_current {
                    builder.move_to(c1.x as f32, c1.y as f32);
                    has_current = true;
                }
                builder.cubic_to(
                    c1.x as f32,
                    c1.y as f32,
                    c2.x as f32,
                    c2.y as f32,
                    end.x as f32,
                    end.y as f32,
                );
            }
            PathSegment::Close => {
                builder.close();
            }
        }
    }

    builder.finish()
}

/// Build a tiny-skia rectangle path from a [`Rect`].
pub fn rect_path(rect: &Rect) -> Option<Path> {
    let r = tiny_skia::Rect::from_xywh(
        rect.x() as f32,
        rect.y() as f32,
        rect.width() as f32,
        rect.height() as f32,
    )?;
    Some(PathBuilder::from_rect(r))
}

/// Convert a [`Transform2D`] into a tiny-skia [`Transform`].
pub fn convert_transform(t: &Transform2D) -> Transform {
    Transform::from_row(
        t.a as f32,
        t.b as f32,
        t.c as f32,
        t.d as f32,
        t.tx as f32,
        t.ty as f32,
    )
}

/// The approximate uniform scale factor of a tiny-skia transform.
///
/// Used to size glyph rasterization so text stays crisp when the scene is
/// drawn at a higher device-pixel scale.
pub fn transform_scale(t: &Transform) -> f32 {
    let sx = (t.sx * t.sx + t.kx * t.kx).sqrt();
    let sy = (t.sy * t.sy + t.ky * t.ky).sqrt();
    sx.max(sy).max(1e-3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_geometry::Point;

    #[test]
    fn converts_rect_path() {
        let mut path = ScenePath::new();
        path.move_to(Point::new(0.0, 0.0))
            .line_to(Point::new(10.0, 0.0))
            .line_to(Point::new(10.0, 10.0))
            .close();
        assert!(convert_path(&path).is_some());
    }

    #[test]
    fn empty_path_is_none() {
        let path = ScenePath::new();
        assert!(convert_path(&path).is_none());
    }

    #[test]
    fn rect_path_builds() {
        assert!(rect_path(&Rect::new(1.0, 2.0, 30.0, 40.0)).is_some());
    }

    #[test]
    fn identity_transform_converts() {
        let t = convert_transform(&Transform2D::IDENTITY);
        assert_eq!(t, Transform::identity());
    }
}
