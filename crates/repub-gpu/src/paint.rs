//! Turning a scene [`Paint`] into a per-vertex color closure.
//!
//! The shape pipeline carries one RGBA color per vertex, so gradients are
//! realized by **baking** a sampled color into every tessellated vertex rather
//! than evaluating a gradient in a fragment shader. This is exact at vertices
//! and linearly interpolated across triangles, which is faithful for the dense
//! triangle meshes lyon produces.
//!
//! Simplifications in this pass:
//! - **Linear gradients** are sampled by projecting each vertex onto the
//!   gradient axis (start→end in the paint's on-page bounds).
//! - **Radial gradients** are sampled by normalized distance from the center,
//!   using the mean half-extent of the bounds as the radius unit (matching the
//!   CPU raster backend).
//! - **Pattern** fills are approximated by the pattern's foreground color;
//!   true tiling is deferred.

use repub_color::Color;
use repub_geometry::{Point, Rect};
use repub_scene::Paint;

/// A closure mapping a device-space vertex position to its RGBA color.
pub type ColorFn = Box<dyn Fn([f32; 2]) -> [f32; 4]>;

/// RGBA components of a [`Color`] in 0–1, optionally scaled by `opacity`.
pub fn color_array(color: &Color, opacity: f64) -> [f32; 4] {
    [
        color.r as f32,
        color.g as f32,
        color.b as f32,
        (color.a * opacity).clamp(0.0, 1.0) as f32,
    ]
}

/// Build a per-vertex color closure for `paint`.
///
/// `scale` is the scene device scale, used to map the paint's page-space bounds
/// into the device-space coordinates the vertices live in. `opacity` multiplies
/// the resulting alpha.
pub fn color_fn(paint: &Paint, scale: f64, opacity: f64) -> ColorFn {
    match paint {
        Paint::Solid(c) => {
            let rgba = color_array(c, opacity);
            Box::new(move |_| rgba)
        }
        Paint::Pattern { pattern, .. } => {
            let rgba = color_array(&pattern.foreground, opacity);
            Box::new(move |_| rgba)
        }
        Paint::LinearGradient { gradient, bounds } => {
            let gradient = gradient.clone();
            let bounds = device_bounds(bounds, scale);
            let start = map_norm(gradient.start, &bounds);
            let end = map_norm(gradient.end, &bounds);
            let axis = (end.x - start.x, end.y - start.y);
            let len_sq = axis.0 * axis.0 + axis.1 * axis.1;
            Box::new(move |pos| {
                let t = if len_sq < 1e-9 {
                    0.0
                } else {
                    ((pos[0] as f64 - start.x) * axis.0 + (pos[1] as f64 - start.y) * axis.1)
                        / len_sq
                };
                color_array(&gradient.sample(t), opacity)
            })
        }
        Paint::RadialGradient { gradient, bounds } => {
            let gradient = gradient.clone();
            let bounds = device_bounds(bounds, scale);
            let center = map_norm(gradient.center, &bounds);
            let half = (bounds.width() + bounds.height()) * 0.5 * 0.5;
            let radius = (gradient.radius * half).max(1e-3);
            Box::new(move |pos| {
                let dx = pos[0] as f64 - center.x;
                let dy = pos[1] as f64 - center.y;
                let t = (dx * dx + dy * dy).sqrt() / radius;
                color_array(&gradient.sample(t), opacity)
            })
        }
    }
}

fn device_bounds(bounds: &Rect, scale: f64) -> Rect {
    Rect::new(
        bounds.x() * scale,
        bounds.y() * scale,
        bounds.width() * scale,
        bounds.height() * scale,
    )
}

fn map_norm(norm: Point, bounds: &Rect) -> Point {
    Point::new(
        bounds.x() + norm.x * bounds.width(),
        bounds.y() + norm.y * bounds.height(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_color::LinearGradient;

    #[test]
    fn solid_is_constant() {
        let f = color_fn(&Paint::Solid(Color::RED), 1.0, 1.0);
        assert_eq!(f([0.0, 0.0]), [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(f([50.0, 50.0]), [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn opacity_scales_alpha() {
        let f = color_fn(&Paint::Solid(Color::rgb(0.0, 0.0, 0.0)), 1.0, 0.5);
        assert_eq!(f([0.0, 0.0])[3], 0.5);
    }

    #[test]
    fn linear_gradient_endpoints() {
        let paint = Paint::LinearGradient {
            gradient: LinearGradient::horizontal(Color::BLACK, Color::WHITE),
            bounds: Rect::new(0.0, 0.0, 100.0, 10.0),
        };
        let f = color_fn(&paint, 1.0, 1.0);
        // Left edge ~ black, right edge ~ white.
        assert!(f([0.0, 5.0])[0] < 0.1);
        assert!(f([100.0, 5.0])[0] > 0.9);
    }
}
