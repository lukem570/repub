//! Linear and radial gradients.
//!
//! Gradient coordinates are normalized to the unit square (0.0–1.0) of the
//! shape being filled, so a gradient is independent of the shape's size.

use repub_geometry::Point;

use crate::color::Color;

/// A single color stop within a gradient.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientStop {
    /// Position along the gradient, 0.0 to 1.0.
    pub position: f64,
    /// Color at this stop.
    pub color: Color,
}

impl GradientStop {
    /// Create a new gradient stop.
    pub fn new(position: f64, color: Color) -> Self {
        Self { position, color }
    }
}

/// A linear gradient between two normalized points.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearGradient {
    /// Start point (normalized 0–1).
    pub start: Point,
    /// End point (normalized 0–1).
    pub end: Point,
    /// Color stops, expected in ascending position order.
    pub stops: Vec<GradientStop>,
}

impl LinearGradient {
    /// A horizontal gradient from left to right.
    pub fn horizontal(start_color: Color, end_color: Color) -> Self {
        Self {
            start: Point::new(0.0, 0.5),
            end: Point::new(1.0, 0.5),
            stops: vec![
                GradientStop::new(0.0, start_color),
                GradientStop::new(1.0, end_color),
            ],
        }
    }

    /// A vertical gradient from top to bottom.
    pub fn vertical(start_color: Color, end_color: Color) -> Self {
        Self {
            start: Point::new(0.5, 0.0),
            end: Point::new(0.5, 1.0),
            stops: vec![
                GradientStop::new(0.0, start_color),
                GradientStop::new(1.0, end_color),
            ],
        }
    }

    /// Sample the gradient color at normalized parameter `t` along the axis.
    pub fn sample(&self, t: f64) -> Color {
        sample_stops(&self.stops, t)
    }
}

/// A radial gradient from a center point outward.
#[derive(Debug, Clone, PartialEq)]
pub struct RadialGradient {
    /// Center point (normalized 0–1).
    pub center: Point,
    /// Radius (normalized, 1.0 = half the shape size).
    pub radius: f64,
    /// Color stops, expected in ascending position order.
    pub stops: Vec<GradientStop>,
}

impl RadialGradient {
    /// A centered radial gradient from inner to outer color.
    pub fn centered(inner_color: Color, outer_color: Color) -> Self {
        Self {
            center: Point::new(0.5, 0.5),
            radius: 0.5,
            stops: vec![
                GradientStop::new(0.0, inner_color),
                GradientStop::new(1.0, outer_color),
            ],
        }
    }

    /// Sample the gradient color at normalized radial distance `t`.
    pub fn sample(&self, t: f64) -> Color {
        sample_stops(&self.stops, t)
    }
}

/// Interpolate a color from ordered stops at parameter `t` (clamped to [0, 1]).
fn sample_stops(stops: &[GradientStop], t: f64) -> Color {
    match stops {
        [] => Color::TRANSPARENT,
        [only] => only.color,
        _ => {
            let t = t.clamp(0.0, 1.0);
            if t <= stops[0].position {
                return stops[0].color;
            }
            if t >= stops[stops.len() - 1].position {
                return stops[stops.len() - 1].color;
            }
            for pair in stops.windows(2) {
                let (a, b) = (pair[0], pair[1]);
                if t >= a.position && t <= b.position {
                    let span = b.position - a.position;
                    let local = if span.abs() < 1e-12 {
                        0.0
                    } else {
                        (t - a.position) / span
                    };
                    return a.color.lerp(b.color, local);
                }
            }
            stops[stops.len() - 1].color
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_endpoints_and_midpoint() {
        let g = LinearGradient::horizontal(Color::BLACK, Color::WHITE);
        assert_eq!(g.sample(0.0), Color::BLACK);
        assert_eq!(g.sample(1.0), Color::WHITE);
        let mid = g.sample(0.5);
        assert!((mid.r - 0.5).abs() < 1e-9);
    }

    #[test]
    fn sample_clamps_out_of_range() {
        let g = RadialGradient::centered(Color::RED, Color::BLUE);
        assert_eq!(g.sample(-1.0), Color::RED);
        assert_eq!(g.sample(2.0), Color::BLUE);
    }

    #[test]
    fn empty_and_single_stop() {
        assert_eq!(sample_stops(&[], 0.5), Color::TRANSPARENT);
        assert_eq!(sample_stops(&[GradientStop::new(0.3, Color::GREEN)], 0.9), Color::GREEN);
    }
}
