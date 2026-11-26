//! Gradient fills for the canvas.
//!
//! egui only fills with flat colours, so a gradient-filled shape is drawn by
//! letting egui tessellate the flat fill (which keeps its anti-aliased edges),
//! splitting the triangles until they are a few pixels across, and colouring
//! every vertex from the gradient. Gradient coordinates are the shape's own
//! unrotated frame, normalised to 0–1, so gradients rotate with the shape.

use egui::{Color32, Mesh, Pos2, Shape};
use repub_geometry::Rect;
use repub_model::styles::{Fill, GradientStop};
use repub_color::Color;

use crate::canvas::DocToScreen;

/// Longest triangle edge after subdivision, in screen points. Colour is
/// interpolated linearly across each triangle, so this bounds the error for
/// radial and multi-stop gradients.
const MAX_EDGE: f32 = 6.0;

/// Whether `fill` is drawn by this module.
pub fn is_gradient(fill: &Fill) -> bool {
    matches!(fill, Fill::LinearGradient(_) | Fill::RadialGradient(_))
}

/// Colour of `fill` at `(u, v)` in the 0–1 frame (straight alpha).
pub fn color_at(fill: &Fill, u: f64, v: f64) -> Color {
    match fill {
        Fill::LinearGradient(g) => {
            let (dx, dy) = (g.end.x - g.start.x, g.end.y - g.start.y);
            let len_sq = dx * dx + dy * dy;
            let t = if len_sq > 0.0 { ((u - g.start.x) * dx + (v - g.start.y) * dy) / len_sq } else { 0.0 };
            stop_color(&g.stops, t)
        }
        Fill::RadialGradient(g) => {
            // Radius 1.0 is half the shape's size
            let r = (g.radius * 0.5).max(1e-6);
            let (du, dv) = ((u - g.center.x) / r, (v - g.center.y) / r);
            stop_color(&g.stops, (du * du + dv * dv).sqrt())
        }
        other => other.primary_color(),
    }
}

/// Colour at position `t` along the stops (clamped to the end stops).
fn stop_color(stops: &[GradientStop], t: f64) -> Color {
    let Some(first) = stops.first() else { return Color::WHITE };
    if t <= first.position {
        return first.color;
    }
    for pair in stops.windows(2) {
        let (a, b) = (&pair[0], &pair[1]);
        if t <= b.position {
            let span = (b.position - a.position).max(1e-9);
            let k = ((t - a.position) / span).clamp(0.0, 1.0);
            let mix = |x: f64, y: f64| x + (y - x) * k;
            return Color::new(mix(a.color.r, b.color.r), mix(a.color.g, b.color.g), mix(a.color.b, b.color.b), mix(a.color.a, b.color.a));
        }
    }
    stops.last().map_or(first.color, |s| s.color)
}

/// Fill `outline` shapes (screen space, any flat colour) with `fill`.
/// `frame` is the object's bounds and `rotation` its rotation in degrees;
/// `alpha` fades the result (thumbnails of dragged pages).
pub fn fill_shapes(outline: Vec<Shape>, fill: &Fill, frame: Rect, rotation: f64, map: DocToScreen, alpha: f32) -> Shape {
    // Tessellate in screen points with a marker colour: white, so each vertex's
    // alpha is its anti-aliasing coverage
    let mut tessellator = egui::epaint::Tessellator::new(1.0, Default::default(), [1, 1], Vec::new());
    let mut flat = Mesh::default();
    for shape in outline {
        tessellator.tessellate_shape(recolor(shape), &mut flat);
    }

    // Screen point -> 0–1 frame coordinates (undoing the rotation)
    let centre = map.point(frame.origin.x + frame.size.width / 2.0, frame.origin.y + frame.size.height / 2.0);
    let (sin, cos) = (-rotation as f32).to_radians().sin_cos();
    let (w, h) = ((frame.size.width as f32 * map.scale).max(1e-3), (frame.size.height as f32 * map.scale).max(1e-3));
    let to_uv = |p: Pos2| -> (f64, f64) {
        let (dx, dy) = (p.x - centre.x, p.y - centre.y);
        let (x, y) = (dx * cos - dy * sin, dx * sin + dy * cos);
        ((x / w + 0.5) as f64, (y / h + 0.5) as f64)
    };

    let mut out = Mesh::default();
    for tri in flat.indices.chunks_exact(3) {
        let [a, b, c] = [tri[0], tri[1], tri[2]].map(|i| {
            let v = flat.vertices[i as usize];
            (v.pos, v.color.a() as f32 / 255.0)
        });
        subdivide(&mut out, a, b, c, 0, &|p, coverage| {
            let (u, v) = to_uv(p);
            let col = color_at(fill, u, v);
            Color32::from_rgba_unmultiplied(
                (col.r * 255.0).round() as u8,
                (col.g * 255.0).round() as u8,
                (col.b * 255.0).round() as u8,
                (col.a as f32 * coverage * alpha * 255.0).round() as u8,
            )
        });
    }
    Shape::mesh(out)
}

/// Make a fill shape opaque white so tessellated alpha is pure coverage.
pub(crate) fn recolor(shape: Shape) -> Shape {
    match shape {
        Shape::Rect(mut r) => {
            r.fill = Color32::WHITE;
            Shape::Rect(r)
        }
        Shape::Path(mut p) => {
            p.fill = Color32::WHITE;
            Shape::Path(p)
        }
        Shape::Mesh(mut m) => {
            for v in &mut m.vertices {
                v.color = Color32::from_white_alpha(v.color.a());
            }
            Shape::Mesh(m)
        }
        other => other,
    }
}

type Vert = (Pos2, f32);

fn subdivide(out: &mut Mesh, a: Vert, b: Vert, c: Vert, depth: u32, color: &dyn Fn(Pos2, f32) -> Color32) {
    let edges = [(a.0 - b.0).length(), (b.0 - c.0).length(), (c.0 - a.0).length()];
    let longest = edges.iter().copied().fold(0.0, f32::max);
    if longest > MAX_EDGE && depth < 10 {
        // Split the longest edge at its midpoint
        let mid = |p: Vert, q: Vert| -> Vert { (Pos2::new((p.0.x + q.0.x) / 2.0, (p.0.y + q.0.y) / 2.0), (p.1 + q.1) / 2.0) };
        if edges[0] == longest {
            let m = mid(a, b);
            subdivide(out, a, m, c, depth + 1, color);
            subdivide(out, m, b, c, depth + 1, color);
        } else if edges[1] == longest {
            let m = mid(b, c);
            subdivide(out, a, b, m, depth + 1, color);
            subdivide(out, a, m, c, depth + 1, color);
        } else {
            let m = mid(c, a);
            subdivide(out, a, b, m, depth + 1, color);
            subdivide(out, m, b, c, depth + 1, color);
        }
        return;
    }
    let base = out.vertices.len() as u32;
    for (p, coverage) in [a, b, c] {
        out.colored_vertex(p, color(p, coverage));
    }
    out.add_triangle(base, base + 1, base + 2);
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_geometry::Point;
    use repub_model::styles::{LinearGradient, RadialGradient};

    fn red() -> Color {
        Color::new(1.0, 0.0, 0.0, 1.0)
    }
    fn blue() -> Color {
        Color::new(0.0, 0.0, 1.0, 1.0)
    }

    #[test]
    fn linear_gradient_runs_from_start_to_end() {
        let g = Fill::LinearGradient(LinearGradient {
            start: Point::new(0.0, 0.0),
            end: Point::new(1.0, 0.0),
            stops: vec![GradientStop::new(0.0, red()), GradientStop::new(1.0, blue())],
        });
        assert_eq!(color_at(&g, 0.0, 0.5), red());
        assert_eq!(color_at(&g, 1.0, 0.5), blue());
        let mid = color_at(&g, 0.5, 0.9);
        assert!((mid.r - 0.5).abs() < 1e-9 && (mid.b - 0.5).abs() < 1e-9);
        assert_eq!(color_at(&g, -1.0, 0.0), red(), "clamped before the start");
    }

    #[test]
    fn radial_gradient_reaches_the_outer_stop_at_its_radius() {
        let g = Fill::RadialGradient(RadialGradient {
            center: Point::new(0.5, 0.5),
            radius: 1.0,
            stops: vec![GradientStop::new(0.0, red()), GradientStop::new(1.0, blue())],
        });
        assert_eq!(color_at(&g, 0.5, 0.5), red());
        assert_eq!(color_at(&g, 1.0, 0.5), blue(), "edge of the shape");
        assert_eq!(color_at(&g, 0.0, 0.0), blue(), "corners clamp to the last stop");
    }

    #[test]
    fn filled_mesh_is_fine_enough_and_coloured_across() {
        let g = Fill::LinearGradient(LinearGradient {
            start: Point::new(0.0, 0.0),
            end: Point::new(1.0, 0.0),
            stops: vec![GradientStop::new(0.0, red()), GradientStop::new(1.0, blue())],
        });
        let map = DocToScreen { origin: Pos2::ZERO, scale: 1.0 };
        let frame = Rect::new(0.0, 0.0, 100.0, 40.0);
        let rect = Shape::rect_filled(map.rect(&frame), 0.0, Color32::GRAY);
        let Shape::Mesh(mesh) = fill_shapes(vec![rect], &g, frame, 0.0, map, 1.0) else { panic!("mesh") };
        assert!(mesh.indices.len() / 3 > 100, "subdivided");
        let left = mesh.vertices.iter().filter(|v| v.pos.x < 1.0 && v.color.a() == 255).map(|v| v.color).next().unwrap();
        let right = mesh.vertices.iter().filter(|v| v.pos.x > 99.0 && v.color.a() == 255).map(|v| v.color).next().unwrap();
        assert!(left.r() > 240 && left.b() < 15, "left is red: {left:?}");
        assert!(right.b() > 240 && right.r() < 15, "right is blue: {right:?}");
    }
}
