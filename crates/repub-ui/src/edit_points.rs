//! Editing a shape's path points on the canvas.
//!
//! Anchor points are drawn as small white squares and the selected one shows
//! its curve handles as circles. Dragging a point or handle reshapes the
//! object; double-clicking an edge adds a point; clicking away or pressing
//! Escape leaves point-editing mode.

use egui::{Pos2, Rect, Stroke};
use repub_geometry::Point;
use repub_model::geometry::{Path, ShapeGeometry};
use repub_model::path_edit::ControlRef;
use repub_model::shape::{Shape, ShapeKind};
use repub_model::ObjectId;

use crate::state::AppState;
use crate::theme::{colors, metrics};

/// Hit radius in screen points.
const HIT_RADIUS: f32 = 6.0;

/// What the Edit Shape menu does to the selected point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointOp {
    /// Remove the point
    Delete,
    /// Make the segment arriving at the point a straight line
    Straight,
    /// Make the segment arriving at the point a curve
    Curved,
}

fn doc_to_screen(page_rect: Rect, zoom: f32, p: Point) -> Pos2 {
    Pos2::new(page_rect.min.x + p.x as f32 * zoom, page_rect.min.y + p.y as f32 * zoom)
}

fn screen_to_doc(page_rect: Rect, zoom: f32, p: Pos2) -> Point {
    Point::new(((p.x - page_rect.min.x) / zoom) as f64, ((p.y - page_rect.min.y) / zoom) as f64)
}

/// A point of the shape's own path on the page: flipped, then turned about the center.
fn to_page(shape: &Shape, mut p: Point) -> Point {
    let c = shape.bounds().center();
    if shape.common.flip_h {
        p.x = 2.0 * c.x - p.x;
    }
    if shape.common.flip_v {
        p.y = 2.0 * c.y - p.y;
    }
    let (sin, cos) = shape.common.rotation.to_radians().sin_cos();
    let (dx, dy) = (p.x - c.x, p.y - c.y);
    Point::new(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos)
}

/// The inverse of [`to_page`].
fn from_page(shape: &Shape, p: Point) -> Point {
    let c = shape.bounds().center();
    let (sin, cos) = (-shape.common.rotation).to_radians().sin_cos();
    let (dx, dy) = (p.x - c.x, p.y - c.y);
    let mut q = Point::new(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos);
    if shape.common.flip_h {
        q.x = 2.0 * c.x - q.x;
    }
    if shape.common.flip_v {
        q.y = 2.0 * c.y - q.y;
    }
    q
}

/// The shape being edited and its path (the outline of a primitive, until it is changed).
fn session(state: &AppState) -> Option<(ObjectId, &Shape, Path)> {
    let id = state.edit_points.as_ref()?.shape_id;
    let shape = state.current_page()?.get_shape(id)?;
    let ShapeKind::AutoShape(auto) = &shape.kind else { return None };
    Some((id, shape, auto.geometry.to_editable_path(shape.bounds())))
}

/// The path of the shape being edited, made editable, for `f` to change.
fn edit_path(state: &mut AppState, id: ObjectId, f: impl FnOnce(&mut Path) -> bool) -> bool {
    let Some(shape) = state.current_page_mut().and_then(|p| p.get_shape_mut(id)) else { return false };
    let bounds = shape.bounds();
    let ShapeKind::AutoShape(auto) = &mut shape.kind else { return false };
    auto.convert_to_path(bounds);
    let ShapeGeometry::Path(path) = &mut auto.geometry else { return false };
    let changed = f(path);
    if changed {
        state.mark_dirty();
    }
    changed
}

/// Draw the anchor points of the shape being edited, and the handles of the selected one.
pub fn draw(painter: &egui::Painter, page_rect: Rect, state: &AppState) {
    let Some((_, shape, path)) = session(state) else { return };
    let zoom = state.zoom;
    let r = metrics::HANDLE_SIZE * 0.4;
    let selected = state.edit_points.as_ref().and_then(|s| s.selected);
    let screen = |p: Point| doc_to_screen(page_rect, zoom, to_page(shape, p));
    let is_path = matches!(&shape.kind, ShapeKind::AutoShape(a) if a.geometry.is_path());
    if let (Some(index), true) = (selected, is_path) {
        for (_, handle, anchor) in path.control_handles(index) {
            let (h, a) = (screen(handle), screen(anchor));
            painter.line_segment([a, h], Stroke::new(1.0, colors::accent()));
            painter.circle(h, r * 0.7, egui::Color32::WHITE, Stroke::new(1.0, colors::accent()));
        }
    }
    for (i, p) in path.anchor_points().iter().enumerate() {
        let c = screen(*p);
        let fill = if selected == Some(i) { colors::accent() } else { egui::Color32::WHITE };
        painter.rect_filled(Rect::from_center_size(c, egui::Vec2::splat(r)), egui::Rounding::ZERO, fill);
        painter.rect_stroke(Rect::from_center_size(c, egui::Vec2::splat(r)), egui::Rounding::ZERO, Stroke::new(1.0, colors::accent()));
    }
}

/// Handle point-editing input. Returns true when the input was consumed.
pub fn handle_input(response: &egui::Response, pointer: Pos2, page_rect: Rect, state: &mut AppState) -> bool {
    let Some((shape_id, shape, path)) = session(state) else { return false };
    let zoom = state.zoom;
    let is_path = matches!(&shape.kind, ShapeKind::AutoShape(a) if a.geometry.is_path());
    let anchors: Vec<Point> = path.anchor_points().iter().map(|p| to_page(shape, *p)).collect();
    let selected = state.edit_points.as_ref().and_then(|s| s.selected);
    let handles: Vec<(ControlRef, Point)> = match (selected, is_path) {
        (Some(i), true) => path.control_handles(i).into_iter().map(|(r, p, _)| (r, to_page(shape, p))).collect(),
        _ => Vec::new(),
    };
    let doc = screen_to_doc(page_rect, zoom, pointer);
    let local = from_page(shape, doc);

    if response.double_clicked() {
        let tolerance = f64::from(HIT_RADIUS / zoom);
        if nearest(&anchors, page_rect, zoom, pointer).is_none() {
            if let Some((command, t, _)) = path.nearest_segment(local, tolerance) {
                state.record_undo("Add Shape Point");
                let mut added = None;
                edit_path(state, shape_id, |p| {
                    added = p.split_segment(command, t);
                    added.is_some()
                });
                if let Some(session) = state.edit_points.as_mut() {
                    session.selected = added;
                }
            }
        }
        return true;
    }

    if response.drag_started() {
        let control = handles.iter().find(|(_, p)| doc_to_screen(page_rect, zoom, *p).distance(pointer) <= HIT_RADIUS).map(|(r, _)| *r);
        if let Some(control) = control {
            state.record_undo("Edit Shape Curve");
            if let Some(session) = state.edit_points.as_mut() {
                session.dragging_control = Some(control);
            }
            return true;
        }
        if let Some(idx) = nearest(&anchors, page_rect, zoom, pointer) {
            state.record_undo("Edit Shape Points");
            if let Some(session) = state.edit_points.as_mut() {
                session.selected = Some(idx);
                session.dragging = true;
            }
            return true;
        }
    }

    let (dragging, dragging_control) = state.edit_points.as_ref().map_or((false, None), |s| (s.dragging, s.dragging_control));
    if response.dragged() && (dragging || dragging_control.is_some()) {
        if let Some(control) = dragging_control {
            edit_path(state, shape_id, |p| p.move_control(control, local));
        } else if let Some(idx) = selected {
            edit_path(state, shape_id, |p| p.move_anchor_point(idx, local));
        }
        return true;
    }

    if response.drag_stopped() {
        if let Some(session) = state.edit_points.as_mut() {
            session.dragging = false;
            session.dragging_control = None;
        }
        return dragging || dragging_control.is_some();
    }

    if response.clicked() {
        if let Some(idx) = nearest(&anchors, page_rect, zoom, pointer) {
            if let Some(session) = state.edit_points.as_mut() {
                session.selected = Some(idx);
            }
        } else {
            // Click away from any point leaves point-editing mode.
            state.edit_points = None;
        }
        return true;
    }

    false
}

/// Whether `op` can be done now: a point is selected and the path allows it.
pub fn can_apply(state: &AppState, op: PointOp) -> bool {
    let Some((_, _, path)) = session(state) else { return false };
    let Some(index) = state.edit_points.as_ref().and_then(|s| s.selected) else { return false };
    let arrives_by_line = path.anchor_command(index).is_some_and(|i| matches!(path.commands[i], repub_model::geometry::PathCommand::LineTo(_)));
    match op {
        PointOp::Delete => path.anchor_points().len() > 2,
        PointOp::Straight => path.segment_is_curved(index),
        PointOp::Curved => arrives_by_line,
    }
}

/// Do `op` to the selected point. Returns whether anything changed.
pub fn apply_op(state: &mut AppState, op: PointOp) -> bool {
    let Some((id, _, _)) = session(state) else { return false };
    let Some(index) = state.edit_points.as_ref().and_then(|s| s.selected) else { return false };
    let label = match op {
        PointOp::Delete => "Delete Shape Point",
        PointOp::Straight => "Straighten Segment",
        PointOp::Curved => "Curve Segment",
    };
    state.record_undo(label);
    let done = edit_path(state, id, |p| match op {
        PointOp::Delete => p.remove_anchor_point(index),
        PointOp::Straight => p.set_segment_curved(index, false),
        PointOp::Curved => p.set_segment_curved(index, true),
    });
    if done && op == PointOp::Delete {
        if let Some(session) = state.edit_points.as_mut() {
            session.selected = None;
        }
    }
    done
}

fn nearest(points: &[Point], page_rect: Rect, zoom: f32, pointer: Pos2) -> Option<usize> {
    let mut best: Option<(usize, f32)> = None;
    for (i, p) in points.iter().enumerate() {
        let d = doc_to_screen(page_rect, zoom, *p).distance(pointer);
        if d <= HIT_RADIUS && best.map_or(true, |(_, bd)| d < bd) {
            best = Some((i, d));
        }
    }
    best.map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::EditPointsState;
    use repub_geometry::Rect as DocRect;
    use repub_model::{Document, PageOrientation, PageSize};

    fn editing_a_rectangle() -> (AppState, ObjectId) {
        let mut state = AppState::new();
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        let id = doc.pages_mut()[0].add_shape(Shape::rectangle(DocRect::new(100.0, 100.0, 80.0, 40.0)));
        state.document = Some(doc);
        state.edit_points = Some(EditPointsState { shape_id: id, selected: Some(1), dragging: false, dragging_control: None });
        (state, id)
    }

    fn path_of(state: &AppState, id: ObjectId) -> Path {
        match &state.current_page().unwrap().get_shape(id).unwrap().kind {
            ShapeKind::AutoShape(a) => a.geometry.to_editable_path(DocRect::default()),
            _ => unreachable!(),
        }
    }

    #[test]
    fn curve_straighten_and_delete_work_on_the_selected_point() {
        let (mut state, id) = editing_a_rectangle();
        assert!(can_apply(&state, PointOp::Curved));
        assert!(!can_apply(&state, PointOp::Straight));
        assert!(apply_op(&mut state, PointOp::Curved));
        assert!(path_of(&state, id).segment_is_curved(1));
        assert!(can_apply(&state, PointOp::Straight));
        assert!(apply_op(&mut state, PointOp::Straight));
        assert!(!path_of(&state, id).segment_is_curved(1));
        assert!(apply_op(&mut state, PointOp::Delete));
        assert_eq!(path_of(&state, id).anchor_points().len(), 3);
        assert_eq!(state.edit_points.as_ref().unwrap().selected, None);
        // Each change is one undo step
        assert!(state.undo());
        assert_eq!(path_of(&state, id).anchor_points().len(), 4);
    }

    #[test]
    fn points_follow_the_shape_when_it_is_turned() {
        let (mut state, id) = editing_a_rectangle();
        state.current_page_mut().unwrap().get_shape_mut(id).unwrap().common.rotation = 90.0;
        let (_, shape, path) = session(&state).unwrap();
        // The top-left corner of an 80 x 40 box at (100,100) turned 90 degrees about (140, 120)
        let p = to_page(shape, path.anchor_points()[0]);
        assert!((p.x - 160.0).abs() < 1e-6 && (p.y - 80.0).abs() < 1e-6, "{p:?}");
        let back = from_page(shape, p);
        assert!((back.x - 100.0).abs() < 1e-6 && (back.y - 100.0).abs() < 1e-6);
    }
}
