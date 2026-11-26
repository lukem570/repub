//! Shape layout and positioning.
//!
//! Publisher represents every object on a page as one [`Shape`]. These helpers
//! compute transformed bounds, align and distribute shapes, snap them to a
//! grid or to each other, and position one shape relative to another. They
//! operate on `&Shape` / `&mut [&mut Shape]` and use the shape's common
//! placement (`shape.common`) and its `bounds()` / `set_position_xy` /
//! `set_size` helpers.

use repub_geometry::{Point, Rect, Transform2D};
use repub_model::shape::Shape;

/// Object alignment options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    /// No alignment
    #[default]
    None,
    /// Align left edges
    Left,
    /// Align horizontal centers
    Center,
    /// Align right edges
    Right,
    /// Align top edges
    Top,
    /// Align vertical centers
    Middle,
    /// Align bottom edges
    Bottom,
}

/// Object distribution options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Distribution {
    /// No distribution
    #[default]
    None,
    /// Distribute left edges evenly
    LeftEdges,
    /// Distribute horizontal centers evenly
    HorizontalCenters,
    /// Distribute right edges evenly
    RightEdges,
    /// Distribute horizontal spacing evenly
    HorizontalSpacing,
    /// Distribute top edges evenly
    TopEdges,
    /// Distribute vertical centers evenly
    VerticalCenters,
    /// Distribute bottom edges evenly
    BottomEdges,
    /// Distribute vertical spacing evenly
    VerticalSpacing,
}

/// Snap options.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Snap {
    /// Snap to grid
    pub to_grid: bool,
    /// Grid spacing in points
    pub grid_spacing: f64,
    /// Snap to guides
    pub to_guides: bool,
    /// Snap to objects
    pub to_objects: bool,
    /// Snap threshold in points
    pub threshold: f64,
}

impl Default for Snap {
    fn default() -> Self {
        Self {
            to_grid: true,
            grid_spacing: 12.0, // 1/6 inch
            to_guides: true,
            to_objects: true,
            threshold: 6.0,
        }
    }
}

/// Result of snapping to objects.
#[derive(Debug, Clone, PartialEq)]
pub struct SnapResult {
    /// The snapped position
    pub position: Point,
    /// Whether x was snapped
    pub snapped_x: bool,
    /// Whether y was snapped
    pub snapped_y: bool,
    /// The x value that was snapped to (if any)
    pub snap_x_value: Option<f64>,
    /// The y value that was snapped to (if any)
    pub snap_y_value: Option<f64>,
}

impl Snap {
    /// Snap a point to the grid.
    pub fn snap_to_grid(&self, point: Point) -> Point {
        if !self.to_grid {
            return point;
        }

        Point::new(
            (point.x / self.grid_spacing).round() * self.grid_spacing,
            (point.y / self.grid_spacing).round() * self.grid_spacing,
        )
    }

    /// Snap a value to guides if within threshold.
    pub fn snap_to_guide(&self, value: f64, guides: &[f64]) -> f64 {
        if !self.to_guides {
            return value;
        }

        for guide in guides {
            if (value - guide).abs() <= self.threshold {
                return *guide;
            }
        }

        value
    }

    /// Snap a moving object's bounds to nearby object edges.
    ///
    /// Finds the closest snap points from other objects' edges (left, right, top, bottom)
    /// and centers. Returns the snapped position if within threshold.
    pub fn snap_to_objects(&self, moving_bounds: &Rect, other_objects: &[Rect]) -> SnapResult {
        if !self.to_objects || other_objects.is_empty() {
            return SnapResult {
                position: moving_bounds.origin,
                snapped_x: false,
                snapped_y: false,
                snap_x_value: None,
                snap_y_value: None,
            };
        }

        // Collect all candidate x positions from other objects
        let mut x_candidates: Vec<(f64, f64)> = Vec::new(); // (target_x, snap_to_value)
        let mut y_candidates: Vec<(f64, f64)> = Vec::new(); // (target_y, snap_to_value)

        let moving_left = moving_bounds.x();
        let moving_right = moving_bounds.right();
        let moving_center_x = moving_bounds.center().x;
        let moving_top = moving_bounds.y();
        let moving_bottom = moving_bounds.bottom();
        let moving_center_y = moving_bounds.center().y;

        for other in other_objects {
            let other_left = other.x();
            let other_right = other.right();
            let other_center_x = other.center().x;
            let other_top = other.y();
            let other_bottom = other.bottom();
            let other_center_y = other.center().y;

            // X snapping
            x_candidates.push((moving_left - other_left, other_left));
            x_candidates.push((moving_left - other_right, other_right));
            x_candidates.push((moving_right - other_left, other_left - moving_bounds.width()));
            x_candidates.push((moving_right - other_right, other_right - moving_bounds.width()));
            x_candidates.push((
                moving_center_x - other_center_x,
                other_center_x - moving_bounds.width() / 2.0,
            ));

            // Y snapping
            y_candidates.push((moving_top - other_top, other_top));
            y_candidates.push((moving_top - other_bottom, other_bottom));
            y_candidates.push((moving_bottom - other_top, other_top - moving_bounds.height()));
            y_candidates.push((moving_bottom - other_bottom, other_bottom - moving_bounds.height()));
            y_candidates.push((
                moving_center_y - other_center_y,
                other_center_y - moving_bounds.height() / 2.0,
            ));
        }

        // Find closest x snap within threshold
        let mut best_x: Option<(f64, f64)> = None;
        for (diff, snap_to) in &x_candidates {
            if diff.abs() <= self.threshold
                && (best_x.is_none() || diff.abs() < best_x.unwrap().0.abs())
            {
                best_x = Some((*diff, *snap_to));
            }
        }

        // Find closest y snap within threshold
        let mut best_y: Option<(f64, f64)> = None;
        for (diff, snap_to) in &y_candidates {
            if diff.abs() <= self.threshold
                && (best_y.is_none() || diff.abs() < best_y.unwrap().0.abs())
            {
                best_y = Some((*diff, *snap_to));
            }
        }

        let (new_x, snapped_x, snap_x_value) = match best_x {
            Some((_, snap_to)) => (snap_to, true, Some(snap_to)),
            None => (moving_bounds.origin.x, false, None),
        };

        let (new_y, snapped_y, snap_y_value) = match best_y {
            Some((_, snap_to)) => (snap_to, true, Some(snap_to)),
            None => (moving_bounds.origin.y, false, None),
        };

        SnapResult {
            position: Point::new(new_x, new_y),
            snapped_x,
            snapped_y,
            snap_x_value,
            snap_y_value,
        }
    }
}

/// Orientation of a smart guide.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GuideOrientation {
    /// Horizontal guide (aligns y values)
    Horizontal,
    /// Vertical guide (aligns x values)
    Vertical,
}

/// Type of edge or feature being aligned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentEdge {
    /// Left edge of an object
    Left,
    /// Right edge of an object
    Right,
    /// Top edge of an object
    Top,
    /// Bottom edge of an object
    Bottom,
    /// Horizontal center of an object
    HorizontalCenter,
    /// Vertical center of an object
    VerticalCenter,
}

/// A smart guide that appears during object dragging to show alignment.
#[derive(Debug, Clone, PartialEq)]
pub struct SmartGuide {
    /// The position of the guide line (x for vertical, y for horizontal)
    pub position: f64,
    /// Orientation of the guide
    pub orientation: GuideOrientation,
    /// The edge type being aligned
    pub edge: AlignmentEdge,
    /// Start coordinate for rendering the guide line
    pub start: f64,
    /// End coordinate for rendering the guide line
    pub end: f64,
}

impl SmartGuide {
    /// Create a new smart guide.
    pub fn new(
        position: f64,
        orientation: GuideOrientation,
        edge: AlignmentEdge,
        start: f64,
        end: f64,
    ) -> Self {
        Self {
            position,
            orientation,
            edge,
            start,
            end,
        }
    }
}

/// Find smart guides showing alignment between a moving object and other objects.
///
/// Returns guides for any edges or centers that align within the given threshold.
pub fn find_smart_guides(
    moving_bounds: &Rect,
    other_objects: &[Rect],
    threshold: f64,
) -> Vec<SmartGuide> {
    let mut guides = Vec::new();

    if other_objects.is_empty() {
        return guides;
    }

    let moving_left = moving_bounds.x();
    let moving_right = moving_bounds.right();
    let moving_center_x = moving_bounds.center().x;
    let moving_top = moving_bounds.y();
    let moving_bottom = moving_bounds.bottom();
    let moving_center_y = moving_bounds.center().y;

    for other in other_objects {
        let other_left = other.x();
        let other_right = other.right();
        let other_center_x = other.center().x;
        let other_top = other.y();
        let other_bottom = other.bottom();
        let other_center_y = other.center().y;

        // Check vertical guides (x alignment)
        if (moving_left - other_left).abs() <= threshold {
            let start = moving_top.min(other_top);
            let end = moving_bottom.max(other_bottom);
            guides.push(SmartGuide::new(
                other_left,
                GuideOrientation::Vertical,
                AlignmentEdge::Left,
                start,
                end,
            ));
        }
        if (moving_left - other_right).abs() <= threshold {
            let start = moving_top.min(other_top);
            let end = moving_bottom.max(other_bottom);
            guides.push(SmartGuide::new(
                other_right,
                GuideOrientation::Vertical,
                AlignmentEdge::Right,
                start,
                end,
            ));
        }
        if (moving_right - other_left).abs() <= threshold {
            let start = moving_top.min(other_top);
            let end = moving_bottom.max(other_bottom);
            guides.push(SmartGuide::new(
                other_left,
                GuideOrientation::Vertical,
                AlignmentEdge::Left,
                start,
                end,
            ));
        }
        if (moving_right - other_right).abs() <= threshold {
            let start = moving_top.min(other_top);
            let end = moving_bottom.max(other_bottom);
            guides.push(SmartGuide::new(
                other_right,
                GuideOrientation::Vertical,
                AlignmentEdge::Right,
                start,
                end,
            ));
        }
        if (moving_center_x - other_center_x).abs() <= threshold {
            let start = moving_top.min(other_top);
            let end = moving_bottom.max(other_bottom);
            guides.push(SmartGuide::new(
                other_center_x,
                GuideOrientation::Vertical,
                AlignmentEdge::HorizontalCenter,
                start,
                end,
            ));
        }

        // Check horizontal guides (y alignment)
        if (moving_top - other_top).abs() <= threshold {
            let start = moving_left.min(other_left);
            let end = moving_right.max(other_right);
            guides.push(SmartGuide::new(
                other_top,
                GuideOrientation::Horizontal,
                AlignmentEdge::Top,
                start,
                end,
            ));
        }
        if (moving_top - other_bottom).abs() <= threshold {
            let start = moving_left.min(other_left);
            let end = moving_right.max(other_right);
            guides.push(SmartGuide::new(
                other_bottom,
                GuideOrientation::Horizontal,
                AlignmentEdge::Bottom,
                start,
                end,
            ));
        }
        if (moving_bottom - other_top).abs() <= threshold {
            let start = moving_left.min(other_left);
            let end = moving_right.max(other_right);
            guides.push(SmartGuide::new(
                other_top,
                GuideOrientation::Horizontal,
                AlignmentEdge::Top,
                start,
                end,
            ));
        }
        if (moving_bottom - other_bottom).abs() <= threshold {
            let start = moving_left.min(other_left);
            let end = moving_right.max(other_right);
            guides.push(SmartGuide::new(
                other_bottom,
                GuideOrientation::Horizontal,
                AlignmentEdge::Bottom,
                start,
                end,
            ));
        }
        if (moving_center_y - other_center_y).abs() <= threshold {
            let start = moving_left.min(other_left);
            let end = moving_right.max(other_right);
            guides.push(SmartGuide::new(
                other_center_y,
                GuideOrientation::Horizontal,
                AlignmentEdge::VerticalCenter,
                start,
                end,
            ));
        }
    }

    // Deduplicate guides with the same position and orientation
    guides.sort_by(|a, b| {
        a.orientation
            .cmp(&b.orientation)
            .then(a.position.partial_cmp(&b.position).unwrap())
    });
    guides
        .dedup_by(|a, b| a.orientation == b.orientation && (a.position - b.position).abs() < 0.001);

    guides
}

/// Layout information for a single shape.
#[derive(Debug, Clone)]
pub struct ObjectLayout {
    /// Shape bounds after transform
    pub bounds: Rect,
    /// Rotated bounding box (may be larger than bounds)
    pub rotated_bounds: Rect,
    /// Transform applied to the shape
    pub transform: Transform2D,
}

impl ObjectLayout {
    /// Compute layout for a shape from its common placement.
    pub fn from_shape(shape: &Shape) -> Self {
        let common = &shape.common;
        let bounds = common.bounds;

        let rotated_bounds = if common.rotation != 0.0 {
            compute_rotated_bounds(&bounds, common.rotation)
        } else {
            bounds
        };

        Self {
            bounds,
            rotated_bounds,
            transform: common.transform,
        }
    }
}

/// Compute the axis-aligned bounding box of a rotated rectangle.
pub fn compute_rotated_bounds(rect: &Rect, rotation: f64) -> Rect {
    let angle_rad = rotation.to_radians();
    let cos = angle_rad.cos();
    let sin = angle_rad.sin();

    let cx = rect.origin.x + rect.width() / 2.0;
    let cy = rect.origin.y + rect.height() / 2.0;

    let hw = rect.width() / 2.0;
    let hh = rect.height() / 2.0;

    let corners = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)];

    let rotated: Vec<(f64, f64)> = corners
        .iter()
        .map(|(x, y)| {
            let rx = x * cos - y * sin;
            let ry = x * sin + y * cos;
            (cx + rx, cy + ry)
        })
        .collect();

    let min_x = rotated.iter().map(|(x, _)| *x).fold(f64::INFINITY, f64::min);
    let max_x = rotated
        .iter()
        .map(|(x, _)| *x)
        .fold(f64::NEG_INFINITY, f64::max);
    let min_y = rotated.iter().map(|(_, y)| *y).fold(f64::INFINITY, f64::min);
    let max_y = rotated
        .iter()
        .map(|(_, y)| *y)
        .fold(f64::NEG_INFINITY, f64::max);

    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

/// Align shapes according to the alignment option.
pub fn align_objects(objects: &mut [&mut Shape], alignment: Alignment) {
    if objects.is_empty() {
        return;
    }

    let bounds: Vec<Rect> = objects.iter().map(|o| o.bounds()).collect();

    let (reference_h, reference_v) = match alignment {
        Alignment::None => return,
        Alignment::Left => {
            let min_x = bounds.iter().map(|b| b.x()).fold(f64::INFINITY, f64::min);
            (Some(min_x), None)
        }
        Alignment::Center => {
            let total: f64 = bounds.iter().map(|b| b.center().x).sum();
            (Some(total / bounds.len() as f64), None)
        }
        Alignment::Right => {
            let max_x = bounds.iter().map(|b| b.right()).fold(f64::NEG_INFINITY, f64::max);
            (Some(max_x), None)
        }
        Alignment::Top => {
            let min_y = bounds.iter().map(|b| b.y()).fold(f64::INFINITY, f64::min);
            (None, Some(min_y))
        }
        Alignment::Middle => {
            let total: f64 = bounds.iter().map(|b| b.center().y).sum();
            (None, Some(total / bounds.len() as f64))
        }
        Alignment::Bottom => {
            let max_y = bounds.iter().map(|b| b.bottom()).fold(f64::NEG_INFINITY, f64::max);
            (None, Some(max_y))
        }
    };

    for (obj, bound) in objects.iter_mut().zip(bounds.iter()) {
        let current_pos = bound.origin;
        let mut new_x = current_pos.x;
        let mut new_y = current_pos.y;

        if let Some(ref_x) = reference_h {
            new_x = match alignment {
                Alignment::Left => ref_x,
                Alignment::Center => ref_x - bound.width() / 2.0,
                Alignment::Right => ref_x - bound.width(),
                _ => new_x,
            };
        }

        if let Some(ref_y) = reference_v {
            new_y = match alignment {
                Alignment::Top => ref_y,
                Alignment::Middle => ref_y - bound.height() / 2.0,
                Alignment::Bottom => ref_y - bound.height(),
                _ => new_y,
            };
        }

        obj.set_position_xy(new_x, new_y);
    }
}

/// Distribute shapes according to the distribution option.
pub fn distribute_objects(objects: &mut [&mut Shape], distribution: Distribution) {
    if objects.len() < 3 {
        return; // Need at least 3 objects to distribute
    }

    match distribution {
        Distribution::None => (),
        Distribution::HorizontalSpacing => {
            distribute_horizontal_spacing(objects);
        }
        Distribution::VerticalSpacing => {
            distribute_vertical_spacing(objects);
        }
        Distribution::HorizontalCenters => {
            distribute_horizontal_centers(objects);
        }
        Distribution::VerticalCenters => {
            distribute_vertical_centers(objects);
        }
        _ => {
            // Other distributions are similar
        }
    }
}

fn distribute_horizontal_spacing(objects: &mut [&mut Shape]) {
    let mut indexed: Vec<(usize, f64, f64)> = objects
        .iter()
        .enumerate()
        .map(|(i, o)| {
            let b = o.bounds();
            (i, b.x(), b.width())
        })
        .collect();

    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let total_width: f64 = objects.iter().map(|o| o.bounds().width()).sum();
    let first_x = objects[indexed[0].0].bounds().x();
    let last_right = objects[indexed.last().unwrap().0].bounds().right();
    let total_space = last_right - first_x - total_width;
    let gap = total_space / (objects.len() - 1) as f64;

    let mut current_x = first_x;
    for (i, _, width) in indexed {
        let bounds = objects[i].bounds();
        objects[i].set_position_xy(current_x, bounds.origin.y);
        current_x += width + gap;
    }
}

fn distribute_vertical_spacing(objects: &mut [&mut Shape]) {
    let mut indexed: Vec<(usize, f64, f64)> = objects
        .iter()
        .enumerate()
        .map(|(i, o)| {
            let b = o.bounds();
            (i, b.y(), b.height())
        })
        .collect();

    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let total_height: f64 = objects.iter().map(|o| o.bounds().height()).sum();
    let first_y = objects[indexed[0].0].bounds().y();
    let last_bottom = objects[indexed.last().unwrap().0].bounds().bottom();
    let total_space = last_bottom - first_y - total_height;
    let gap = total_space / (objects.len() - 1) as f64;

    let mut current_y = first_y;
    for (i, _, height) in indexed {
        let bounds = objects[i].bounds();
        objects[i].set_position_xy(bounds.origin.x, current_y);
        current_y += height + gap;
    }
}

fn distribute_horizontal_centers(objects: &mut [&mut Shape]) {
    let mut indexed: Vec<(usize, f64)> = objects
        .iter()
        .enumerate()
        .map(|(i, o)| (i, o.bounds().center().x))
        .collect();

    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let first_center = indexed.first().unwrap().1;
    let last_center = indexed.last().unwrap().1;
    let step = (last_center - first_center) / (objects.len() - 1) as f64;

    for (j, (i, _)) in indexed.iter().enumerate() {
        let bounds = objects[*i].bounds();
        let new_center_x = first_center + step * j as f64;
        let new_x = new_center_x - bounds.width() / 2.0;
        objects[*i].set_position_xy(new_x, bounds.origin.y);
    }
}

fn distribute_vertical_centers(objects: &mut [&mut Shape]) {
    let mut indexed: Vec<(usize, f64)> = objects
        .iter()
        .enumerate()
        .map(|(i, o)| (i, o.bounds().center().y))
        .collect();

    indexed.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());

    let first_center = indexed.first().unwrap().1;
    let last_center = indexed.last().unwrap().1;
    let step = (last_center - first_center) / (objects.len() - 1) as f64;

    for (j, (i, _)) in indexed.iter().enumerate() {
        let bounds = objects[*i].bounds();
        let new_center_y = first_center + step * j as f64;
        let new_y = new_center_y - bounds.height() / 2.0;
        objects[*i].set_position_xy(bounds.origin.x, new_y);
    }
}

/// Options for aligning shapes to a grid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridAlignOptions {
    /// Grid spacing in points
    pub spacing: f64,
    /// Also snap object sizes to grid
    pub snap_size: bool,
}

impl Default for GridAlignOptions {
    fn default() -> Self {
        Self {
            spacing: 12.0,
            snap_size: false,
        }
    }
}

/// Align shapes to a grid.
pub fn align_to_grid(objects: &mut [&mut Shape], grid_spacing: f64) {
    align_to_grid_with_options(
        objects,
        GridAlignOptions {
            spacing: grid_spacing,
            snap_size: false,
        },
    );
}

/// Align shapes to a grid with additional options.
pub fn align_to_grid_with_options(objects: &mut [&mut Shape], options: GridAlignOptions) {
    if options.spacing <= 0.0 {
        return;
    }

    for obj in objects.iter_mut() {
        let bounds = obj.bounds();

        let new_x = (bounds.origin.x / options.spacing).round() * options.spacing;
        let new_y = (bounds.origin.y / options.spacing).round() * options.spacing;
        obj.set_position_xy(new_x, new_y);

        if options.snap_size {
            let new_width = (bounds.width() / options.spacing).round() * options.spacing;
            let new_height = (bounds.height() / options.spacing).round() * options.spacing;
            let final_width = new_width.max(options.spacing);
            let final_height = new_height.max(options.spacing);
            obj.set_size(final_width, final_height);
        }
    }
}

/// Relative positioning relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelativePosition {
    /// Position above the reference object
    Above,
    /// Position below the reference object
    Below,
    /// Position to the left of the reference object
    LeftOf,
    /// Position to the right of the reference object
    RightOf,
    /// Center on the reference object (overlapping)
    CenteredOn,
}

/// Options for relative positioning.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RelativePositionOptions {
    /// Gap between objects (for Above, Below, LeftOf, RightOf)
    pub gap: f64,
    /// Also align perpendicular axis to center
    pub align_center: bool,
}

impl Default for RelativePositionOptions {
    fn default() -> Self {
        Self {
            gap: 0.0,
            align_center: true,
        }
    }
}

/// Position a shape relative to a reference rectangle.
pub fn position_relative_to(object: &mut Shape, reference: &Rect, relation: RelativePosition) {
    position_relative_to_with_options(
        object,
        reference,
        relation,
        RelativePositionOptions::default(),
    );
}

/// Position a shape relative to a reference rectangle with options.
pub fn position_relative_to_with_options(
    object: &mut Shape,
    reference: &Rect,
    relation: RelativePosition,
    options: RelativePositionOptions,
) {
    let obj_bounds = object.bounds();
    let ref_center = reference.center();

    let (new_x, new_y) = match relation {
        RelativePosition::Above => {
            let x = if options.align_center {
                ref_center.x - obj_bounds.width() / 2.0
            } else {
                obj_bounds.origin.x
            };
            let y = reference.y() - obj_bounds.height() - options.gap;
            (x, y)
        }
        RelativePosition::Below => {
            let x = if options.align_center {
                ref_center.x - obj_bounds.width() / 2.0
            } else {
                obj_bounds.origin.x
            };
            let y = reference.bottom() + options.gap;
            (x, y)
        }
        RelativePosition::LeftOf => {
            let x = reference.x() - obj_bounds.width() - options.gap;
            let y = if options.align_center {
                ref_center.y - obj_bounds.height() / 2.0
            } else {
                obj_bounds.origin.y
            };
            (x, y)
        }
        RelativePosition::RightOf => {
            let x = reference.right() + options.gap;
            let y = if options.align_center {
                ref_center.y - obj_bounds.height() / 2.0
            } else {
                obj_bounds.origin.y
            };
            (x, y)
        }
        RelativePosition::CenteredOn => {
            let x = ref_center.x - obj_bounds.width() / 2.0;
            let y = ref_center.y - obj_bounds.height() / 2.0;
            (x, y)
        }
    };

    object.set_position_xy(new_x, new_y);
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_model::shape::Shape;

    #[test]
    fn test_snap_to_grid() {
        let snap = Snap::default();

        let point = Point::new(13.0, 25.0);
        let snapped = snap.snap_to_grid(point);

        assert_eq!(snapped.x, 12.0);
        assert_eq!(snapped.y, 24.0);
    }

    #[test]
    fn test_rotated_bounds() {
        let rect = Rect::new(0.0, 0.0, 100.0, 50.0);
        let rotated = compute_rotated_bounds(&rect, 45.0);

        assert!(rotated.width() > rect.width());
        assert!(rotated.height() > rect.height());
    }

    #[test]
    fn test_alignment() {
        let mut shape1 = Shape::rectangle(Rect::new(0.0, 0.0, 50.0, 50.0));
        let mut shape2 = Shape::rectangle(Rect::new(100.0, 50.0, 50.0, 50.0));

        let mut objects: Vec<&mut Shape> = vec![&mut shape1, &mut shape2];
        align_objects(&mut objects, Alignment::Left);

        assert_eq!(shape1.bounds().x(), 0.0);
        assert_eq!(shape2.bounds().x(), 0.0);
    }

    #[test]
    fn test_snap_to_objects_left_edge() {
        let snap = Snap {
            to_objects: true,
            threshold: 6.0,
            ..Default::default()
        };

        let moving = Rect::new(98.0, 50.0, 50.0, 50.0);
        let others = vec![Rect::new(100.0, 0.0, 50.0, 50.0)];

        let result = snap.snap_to_objects(&moving, &others);

        assert!(result.snapped_x);
        assert_eq!(result.position.x, 100.0);
    }

    #[test]
    fn test_snap_to_objects_right_to_left() {
        let snap = Snap {
            to_objects: true,
            threshold: 6.0,
            ..Default::default()
        };

        let moving = Rect::new(45.0, 50.0, 50.0, 50.0);
        let others = vec![Rect::new(100.0, 0.0, 50.0, 50.0)];

        let result = snap.snap_to_objects(&moving, &others);

        assert!(result.snapped_x);
        assert_eq!(result.position.x, 50.0);
    }

    #[test]
    fn test_snap_to_objects_center_alignment() {
        let snap = Snap {
            to_objects: true,
            threshold: 6.0,
            ..Default::default()
        };

        let moving = Rect::new(73.0, 50.0, 50.0, 50.0);
        let others = vec![Rect::new(75.0, 0.0, 50.0, 50.0)];

        let result = snap.snap_to_objects(&moving, &others);

        assert!(result.snapped_x);
        assert_eq!(result.position.x, 75.0);
    }

    #[test]
    fn test_snap_to_objects_multiple_objects() {
        let snap = Snap {
            to_objects: true,
            threshold: 6.0,
            ..Default::default()
        };

        let moving = Rect::new(102.0, 50.0, 50.0, 50.0);
        let others = vec![
            Rect::new(50.0, 0.0, 50.0, 50.0),
            Rect::new(100.0, 0.0, 50.0, 50.0),
        ];

        let result = snap.snap_to_objects(&moving, &others);

        assert!(result.snapped_x);
        assert_eq!(result.position.x, 100.0);
    }

    #[test]
    fn test_snap_to_objects_disabled() {
        let snap = Snap {
            to_objects: false,
            threshold: 6.0,
            ..Default::default()
        };

        let moving = Rect::new(98.0, 50.0, 50.0, 50.0);
        let others = vec![Rect::new(100.0, 0.0, 50.0, 50.0)];

        let result = snap.snap_to_objects(&moving, &others);

        assert!(!result.snapped_x);
        assert!(!result.snapped_y);
        assert_eq!(result.position.x, 98.0);
    }

    #[test]
    fn test_snap_to_objects_outside_threshold() {
        let snap = Snap {
            to_objects: true,
            threshold: 6.0,
            ..Default::default()
        };

        let moving = Rect::new(50.0, 50.0, 50.0, 50.0);
        let others = vec![Rect::new(200.0, 200.0, 50.0, 50.0)];

        let result = snap.snap_to_objects(&moving, &others);

        assert!(!result.snapped_x);
        assert!(!result.snapped_y);
    }

    #[test]
    fn test_smart_guides_left_alignment() {
        let moving = Rect::new(100.0, 50.0, 50.0, 50.0);
        let others = vec![Rect::new(100.0, 0.0, 60.0, 40.0)];

        let guides = find_smart_guides(&moving, &others, 6.0);

        assert!(!guides.is_empty());
        let vertical_guides: Vec<_> = guides
            .iter()
            .filter(|g| g.orientation == GuideOrientation::Vertical)
            .collect();
        assert!(!vertical_guides.is_empty());
        assert_eq!(vertical_guides[0].position, 100.0);
    }

    #[test]
    fn test_smart_guides_center_alignment() {
        let moving = Rect::new(75.0, 50.0, 50.0, 50.0);
        let others = vec![Rect::new(50.0, 0.0, 100.0, 40.0)];

        let guides = find_smart_guides(&moving, &others, 6.0);

        let center_guides: Vec<_> = guides
            .iter()
            .filter(|g| g.edge == AlignmentEdge::HorizontalCenter)
            .collect();
        assert!(!center_guides.is_empty());
        assert_eq!(center_guides[0].position, 100.0);
    }

    #[test]
    fn test_smart_guides_horizontal() {
        let moving = Rect::new(100.0, 50.0, 50.0, 50.0);
        let others = vec![Rect::new(0.0, 50.0, 60.0, 40.0)];

        let guides = find_smart_guides(&moving, &others, 6.0);

        let horizontal_guides: Vec<_> = guides
            .iter()
            .filter(|g| g.orientation == GuideOrientation::Horizontal)
            .collect();
        assert!(!horizontal_guides.is_empty());
    }

    #[test]
    fn test_smart_guides_no_alignment() {
        let moving = Rect::new(100.0, 100.0, 50.0, 50.0);
        let others = vec![Rect::new(0.0, 0.0, 30.0, 30.0)];

        let guides = find_smart_guides(&moving, &others, 6.0);

        assert!(guides.is_empty());
    }

    #[test]
    fn test_smart_guides_deduplication() {
        let moving = Rect::new(100.0, 50.0, 50.0, 50.0);
        let others = vec![
            Rect::new(100.0, 0.0, 60.0, 40.0),
            Rect::new(100.0, 150.0, 70.0, 30.0),
        ];

        let guides = find_smart_guides(&moving, &others, 6.0);

        let vertical_at_100: Vec<_> = guides
            .iter()
            .filter(|g| {
                g.orientation == GuideOrientation::Vertical && (g.position - 100.0).abs() < 0.001
            })
            .collect();
        assert_eq!(vertical_at_100.len(), 1);
    }

    #[test]
    fn test_align_to_grid_basic() {
        let mut shape1 = Shape::rectangle(Rect::new(13.0, 27.0, 50.0, 50.0));
        let mut shape2 = Shape::rectangle(Rect::new(45.0, 83.0, 50.0, 50.0));

        let mut objects: Vec<&mut Shape> = vec![&mut shape1, &mut shape2];
        align_to_grid(&mut objects, 12.0);

        assert_eq!(shape1.bounds().x(), 12.0);
        assert_eq!(shape1.bounds().y(), 24.0);
        assert_eq!(shape2.bounds().x(), 48.0);
        assert_eq!(shape2.bounds().y(), 84.0);
    }

    #[test]
    fn test_align_to_grid_with_size_snap() {
        let mut shape = Shape::rectangle(Rect::new(13.0, 27.0, 55.0, 47.0));

        let mut objects: Vec<&mut Shape> = vec![&mut shape];
        align_to_grid_with_options(
            &mut objects,
            GridAlignOptions {
                spacing: 12.0,
                snap_size: true,
            },
        );

        assert_eq!(shape.bounds().x(), 12.0);
        assert_eq!(shape.bounds().y(), 24.0);
        assert_eq!(shape.bounds().width(), 60.0);
        assert_eq!(shape.bounds().height(), 48.0);
    }

    #[test]
    fn test_align_to_grid_minimum_size() {
        let mut shape = Shape::rectangle(Rect::new(0.0, 0.0, 3.0, 3.0));

        let mut objects: Vec<&mut Shape> = vec![&mut shape];
        align_to_grid_with_options(
            &mut objects,
            GridAlignOptions {
                spacing: 12.0,
                snap_size: true,
            },
        );

        assert_eq!(shape.bounds().width(), 12.0);
        assert_eq!(shape.bounds().height(), 12.0);
    }

    #[test]
    fn test_align_to_grid_zero_spacing() {
        let mut shape = Shape::rectangle(Rect::new(13.0, 27.0, 50.0, 50.0));
        let original_x = shape.bounds().x();

        let mut objects: Vec<&mut Shape> = vec![&mut shape];
        align_to_grid(&mut objects, 0.0);

        assert_eq!(shape.bounds().x(), original_x);
    }

    #[test]
    fn test_position_above() {
        let mut obj = Shape::rectangle(Rect::new(0.0, 0.0, 40.0, 30.0));
        let reference = Rect::new(100.0, 100.0, 60.0, 50.0);

        position_relative_to(&mut obj, &reference, RelativePosition::Above);

        assert_eq!(obj.bounds().bottom(), 100.0);
        assert_eq!(obj.bounds().center().x, reference.center().x);
    }

    #[test]
    fn test_position_below() {
        let mut obj = Shape::rectangle(Rect::new(0.0, 0.0, 40.0, 30.0));
        let reference = Rect::new(100.0, 100.0, 60.0, 50.0);

        position_relative_to(&mut obj, &reference, RelativePosition::Below);

        assert_eq!(obj.bounds().y(), 150.0);
        assert_eq!(obj.bounds().center().x, reference.center().x);
    }

    #[test]
    fn test_position_left_of() {
        let mut obj = Shape::rectangle(Rect::new(0.0, 0.0, 40.0, 30.0));
        let reference = Rect::new(100.0, 100.0, 60.0, 50.0);

        position_relative_to(&mut obj, &reference, RelativePosition::LeftOf);

        assert_eq!(obj.bounds().right(), 100.0);
        assert_eq!(obj.bounds().center().y, reference.center().y);
    }

    #[test]
    fn test_position_right_of() {
        let mut obj = Shape::rectangle(Rect::new(0.0, 0.0, 40.0, 30.0));
        let reference = Rect::new(100.0, 100.0, 60.0, 50.0);

        position_relative_to(&mut obj, &reference, RelativePosition::RightOf);

        assert_eq!(obj.bounds().x(), 160.0);
        assert_eq!(obj.bounds().center().y, reference.center().y);
    }

    #[test]
    fn test_position_centered_on() {
        let mut obj = Shape::rectangle(Rect::new(0.0, 0.0, 40.0, 30.0));
        let reference = Rect::new(100.0, 100.0, 60.0, 50.0);

        position_relative_to(&mut obj, &reference, RelativePosition::CenteredOn);

        let obj_center = obj.bounds().center();
        let ref_center = reference.center();
        assert!((obj_center.x - ref_center.x).abs() < 0.001);
        assert!((obj_center.y - ref_center.y).abs() < 0.001);
    }

    #[test]
    fn test_position_with_gap() {
        let mut obj = Shape::rectangle(Rect::new(0.0, 0.0, 40.0, 30.0));
        let reference = Rect::new(100.0, 100.0, 60.0, 50.0);

        position_relative_to_with_options(
            &mut obj,
            &reference,
            RelativePosition::Below,
            RelativePositionOptions {
                gap: 10.0,
                align_center: true,
            },
        );

        assert_eq!(obj.bounds().y(), 160.0);
    }

    #[test]
    fn test_position_no_center_align() {
        let mut obj = Shape::rectangle(Rect::new(50.0, 50.0, 40.0, 30.0));
        let original_x = obj.bounds().x();
        let reference = Rect::new(100.0, 100.0, 60.0, 50.0);

        position_relative_to_with_options(
            &mut obj,
            &reference,
            RelativePosition::Below,
            RelativePositionOptions {
                gap: 0.0,
                align_center: false,
            },
        );

        assert_eq!(obj.bounds().x(), original_x);
        assert_eq!(obj.bounds().y(), 150.0);
    }
}
