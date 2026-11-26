//! Drawing and editing tools.
//!
//! The tool state machine is framework-free. Cursors are reported as the
//! framework-free [`CursorKind`]; a GUI maps them to its own cursor type.

use repub_color::Color;
use repub_geometry::{Point, Rect};
use repub_model::geometry::{AutoShape, PresetGeometry, ShapeGeometry};
use repub_model::shape::{LineData, Shape, ShapeKind, TableData};
use repub_model::styles::Stroke;

/// A framework-free cursor shape. A GUI maps these to its own cursor type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorKind {
    /// Default arrow cursor.
    #[default]
    Default,
    /// Text I-beam cursor.
    Text,
    /// Crosshair for drawing shapes.
    Crosshair,
    /// Open-hand for panning.
    Grab,
    /// Zoom-in cursor.
    ZoomIn,
    /// Diagonal resize, top-left to bottom-right.
    ResizeNwSe,
    /// Diagonal resize, top-right to bottom-left.
    ResizeNeSw,
    /// Vertical resize.
    ResizeVertical,
    /// Horizontal resize.
    ResizeHorizontal,
}

/// Available tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    /// Selection tool for selecting and manipulating shapes
    #[default]
    Select,
    /// Text tool for creating and editing text frames
    Text,
    /// Rectangle shape tool
    Rectangle,
    /// Ellipse shape tool
    Ellipse,
    /// Line tool
    Line,
    /// Polygon shape tool
    Polygon,
    /// Star shape tool
    Star,
    /// Image insertion tool
    Image,
    /// Table insertion tool
    Table,
    /// Pan tool for scrolling the canvas
    Pan,
    /// Zoom tool
    Zoom,
    /// Draws an Office preset shape (Escher shape type)
    Preset(u16),
}

impl Tool {
    /// Get the tool name.
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Text => "Text",
            Tool::Rectangle => "Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::Line => "Line",
            Tool::Polygon => "Polygon",
            Tool::Star => "Star",
            Tool::Image => "Image",
            Tool::Table => "Table",
            Tool::Pan => "Pan",
            Tool::Zoom => "Zoom",
            Tool::Preset(_) => "Preset Shape",
        }
    }

    /// Get the cursor for this tool.
    pub fn cursor(&self) -> CursorKind {
        match self {
            Tool::Select => CursorKind::Default,
            Tool::Text => CursorKind::Text,
            Tool::Rectangle | Tool::Ellipse | Tool::Polygon | Tool::Star | Tool::Preset(_) => {
                CursorKind::Crosshair
            }
            Tool::Line => CursorKind::Crosshair,
            Tool::Image => CursorKind::Crosshair,
            Tool::Table => CursorKind::Crosshair,
            Tool::Pan => CursorKind::Grab,
            Tool::Zoom => CursorKind::ZoomIn,
        }
    }

    /// Check if this tool creates shapes.
    pub fn is_creation_tool(&self) -> bool {
        matches!(
            self,
            Tool::Rectangle
                | Tool::Ellipse
                | Tool::Line
                | Tool::Polygon
                | Tool::Star
                | Tool::Text
                | Tool::Table
                | Tool::Preset(_)
        )
    }
}

/// Outline width of new shapes and lines, in points.
pub const DEFAULT_OUTLINE_WIDTH: f64 = 2.0;
/// Outline colour of new shapes that don't set one.
pub const DEFAULT_OUTLINE: Color = Color::new(0.25, 0.25, 0.25, 1.0);

/// State for drawing operations.
#[derive(Debug, Default)]
pub struct DrawState {
    /// Is drawing in progress?
    pub is_drawing: bool,
    /// Start point of the draw operation
    pub start_point: Option<Point>,
    /// Current point during drawing
    pub current_point: Option<Point>,
    /// Modifier: constrain proportions (Shift)
    pub constrain: bool,
    /// Modifier: draw from center (Alt)
    pub from_center: bool,
    /// Number of sides for polygon
    pub polygon_sides: u32,
    /// Number of points for star
    pub star_points: u32,
    /// Inner radius ratio for star (0.0-1.0)
    pub star_inner_ratio: f64,
    /// Number of rows for table creation
    pub table_rows: u32,
    /// Number of columns for table creation
    pub table_cols: u32,
}

impl DrawState {
    /// Create a new draw state.
    pub fn new() -> Self {
        Self {
            polygon_sides: 6,
            star_points: 5,
            star_inner_ratio: 0.5,
            table_rows: 3,
            table_cols: 3,
            ..Default::default()
        }
    }

    /// Start a draw operation.
    pub fn start(&mut self, point: Point) {
        self.is_drawing = true;
        self.start_point = Some(point);
        self.current_point = Some(point);
    }

    /// Update the current point.
    pub fn update(&mut self, point: Point) {
        self.current_point = Some(point);
    }

    /// End the draw operation and return the bounds.
    pub fn end(&mut self) -> Option<Rect> {
        let result = self.bounds();
        self.is_drawing = false;
        self.start_point = None;
        self.current_point = None;
        result
    }

    /// Cancel the draw operation.
    pub fn cancel(&mut self) {
        self.is_drawing = false;
        self.start_point = None;
        self.current_point = None;
    }

    /// Get the bounds of the current draw operation.
    pub fn bounds(&self) -> Option<Rect> {
        let start = self.start_point?;
        let end = self.current_point?;

        let mut min_x = start.x.min(end.x);
        let mut min_y = start.y.min(end.y);
        let mut max_x = start.x.max(end.x);
        let mut max_y = start.y.max(end.y);

        // If constraining proportions, make it a square
        if self.constrain {
            let width = max_x - min_x;
            let height = max_y - min_y;
            let size = width.max(height);

            if end.x < start.x {
                min_x = start.x - size;
                max_x = start.x;
            } else {
                max_x = start.x + size;
            }

            if end.y < start.y {
                min_y = start.y - size;
                max_y = start.y;
            } else {
                max_y = start.y + size;
            }
        }

        // If drawing from center
        if self.from_center {
            let half_w = (max_x - min_x) / 2.0;
            let half_h = (max_y - min_y) / 2.0;
            min_x = start.x - half_w;
            max_x = start.x + half_w;
            min_y = start.y - half_h;
            max_y = start.y + half_h;
        }

        let width = max_x - min_x;
        let height = max_y - min_y;

        // Minimum size
        if width < 1.0 || height < 1.0 {
            return None;
        }

        Some(Rect::new(min_x, min_y, width, height))
    }

    /// Get the line for the current draw operation.
    pub fn line(&self) -> Option<(Point, Point)> {
        let start = self.start_point?;
        let mut end = self.current_point?;

        // If constraining, snap to 45-degree angles
        if self.constrain {
            let dx = end.x - start.x;
            let dy = end.y - start.y;
            let angle = dy.atan2(dx);
            let snapped_angle =
                (angle / (std::f64::consts::PI / 4.0)).round() * (std::f64::consts::PI / 4.0);
            let length = (dx * dx + dy * dy).sqrt();
            end = Point::new(
                start.x + length * snapped_angle.cos(),
                start.y + length * snapped_angle.sin(),
            );
        }

        Some((start, end))
    }

    /// Create a shape from the current draw state.
    pub fn create_shape(&self, tool: Tool) -> Option<Shape> {
        let mut shape = self.new_shape(tool)?;
        // New shapes and lines get a 2 pt outline (keeping its colour when set)
        match &shape.kind {
            ShapeKind::Line(_) => {
                let color = shape.line.as_ref().map_or(DEFAULT_OUTLINE, |s| s.color);
                shape.line = Some(Stroke::new(color, DEFAULT_OUTLINE_WIDTH));
            }
            ShapeKind::AutoShape(_) => {
                let color = shape.line.as_ref().map_or(DEFAULT_OUTLINE, |s| s.color);
                shape.line = Some(Stroke::new(color, DEFAULT_OUTLINE_WIDTH));
            }
            _ => {}
        }
        Some(shape)
    }

    fn new_shape(&self, tool: Tool) -> Option<Shape> {
        match tool {
            Tool::Rectangle => {
                let bounds = self.bounds()?;
                Some(Shape::rectangle(bounds))
            }
            Tool::Ellipse => {
                let bounds = self.bounds()?;
                Some(Shape::ellipse(bounds))
            }
            Tool::Polygon => {
                let bounds = self.bounds()?;
                Some(Shape::polygon(bounds, self.polygon_sides))
            }
            Tool::Star => {
                let bounds = self.bounds()?;
                Some(Shape::star(bounds, self.star_points, self.star_inner_ratio))
            }
            Tool::Line => {
                let (start, end) = self.line()?;
                Some(Shape::line(start, end))
            }
            Tool::Text => {
                let bounds = self.bounds()?;
                Some(Shape::text_frame(bounds))
            }
            Tool::Preset(spt) => {
                let bounds = self.bounds()?;
                let auto = AutoShape {
                    geometry: ShapeGeometry::Rectangle { corner_radius: 0.0 },
                    preset: Some(PresetGeometry::new(spt)),
                };
                Some(Shape::new(bounds, ShapeKind::AutoShape(auto)))
            }
            Tool::Table => {
                let bounds = self.bounds()?;
                let rows = self.table_rows.max(1);
                let cols = self.table_cols.max(1);
                Some(Shape::new(
                    bounds,
                    ShapeKind::Table(TableData::new(bounds, rows, cols)),
                ))
            }
            Tool::Image => {
                let bounds = self.bounds()?;
                Some(Shape::picture(bounds))
            }
            _ => None,
        }
    }
}

/// State for selection operations.
#[derive(Debug, Default)]
pub struct SelectState {
    /// Is marquee selection in progress?
    pub is_marquee: bool,
    /// Start point of marquee
    pub marquee_start: Option<Point>,
    /// Current point of marquee
    pub marquee_current: Option<Point>,
    /// Is dragging selected shapes?
    pub is_dragging: bool,
    /// Start point of drag
    pub drag_start: Option<Point>,
    /// Current point of drag
    pub drag_current: Option<Point>,
    /// Is resizing?
    pub is_resizing: bool,
    /// Resize handle being dragged
    pub resize_handle: Option<ResizeHandle>,
    /// Original bounds before resize
    pub original_bounds: Option<Rect>,
    /// Rotation (degrees) of the box being resized; the resize works in its axes
    pub resize_rotation: f64,
    /// Is rotating?
    pub is_rotating: bool,
    /// Start angle for rotation
    pub rotation_start_angle: Option<f64>,
    /// Current angle for rotation
    pub rotation_current_angle: Option<f64>,
    /// Original rotation before operation
    pub original_rotation: Option<f64>,
    /// Center point for rotation
    pub rotation_center: Option<Point>,
    /// Current handle being hovered (for cursor)
    pub hovered_handle: Option<HandleType>,
}

/// Handle type for selection interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandleType {
    /// Resize handle
    Resize(ResizeHandle),
    /// Rotation handle
    Rotation,
    /// Shape body (for moving)
    Body,
    /// Adjust handle `n` of a preset shape (changes its proportions)
    Adjust(usize),
}

/// Resize handle positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeHandle {
    /// Top-left corner
    TopLeft,
    /// Top-center
    TopCenter,
    /// Top-right corner
    TopRight,
    /// Middle-left
    MiddleLeft,
    /// Middle-right
    MiddleRight,
    /// Bottom-left corner
    BottomLeft,
    /// Bottom-center
    BottomCenter,
    /// Bottom-right corner
    BottomRight,
}

impl ResizeHandle {
    /// Get all handles.
    pub fn all() -> [ResizeHandle; 8] {
        [
            ResizeHandle::TopLeft,
            ResizeHandle::TopCenter,
            ResizeHandle::TopRight,
            ResizeHandle::MiddleLeft,
            ResizeHandle::MiddleRight,
            ResizeHandle::BottomLeft,
            ResizeHandle::BottomCenter,
            ResizeHandle::BottomRight,
        ]
    }

    /// Get the cursor for this handle.
    pub fn cursor(&self) -> CursorKind {
        match self {
            ResizeHandle::TopLeft | ResizeHandle::BottomRight => CursorKind::ResizeNwSe,
            ResizeHandle::TopRight | ResizeHandle::BottomLeft => CursorKind::ResizeNeSw,
            ResizeHandle::TopCenter | ResizeHandle::BottomCenter => CursorKind::ResizeVertical,
            ResizeHandle::MiddleLeft | ResizeHandle::MiddleRight => CursorKind::ResizeHorizontal,
        }
    }

    /// The resize cursor for this handle on a box turned by `rotation` degrees.
    pub fn cursor_rotated(&self, rotation: f64) -> CursorKind {
        let base = match self {
            ResizeHandle::MiddleRight | ResizeHandle::MiddleLeft => 0.0,
            ResizeHandle::BottomRight | ResizeHandle::TopLeft => 45.0,
            ResizeHandle::BottomCenter | ResizeHandle::TopCenter => 90.0,
            ResizeHandle::BottomLeft | ResizeHandle::TopRight => 135.0,
        };
        match (((base + rotation) / 45.0).round() as i64).rem_euclid(4) {
            0 => CursorKind::ResizeHorizontal,
            1 => CursorKind::ResizeNwSe,
            2 => CursorKind::ResizeVertical,
            _ => CursorKind::ResizeNeSw,
        }
    }

    /// Get the position of this handle relative to bounds.
    pub fn position(&self, bounds: &Rect) -> Point {
        let (x_ratio, y_ratio) = match self {
            ResizeHandle::TopLeft => (0.0, 0.0),
            ResizeHandle::TopCenter => (0.5, 0.0),
            ResizeHandle::TopRight => (1.0, 0.0),
            ResizeHandle::MiddleLeft => (0.0, 0.5),
            ResizeHandle::MiddleRight => (1.0, 0.5),
            ResizeHandle::BottomLeft => (0.0, 1.0),
            ResizeHandle::BottomCenter => (0.5, 1.0),
            ResizeHandle::BottomRight => (1.0, 1.0),
        };

        Point::new(
            bounds.origin.x + bounds.width() * x_ratio,
            bounds.origin.y + bounds.height() * y_ratio,
        )
    }
}

impl SelectState {
    /// Create a new selection state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a marquee selection.
    pub fn start_marquee(&mut self, point: Point) {
        self.is_marquee = true;
        self.marquee_start = Some(point);
        self.marquee_current = Some(point);
    }

    /// Update marquee.
    pub fn update_marquee(&mut self, point: Point) {
        self.marquee_current = Some(point);
    }

    /// End marquee and return the selection rect.
    pub fn end_marquee(&mut self) -> Option<Rect> {
        let start = self.marquee_start?;
        let end = self.marquee_current?;

        self.is_marquee = false;
        self.marquee_start = None;
        self.marquee_current = None;

        let min_x = start.x.min(end.x);
        let min_y = start.y.min(end.y);
        let max_x = start.x.max(end.x);
        let max_y = start.y.max(end.y);

        Some(Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
    }

    /// Get the current marquee rect.
    pub fn marquee_rect(&self) -> Option<Rect> {
        let start = self.marquee_start?;
        let end = self.marquee_current?;

        let min_x = start.x.min(end.x);
        let min_y = start.y.min(end.y);
        let max_x = start.x.max(end.x);
        let max_y = start.y.max(end.y);

        Some(Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
    }

    /// Start dragging.
    pub fn start_drag(&mut self, point: Point) {
        self.is_dragging = true;
        self.drag_start = Some(point);
        self.drag_current = Some(point);
    }

    /// Update drag.
    pub fn update_drag(&mut self, point: Point) {
        self.drag_current = Some(point);
    }

    /// End drag and return the delta.
    pub fn end_drag(&mut self) -> Option<Point> {
        let start = self.drag_start?;
        let end = self.drag_current?;

        self.is_dragging = false;
        self.drag_start = None;
        self.drag_current = None;

        Some(Point::new(end.x - start.x, end.y - start.y))
    }

    /// Get the current drag delta.
    pub fn drag_delta(&self) -> Option<Point> {
        let start = self.drag_start?;
        let end = self.drag_current?;
        Some(Point::new(end.x - start.x, end.y - start.y))
    }

    /// Start resizing.
    pub fn start_resize(&mut self, handle: ResizeHandle, bounds: Rect) {
        self.is_resizing = true;
        self.resize_handle = Some(handle);
        self.original_bounds = Some(bounds);
    }

    /// End resize and return the new bounds.
    pub fn end_resize(&mut self) -> Option<Rect> {
        let result = self.original_bounds;
        self.is_resizing = false;
        self.resize_handle = None;
        self.original_bounds = None;
        result
    }

    /// Start rotating.
    pub fn start_rotation(&mut self, center: Point, start_angle: f64, original_rotation: f64) {
        self.is_rotating = true;
        self.rotation_center = Some(center);
        self.rotation_start_angle = Some(start_angle);
        self.rotation_current_angle = Some(start_angle);
        self.original_rotation = Some(original_rotation);
    }

    /// Update rotation.
    pub fn update_rotation(&mut self, current_angle: f64) {
        self.rotation_current_angle = Some(current_angle);
    }

    /// End rotation and return the angle delta.
    pub fn end_rotation(&mut self) -> Option<f64> {
        let start = self.rotation_start_angle?;
        let current = self.rotation_current_angle?;

        self.is_rotating = false;
        self.rotation_center = None;
        self.rotation_start_angle = None;
        self.rotation_current_angle = None;
        self.original_rotation = None;

        Some(current - start)
    }

    /// Get the current rotation delta.
    pub fn rotation_delta(&self) -> Option<f64> {
        let start = self.rotation_start_angle?;
        let current = self.rotation_current_angle?;
        Some(current - start)
    }

    /// Cancel any ongoing operation.
    pub fn cancel(&mut self) {
        self.is_marquee = false;
        self.marquee_start = None;
        self.marquee_current = None;
        self.is_dragging = false;
        self.drag_start = None;
        self.drag_current = None;
        self.is_resizing = false;
        self.resize_handle = None;
        self.original_bounds = None;
        self.is_rotating = false;
        self.rotation_center = None;
        self.rotation_start_angle = None;
        self.rotation_current_angle = None;
        self.original_rotation = None;
        self.hovered_handle = None;
    }

    /// Check if any operation is in progress.
    pub fn is_busy(&self) -> bool {
        self.is_marquee || self.is_dragging || self.is_resizing || self.is_rotating
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_bounds() {
        let mut state = DrawState::new();
        state.start(Point::new(100.0, 100.0));
        state.update(Point::new(200.0, 150.0));

        let bounds = state.bounds().unwrap();
        assert_eq!(bounds.origin.x, 100.0);
        assert_eq!(bounds.origin.y, 100.0);
        assert_eq!(bounds.width(), 100.0);
        assert_eq!(bounds.height(), 50.0);
    }

    #[test]
    fn test_draw_constrain() {
        let mut state = DrawState::new();
        state.constrain = true;
        state.start(Point::new(100.0, 100.0));
        state.update(Point::new(200.0, 120.0));

        let bounds = state.bounds().unwrap();
        // Should be a square
        assert_eq!(bounds.width(), bounds.height());
    }

    #[test]
    fn test_marquee_rect() {
        let mut state = SelectState::new();
        state.start_marquee(Point::new(100.0, 100.0));
        state.update_marquee(Point::new(50.0, 150.0));

        let rect = state.marquee_rect().unwrap();
        assert_eq!(rect.origin.x, 50.0);
        assert_eq!(rect.origin.y, 100.0);
        assert_eq!(rect.width(), 50.0);
        assert_eq!(rect.height(), 50.0);
    }

    #[test]
    fn create_rectangle_gets_default_outline() {
        let mut state = DrawState::new();
        state.start(Point::new(0.0, 0.0));
        state.update(Point::new(50.0, 40.0));
        let shape = state.create_shape(Tool::Rectangle).unwrap();
        assert!(matches!(shape.kind, ShapeKind::AutoShape(_)));
        assert_eq!(shape.line.unwrap().width, DEFAULT_OUTLINE_WIDTH);
    }

    #[test]
    fn preset_tool_builds_a_preset_auto_shape() {
        let mut state = DrawState::new();
        state.start(Point::new(0.0, 0.0));
        state.update(Point::new(50.0, 40.0));
        let shape = state.create_shape(Tool::Preset(5)).unwrap();
        match &shape.kind {
            ShapeKind::AutoShape(auto) => assert_eq!(auto.preset.as_ref().unwrap().spt, 5),
            _ => panic!("expected auto shape"),
        }
    }

    #[test]
    fn tool_cursor_is_framework_free() {
        assert_eq!(Tool::Text.cursor(), CursorKind::Text);
        assert_eq!(Tool::Pan.cursor(), CursorKind::Grab);
        assert_eq!(ResizeHandle::TopLeft.cursor(), CursorKind::ResizeNwSe);
    }
}
