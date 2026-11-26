//! Building textured quads for the [`Primitive::Image`](repub_scene::Primitive::Image).
//!
//! A decoded image is drawn as two triangles whose corners are the destination
//! rectangle transformed into device space, with UVs taken from the optional
//! normalized crop. Sources that cannot be resolved locally (URLs,
//! placeholders, decode failures) are drawn as a flat gray placeholder quad
//! through the shape pipeline.

use repub_geometry::{Point, Rect, Transform2D};

use crate::vertex::{ShapeVertex, TexturedVertex};

/// A quad of four [`TexturedVertex`]es plus six indices.
pub struct TexturedQuad {
    /// The four corner vertices (TL, TR, BR, BL order).
    pub vertices: [TexturedVertex; 4],
    /// Triangle indices.
    pub indices: [u32; 6],
}

/// Build a textured quad for an image.
///
/// `dest` is the page-space destination rectangle (before `transform`), `crop`
/// an optional normalized (0–1) source crop, `opacity` the image opacity folded
/// into the vertex tint alpha, `transform` the primitive's page transform and
/// `scale` the scene device scale.
pub fn image_quad(
    dest: &Rect,
    crop: Option<&Rect>,
    opacity: f64,
    transform: Transform2D,
    scale: f64,
) -> TexturedQuad {
    let (u0, v0, u1, v1) = match crop {
        Some(c) => (
            c.x() as f32,
            c.y() as f32,
            c.right() as f32,
            c.bottom() as f32,
        ),
        None => (0.0, 0.0, 1.0, 1.0),
    };
    let tint = [1.0, 1.0, 1.0, opacity.clamp(0.0, 1.0) as f32];

    let tl = to_device(Point::new(dest.x(), dest.y()), transform, scale);
    let tr = to_device(Point::new(dest.right(), dest.y()), transform, scale);
    let br = to_device(Point::new(dest.right(), dest.bottom()), transform, scale);
    let bl = to_device(Point::new(dest.x(), dest.bottom()), transform, scale);

    TexturedQuad {
        vertices: [
            TexturedVertex::new(tl.0, tl.1, u0, v0, tint),
            TexturedVertex::new(tr.0, tr.1, u1, v0, tint),
            TexturedVertex::new(br.0, br.1, u1, v1, tint),
            TexturedVertex::new(bl.0, bl.1, u0, v1, tint),
        ],
        indices: [0, 1, 2, 0, 2, 3],
    }
}

/// Build a gray placeholder quad (shape vertices) for an unresolved image.
pub fn placeholder_quad(
    dest: &Rect,
    transform: Transform2D,
    scale: f64,
) -> ([ShapeVertex; 4], [u32; 6]) {
    let color = [0.86, 0.86, 0.86, 1.0];
    let tl = to_device(Point::new(dest.x(), dest.y()), transform, scale);
    let tr = to_device(Point::new(dest.right(), dest.y()), transform, scale);
    let br = to_device(Point::new(dest.right(), dest.bottom()), transform, scale);
    let bl = to_device(Point::new(dest.x(), dest.bottom()), transform, scale);
    (
        [
            ShapeVertex::new(tl.0, tl.1, color),
            ShapeVertex::new(tr.0, tr.1, color),
            ShapeVertex::new(br.0, br.1, color),
            ShapeVertex::new(bl.0, bl.1, color),
        ],
        [0, 1, 2, 0, 2, 3],
    )
}

fn to_device(p: Point, transform: Transform2D, scale: f64) -> (f32, f32) {
    let t = transform.transform_point(p);
    ((t.x * scale) as f32, (t.y * scale) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quad_covers_dest() {
        let q = image_quad(
            &Rect::new(10.0, 20.0, 30.0, 40.0),
            None,
            1.0,
            Transform2D::IDENTITY,
            1.0,
        );
        assert_eq!(q.vertices[0].position, [10.0, 20.0]);
        assert_eq!(q.vertices[2].position, [40.0, 60.0]);
        assert_eq!(q.vertices[0].tex_coords, [0.0, 0.0]);
        assert_eq!(q.vertices[2].tex_coords, [1.0, 1.0]);
    }

    #[test]
    fn scale_applies() {
        let q = image_quad(
            &Rect::new(0.0, 0.0, 10.0, 10.0),
            None,
            0.5,
            Transform2D::IDENTITY,
            2.0,
        );
        assert_eq!(q.vertices[2].position, [20.0, 20.0]);
        assert_eq!(q.vertices[0].color[3], 0.5);
    }
}
