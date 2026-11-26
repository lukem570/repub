//! Lyon-based tessellation of scene paths into triangles.
//!
//! A [`repub_geometry::Path`] is mapped straight into a `lyon` path (its
//! quadratic and cubic segments handed to lyon, which flattens them during
//! tessellation). Fills honor the scene [`FillRule`]; strokes honor width, cap
//! and join. A `color_at` closure supplies the per-vertex color so gradients
//! can be baked in by sampling at the vertex position; see [`crate::paint`].
//!
//! Vertices are emitted in device pixels: each path point is transformed by the
//! primitive's page transform and then by the scene's device scale.

use lyon::geom::point;
use lyon::path::Path as LyonPath;
use lyon::tessellation::{
    BuffersBuilder, FillOptions, FillRule as LyonFillRule, FillTessellator, FillVertex,
    LineCap as LyonLineCap, LineJoin as LyonLineJoin, StrokeOptions, StrokeTessellator,
    StrokeVertex, VertexBuffers,
};

use repub_geometry::{Path, PathSegment, Point, Transform2D};
use repub_scene::{FillRule, LineCap, LineJoin};

use crate::error::{GpuError, GpuResult};
use crate::vertex::ShapeVertex;

/// Tessellated triangles: vertices plus a `u32` index buffer.
#[derive(Default)]
pub struct Tessellation {
    /// The emitted vertices.
    pub vertices: Vec<ShapeVertex>,
    /// Triangle indices into [`Tessellation::vertices`].
    pub indices: Vec<u32>,
}

impl Tessellation {
    /// Whether no geometry was produced.
    pub fn is_empty(&self) -> bool {
        self.vertices.is_empty() || self.indices.is_empty()
    }
}

/// Map a [`Path`] into a lyon path, pre-transforming each point into device
/// pixels with `to_device`.
pub fn to_lyon(path: &Path, to_device: &dyn Fn(Point) -> Point) -> LyonPath {
    let mut builder = LyonPath::builder();
    let mut started = false;

    let p = |pt: Point| {
        let d = to_device(pt);
        point(d.x as f32, d.y as f32)
    };

    for seg in &path.segments {
        match *seg {
            PathSegment::MoveTo(pt) => {
                if started {
                    builder.end(false);
                }
                builder.begin(p(pt));
                started = true;
            }
            PathSegment::LineTo(pt) => {
                if !started {
                    builder.begin(p(pt));
                    started = true;
                } else {
                    builder.line_to(p(pt));
                }
            }
            PathSegment::QuadTo(c, end) => {
                if !started {
                    builder.begin(p(c));
                    started = true;
                }
                builder.quadratic_bezier_to(p(c), p(end));
            }
            PathSegment::CubicTo(c1, c2, end) => {
                if !started {
                    builder.begin(p(c1));
                    started = true;
                }
                builder.cubic_bezier_to(p(c1), p(c2), p(end));
            }
            PathSegment::Close => {
                if started {
                    builder.close();
                    started = false;
                }
            }
        }
    }
    if started {
        builder.end(false);
    }
    builder.build()
}

/// Build a device-space transform closure from a primitive page `transform`
/// and the scene `scale`.
pub fn device_mapper(transform: Transform2D, scale: f64) -> impl Fn(Point) -> Point {
    move |pt: Point| {
        let p = transform.transform_point(pt);
        Point::new(p.x * scale, p.y * scale)
    }
}

/// Tessellate a filled path. `color_at` is called per vertex with the
/// device-space position to produce its color.
pub fn fill(
    path: &LyonPath,
    rule: FillRule,
    color_at: &dyn Fn([f32; 2]) -> [f32; 4],
) -> GpuResult<Tessellation> {
    let mut buffers: VertexBuffers<ShapeVertex, u32> = VertexBuffers::new();
    let mut tess = FillTessellator::new();
    let options = FillOptions::tolerance(0.1).with_fill_rule(match rule {
        FillRule::NonZero => LyonFillRule::NonZero,
        FillRule::EvenOdd => LyonFillRule::EvenOdd,
    });

    tess.tessellate_path(
        path,
        &options,
        &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| {
            let position = v.position().to_array();
            ShapeVertex {
                position,
                color: color_at(position),
            }
        }),
    )
    .map_err(|e| GpuError::Tessellation(format!("fill: {e:?}")))?;

    Ok(Tessellation {
        vertices: buffers.vertices,
        indices: buffers.indices,
    })
}

/// Tessellate a stroked path.
///
/// `width` is the stroke width in device pixels (scaled by the caller).
pub fn stroke(
    path: &LyonPath,
    width: f32,
    cap: LineCap,
    join: LineJoin,
    color_at: &dyn Fn([f32; 2]) -> [f32; 4],
) -> GpuResult<Tessellation> {
    let mut buffers: VertexBuffers<ShapeVertex, u32> = VertexBuffers::new();
    let mut tess = StrokeTessellator::new();
    let options = StrokeOptions::tolerance(0.1)
        .with_line_width(width.max(0.01))
        .with_line_cap(match cap {
            LineCap::Butt => LyonLineCap::Butt,
            LineCap::Round => LyonLineCap::Round,
            LineCap::Square => LyonLineCap::Square,
        })
        .with_line_join(match join {
            LineJoin::Miter => LyonLineJoin::Miter,
            LineJoin::Round => LyonLineJoin::Round,
            LineJoin::Bevel => LyonLineJoin::Bevel,
        });

    tess.tessellate_path(
        path,
        &options,
        &mut BuffersBuilder::new(&mut buffers, |v: StrokeVertex| {
            let position = v.position().to_array();
            ShapeVertex {
                position,
                color: color_at(position),
            }
        }),
    )
    .map_err(|e| GpuError::Tessellation(format!("stroke: {e:?}")))?;

    Ok(Tessellation {
        vertices: buffers.vertices,
        indices: buffers.indices,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_geometry::Rect;

    fn white(_: [f32; 2]) -> [f32; 4] {
        [1.0, 1.0, 1.0, 1.0]
    }

    #[test]
    fn fills_a_rect() {
        let path = Path::from_rect(Rect::new(0.0, 0.0, 10.0, 10.0));
        let map = device_mapper(Transform2D::IDENTITY, 1.0);
        let lyon = to_lyon(&path, &map);
        let geo = fill(&lyon, FillRule::NonZero, &white).unwrap();
        assert!(!geo.is_empty());
        assert!(geo.indices.len() >= 6);
    }

    #[test]
    fn strokes_a_line() {
        let mut path = Path::new();
        path.move_to(Point::new(0.0, 5.0)).line_to(Point::new(10.0, 5.0));
        let map = device_mapper(Transform2D::IDENTITY, 1.0);
        let lyon = to_lyon(&path, &map);
        let geo = stroke(&lyon, 2.0, LineCap::Butt, LineJoin::Miter, &white).unwrap();
        assert!(!geo.is_empty());
    }

    #[test]
    fn device_mapper_applies_scale() {
        let map = device_mapper(Transform2D::translate(1.0, 2.0), 2.0);
        let p = map(Point::new(3.0, 4.0));
        assert_eq!((p.x, p.y), (8.0, 12.0));
    }
}
