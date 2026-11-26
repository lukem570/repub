//! Editing a shape's path points.
//!
//! This holds the framework-free state and math for point editing: which
//! anchor is selected, hit-testing an anchor near a point, and moving an
//! anchor on a shape's path. Drawing the handles and routing pointer input are
//! rendering concerns left to the GUI, which calls these functions.

use repub_geometry::Point;
use repub_model::id::ObjectId;
use repub_model::shape::{Shape, ShapeKind};

/// A shape point-editing session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditPointsState {
    /// Shape being edited.
    pub shape_id: ObjectId,
    /// Index of the selected anchor point.
    pub selected: Option<usize>,
    /// Whether a point is being dragged.
    pub dragging: bool,
}

impl EditPointsState {
    /// Start a point-editing session for `shape_id`.
    pub fn new(shape_id: ObjectId) -> Self {
        Self {
            shape_id,
            selected: None,
            dragging: false,
        }
    }
}

/// The anchor points of `shape`'s editable path, in document points. Only auto
/// shapes have editable anchors; other kinds return an empty list.
pub fn anchor_points(shape: &Shape) -> Vec<Point> {
    match &shape.kind {
        ShapeKind::AutoShape(auto) => auto.anchor_points(shape.bounds()),
        _ => Vec::new(),
    }
}

/// The index of the anchor point within `radius` document points of `point`,
/// nearest first, or `None` when none is close enough.
pub fn anchor_at(shape: &Shape, point: Point, radius: f64) -> Option<usize> {
    let points = anchor_points(shape);
    let mut best: Option<(usize, f64)> = None;
    for (i, p) in points.iter().enumerate() {
        let d = ((p.x - point.x).powi(2) + (p.y - point.y).powi(2)).sqrt();
        if d <= radius && best.map_or(true, |(_, bd)| d < bd) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

/// Move the anchor point at `index` of `shape`'s path to `point`. Converts a
/// primitive auto shape to an editable path first. Returns whether a point was
/// moved.
pub fn move_anchor(shape: &mut Shape, index: usize, point: Point) -> bool {
    let bounds = shape.bounds();
    match &mut shape.kind {
        ShapeKind::AutoShape(auto) => auto.move_anchor_point(bounds, index, point),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_geometry::Rect;

    #[test]
    fn rectangle_has_four_anchors_and_hit_tests() {
        let shape = Shape::rectangle(Rect::new(0.0, 0.0, 100.0, 50.0));
        let points = anchor_points(&shape);
        assert_eq!(points.len(), 4);
        // Near the top-left corner
        assert_eq!(anchor_at(&shape, Point::new(2.0, 2.0), 6.0), Some(0));
        // Far from any corner
        assert_eq!(anchor_at(&shape, Point::new(50.0, 25.0), 6.0), None);
    }

    #[test]
    fn moving_an_anchor_reshapes_the_path() {
        let mut shape = Shape::rectangle(Rect::new(0.0, 0.0, 100.0, 50.0));
        assert!(move_anchor(&mut shape, 0, Point::new(10.0, 10.0)));
        assert_eq!(anchor_points(&shape)[0], Point::new(10.0, 10.0));
    }

    #[test]
    fn non_auto_shapes_have_no_anchors() {
        let tf = Shape::text_frame(Rect::new(0.0, 0.0, 10.0, 10.0));
        assert!(anchor_points(&tf).is_empty());
        assert_eq!(anchor_at(&tf, Point::new(0.0, 0.0), 6.0), None);
    }
}
