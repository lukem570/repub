//! # Repub Escher
//!
//! The Office legacy ("Escher", MS-ODRAW) preset-shape geometry engine.
//!
//! Office stores AutoShapes not as explicit outlines but as parametric
//! templates in a fixed 21600 x 21600 space: *guides* (small formulas over the
//! shape's *adjust values*), *vertices* (constants or guide references), and
//! *segment codes* joining them. This crate holds the 202-template catalog and
//! the engine that evaluates a template for a given size and adjust values,
//! producing a [`Path`], along with its text rectangle, connection sites, and
//! adjust-handle positions.
//!
//! The model ([`repub_model::PresetGeometry`]) only stores *which* preset a
//! shape uses and its adjust values. This crate provides [`PresetExt`], an
//! extension trait on that type that computes geometry on demand.

#![warn(missing_docs)]
#![warn(clippy::all)]

mod engine;
mod presets;

pub use engine::{preset, presets, PresetDef, UNITS};

use repub_geometry::{Point, Rect};
use repub_model::geometry::{Path, PresetGeometry};

/// Geometry operations for a [`PresetGeometry`], backed by the Escher engine.
///
/// All results are in page coordinates for the given `bounds`, before the
/// shape's own flips and rotation are applied.
pub trait PresetExt {
    /// The preset's template, or `None` for an unknown shape type.
    fn def(&self) -> Option<&'static PresetDef>;

    /// The effective adjust value at `index`: the override if set, else the
    /// preset's default, else 0.
    fn adjust_value(&self, index: usize) -> i32;

    /// The shape's outline for `bounds`, or `None` for an unknown type.
    fn path(&self, bounds: Rect) -> Option<Path>;

    /// The text rectangle for `bounds` (the whole box if the preset has none).
    fn text_rect(&self, bounds: Rect) -> Rect;

    /// The connection sites on the page for `bounds`.
    fn connection_sites(&self, bounds: Rect) -> Vec<Point>;

    /// The positions of the adjust handles on the page for `bounds`.
    fn handles(&self, bounds: Rect) -> Vec<Point>;

    /// Drag adjust handle `index` toward page point `to` within `bounds`,
    /// updating the adjust values it controls (within their ranges). Returns
    /// whether anything changed.
    fn drag_handle(&mut self, index: usize, bounds: Rect, to: Point) -> bool;
}

impl PresetExt for PresetGeometry {
    fn def(&self) -> Option<&'static PresetDef> {
        preset(self.spt)
    }

    fn adjust_value(&self, index: usize) -> i32 {
        engine::adjust_value(self.spt, &self.adjust, index)
    }

    fn path(&self, bounds: Rect) -> Option<Path> {
        engine::build_path(self.spt, &self.adjust, bounds)
    }

    fn text_rect(&self, bounds: Rect) -> Rect {
        engine::text_rect(self.spt, &self.adjust, bounds)
    }

    fn connection_sites(&self, bounds: Rect) -> Vec<Point> {
        engine::connection_sites(self.spt, &self.adjust, bounds)
    }

    fn handles(&self, bounds: Rect) -> Vec<Point> {
        engine::handles(self.spt, &self.adjust, bounds)
    }

    fn drag_handle(&mut self, index: usize, bounds: Rect, to: Point) -> bool {
        let changes = engine::drag_handle(self.spt, &self.adjust, index, bounds, to);
        for (n, v) in &changes {
            self.set_adjust(*n, *v);
        }
        !changes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn points(path: &Path) -> Vec<(f64, f64)> {
        use repub_model::geometry::PathCommand;
        path.commands
            .iter()
            .filter_map(|c| match c {
                PathCommand::MoveTo(p) | PathCommand::LineTo(p) => Some((p.x.round(), p.y.round())),
                PathCommand::CubicTo { end, .. } => Some((end.x.round(), end.y.round())),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn every_template_builds() {
        assert_eq!(presets().len(), 202);
        for def in presets() {
            let path = PresetGeometry::new(def.spt)
                .path(Rect::new(0.0, 0.0, 216.0, 216.0))
                .unwrap();
            assert!(!path.commands.is_empty(), "{} ({})", def.name, def.spt);
            for c in &path.commands {
                use repub_model::geometry::PathCommand;
                if let PathCommand::LineTo(p) | PathCommand::MoveTo(p) = c {
                    assert!(p.x.is_finite() && p.y.is_finite(), "{}", def.name);
                }
            }
        }
    }

    #[test]
    fn simple_shapes_come_out_right() {
        let b = Rect::new(0.0, 0.0, 216.0, 108.0);
        assert_eq!(preset(1).unwrap().name, "Rectangle");
        assert_eq!(
            points(&PresetGeometry::new(1).path(b).unwrap()),
            vec![(0.0, 0.0), (0.0, 108.0), (216.0, 108.0), (216.0, 0.0)]
        );
        assert_eq!(preset(4).unwrap().name, "Diamond");
        assert_eq!(
            points(&PresetGeometry::new(4).path(b).unwrap()),
            vec![(108.0, 0.0), (0.0, 54.0), (108.0, 108.0), (216.0, 54.0)]
        );
        assert_eq!(
            points(&PresetGeometry::new(3).path(b).unwrap()),
            vec![
                (108.0, 0.0),
                (0.0, 54.0),
                (108.0, 108.0),
                (216.0, 54.0),
                (108.0, 0.0)
            ]
        );
    }

    #[test]
    fn adjust_values_move_the_outline() {
        let b = Rect::new(0.0, 0.0, 216.0, 216.0);
        assert_eq!(
            points(&PresetGeometry::new(5).path(b).unwrap())[0],
            (108.0, 0.0)
        );
        let moved = PresetGeometry {
            spt: 5,
            adjust: vec![Some(5400)],
        };
        assert_eq!(points(&moved.path(b).unwrap())[0], (54.0, 0.0));
    }

    #[test]
    fn handles_sit_on_their_adjust_values_and_drag_them() {
        let b = Rect::new(0.0, 0.0, 216.0, 216.0);
        let mut tri = PresetGeometry::new(5);
        let h = tri.handles(b);
        assert_eq!(h.len(), 1);
        assert!(
            (h[0].x - 108.0).abs() < 0.01 && h[0].y.abs() < 0.01,
            "{h:?}"
        );
        assert!(
            tri.drag_handle(0, b, Point::new(54.0, 80.0)),
            "x moves the apex; y is fixed"
        );
        assert_eq!(tri.adjust_value(0), 5400);
        tri.drag_handle(0, b, Point::new(500.0, 0.0));
        assert_eq!(tri.adjust_value(0), 21600);
        let mut arrow = PresetGeometry::new(13);
        arrow.drag_handle(0, b, Point::new(108.0, 20.0));
        assert_eq!(
            (arrow.adjust_value(0), arrow.adjust_value(1)),
            (10800, 2000)
        );
    }

    #[test]
    fn polar_handles_set_angles() {
        let b = Rect::new(0.0, 0.0, 216.0, 216.0);
        let mut arc = PresetGeometry::new(95);
        assert_eq!(arc.handles(b).len(), 1);
        arc.drag_handle(0, b, Point::new(108.0, 10.0));
        assert_eq!(arc.adjust_value(0), 90 * 65536);
    }

    #[test]
    fn donut_has_a_hole() {
        let p = PresetGeometry::new(23)
            .path(Rect::new(0.0, 0.0, 100.0, 100.0))
            .unwrap();
        assert!(!p.even_odd, "nonzero, as Office fills");
        assert_eq!(p.split_subpaths().len(), 2, "outer and inner circles");
    }

    #[test]
    fn text_rect_is_inside() {
        let r = PresetGeometry::new(3).text_rect(Rect::new(0.0, 0.0, 216.0, 216.0));
        assert!(r.origin.x > 20.0 && r.size.width < 180.0, "{r:?}");
    }
}
