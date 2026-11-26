//! Conversion from a scene [`Paint`] to a tiny-skia paint/shader.
//!
//! Solid colors and linear/radial gradients are realized faithfully using the
//! paint's resolved on-page `bounds`. Pattern fills are **approximated** by a
//! solid fill of the pattern's foreground color: a correctly tiled pattern
//! bitmap is out of scope for this pass. See [`build_paint`] for details.

use repub_color::{Color, GradientStop as ColorStop, LinearGradient, RadialGradient};
use repub_geometry::{Point, Rect};
use repub_scene::Paint as ScenePaint;
use tiny_skia::{
    GradientStop, Paint, Point as SkPoint, Shader, SpreadMode, Transform,
};

/// Convert a scene [`Color`] into a tiny-skia color.
pub fn convert_color(color: &Color) -> tiny_skia::Color {
    let (r, g, b, a) = color.to_rgba8();
    tiny_skia::Color::from_rgba8(r, g, b, a)
}

/// Build a tiny-skia [`Paint`] for a scene [`Paint`].
///
/// - [`ScenePaint::Solid`] maps directly to a solid shader.
/// - [`ScenePaint::LinearGradient`] / [`ScenePaint::RadialGradient`] map the
///   normalized (0–1) gradient coordinates onto the paint's `bounds` and build
///   the matching tiny-skia gradient shader. If the gradient cannot be built
///   (e.g. degenerate endpoints or no stops), the first stop color is used as a
///   solid fallback.
/// - [`ScenePaint::Pattern`] is **approximated** with the pattern's foreground
///   color. Tiled pattern bitmaps are not realized in this pass.
pub fn build_paint<'a>(paint: &ScenePaint, anti_alias: bool, opacity: f32) -> Paint<'a> {
    let shader = build_shader(paint).unwrap_or_else(|| {
        Shader::SolidColor(with_opacity(convert_color(&paint.primary_color()), opacity))
    });

    let shader = match shader {
        Shader::SolidColor(c) => Shader::SolidColor(with_opacity(c, opacity)),
        other => other,
    };

    Paint {
        shader,
        anti_alias,
        ..Default::default()
    }
}

fn with_opacity(mut c: tiny_skia::Color, opacity: f32) -> tiny_skia::Color {
    if opacity < 1.0 {
        c.set_alpha((c.alpha() * opacity).clamp(0.0, 1.0));
    }
    c
}

fn build_shader<'a>(paint: &ScenePaint) -> Option<Shader<'a>> {
    match paint {
        ScenePaint::Solid(color) => Some(Shader::SolidColor(convert_color(color))),
        ScenePaint::LinearGradient { gradient, bounds } => linear_shader(gradient, bounds),
        ScenePaint::RadialGradient { gradient, bounds } => radial_shader(gradient, bounds),
        // Pattern tiling is not realized this pass: approximate with foreground.
        ScenePaint::Pattern { pattern, .. } => {
            Some(Shader::SolidColor(convert_color(&pattern.foreground)))
        }
    }
}

fn map_point(norm: Point, bounds: &Rect) -> SkPoint {
    SkPoint::from_xy(
        (bounds.x() + norm.x * bounds.width()) as f32,
        (bounds.y() + norm.y * bounds.height()) as f32,
    )
}

fn convert_stops(stops: &[ColorStop]) -> Vec<GradientStop> {
    stops
        .iter()
        .map(|s| GradientStop::new(s.position.clamp(0.0, 1.0) as f32, convert_color(&s.color)))
        .collect()
}

fn linear_shader<'a>(gradient: &LinearGradient, bounds: &Rect) -> Option<Shader<'a>> {
    if gradient.stops.is_empty() {
        return None;
    }
    let start = map_point(gradient.start, bounds);
    let end = map_point(gradient.end, bounds);
    tiny_skia::LinearGradient::new(
        start,
        end,
        convert_stops(&gradient.stops),
        SpreadMode::Pad,
        Transform::identity(),
    )
}

fn radial_shader<'a>(gradient: &RadialGradient, bounds: &Rect) -> Option<Shader<'a>> {
    if gradient.stops.is_empty() {
        return None;
    }
    let center = map_point(gradient.center, bounds);
    // Radius is normalized so 1.0 == half the shape size; scale by the mean
    // half-extent of the bounds to get an on-page radius.
    let half = ((bounds.width() + bounds.height()) * 0.5 * 0.5) as f32;
    let radius = (gradient.radius as f32 * half).max(1e-3);
    tiny_skia::RadialGradient::new(
        center,
        center,
        radius,
        convert_stops(&gradient.stops),
        SpreadMode::Pad,
        Transform::identity(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_color::Color;

    #[test]
    fn solid_paint_builds() {
        let paint = build_paint(&ScenePaint::Solid(Color::RED), true, 1.0);
        match paint.shader {
            Shader::SolidColor(c) => assert_eq!(c.red(), 1.0),
            _ => panic!("expected solid shader"),
        }
    }

    #[test]
    fn linear_gradient_builds_gradient_shader() {
        let paint = ScenePaint::LinearGradient {
            gradient: LinearGradient::horizontal(Color::BLACK, Color::WHITE),
            bounds: Rect::new(0.0, 0.0, 100.0, 50.0),
        };
        let built = build_paint(&paint, true, 1.0);
        assert!(matches!(built.shader, Shader::LinearGradient(_)));
    }

    #[test]
    fn radial_gradient_builds_gradient_shader() {
        let paint = ScenePaint::RadialGradient {
            gradient: RadialGradient::centered(Color::RED, Color::BLUE),
            bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
        };
        let built = build_paint(&paint, true, 1.0);
        assert!(matches!(built.shader, Shader::RadialGradient(_)));
    }

    #[test]
    fn pattern_falls_back_to_foreground() {
        let mut pattern = repub_color::Pattern::default();
        pattern.foreground = Color::GREEN;
        let paint = ScenePaint::Pattern {
            pattern,
            bounds: Rect::new(0.0, 0.0, 10.0, 10.0),
        };
        let built = build_paint(&paint, true, 1.0);
        match built.shader {
            Shader::SolidColor(c) => assert_eq!(c.green(), 1.0),
            _ => panic!("expected solid fallback"),
        }
    }
}
