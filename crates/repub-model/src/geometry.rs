//! Vector geometry for auto shapes: paths, primitive outlines, and the
//! plain-data reference to an Office preset ("Escher") shape.

use repub_geometry::{Point, Rect};

/// A Bézier path command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PathCommand {
    /// Move to a point (start a new subpath).
    MoveTo(Point),
    /// Draw a line to a point.
    LineTo(Point),
    /// Draw a quadratic Bézier curve.
    QuadTo {
        /// Control point.
        control: Point,
        /// End point.
        end: Point,
    },
    /// Draw a cubic Bézier curve.
    CubicTo {
        /// First control point.
        control1: Point,
        /// Second control point.
        control2: Point,
        /// End point.
        end: Point,
    },
    /// Close the current subpath.
    Close,
}

/// How one subpath of a [`Path`] is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SubpathStyle {
    /// Not filled (only outlined).
    pub no_fill: bool,
    /// Not outlined (only filled).
    pub no_stroke: bool,
}

/// A Bézier path built from [`PathCommand`]s.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Path {
    /// The ordered path commands.
    pub commands: Vec<PathCommand>,
    /// Per-subpath drawing flags, in subpath order (each `MoveTo` starts one);
    /// subpaths beyond the list are both filled and stroked.
    pub subpaths: Vec<SubpathStyle>,
    /// Fill with the even-odd rule (holes where subpaths overlap, as Office
    /// draws its shapes) instead of nonzero.
    pub even_odd: bool,
}

impl Path {
    /// Create an empty path.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
            subpaths: Vec::new(),
            even_odd: false,
        }
    }

    /// The drawing flags of subpath `index`.
    pub fn subpath_style(&self, index: usize) -> SubpathStyle {
        self.subpaths.get(index).copied().unwrap_or_default()
    }

    /// The subpaths as separate command lists (each starting at a `MoveTo`).
    pub fn split_subpaths(&self) -> Vec<&[PathCommand]> {
        let mut out = Vec::new();
        let mut start = 0;
        for (i, c) in self.commands.iter().enumerate() {
            if matches!(c, PathCommand::MoveTo(_)) && i > start {
                out.push(&self.commands[start..i]);
                start = i;
            }
        }
        if start < self.commands.len() {
            out.push(&self.commands[start..]);
        }
        out
    }

    /// Create a fluent path builder.
    pub fn builder() -> PathBuilder {
        PathBuilder::new()
    }

    /// Move to a point.
    pub fn move_to(&mut self, x: f64, y: f64) {
        self.commands.push(PathCommand::MoveTo(Point::new(x, y)));
    }

    /// Draw a line to a point.
    pub fn line_to(&mut self, x: f64, y: f64) {
        self.commands.push(PathCommand::LineTo(Point::new(x, y)));
    }

    /// Draw a quadratic Bézier curve.
    pub fn quad_to(&mut self, cx: f64, cy: f64, x: f64, y: f64) {
        self.commands.push(PathCommand::QuadTo {
            control: Point::new(cx, cy),
            end: Point::new(x, y),
        });
    }

    /// Draw a cubic Bézier curve.
    pub fn cubic_to(&mut self, c1x: f64, c1y: f64, c2x: f64, c2y: f64, x: f64, y: f64) {
        self.commands.push(PathCommand::CubicTo {
            control1: Point::new(c1x, c1y),
            control2: Point::new(c2x, c2y),
            end: Point::new(x, y),
        });
    }

    /// Close the current subpath.
    pub fn close(&mut self) {
        self.commands.push(PathCommand::Close);
    }

    /// Whether the path has no commands.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// The bounding box of the path's control points, or `None` if empty.
    pub fn bounds(&self) -> Option<Rect> {
        if self.commands.is_empty() {
            return None;
        }
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for cmd in &self.commands {
            let points: &[Point] = match cmd {
                PathCommand::MoveTo(p) | PathCommand::LineTo(p) => std::slice::from_ref(p),
                PathCommand::QuadTo { control, end } => &[*control, *end],
                PathCommand::CubicTo {
                    control1,
                    control2,
                    end,
                } => &[*control1, *control2, *end],
                PathCommand::Close => &[],
            };
            for p in points {
                min_x = min_x.min(p.x);
                min_y = min_y.min(p.y);
                max_x = max_x.max(p.x);
                max_y = max_y.max(p.y);
            }
        }
        if min_x == f64::MAX {
            None
        } else {
            Some(Rect::new(min_x, min_y, max_x - min_x, max_y - min_y))
        }
    }

    /// Anchor points of the path (start and curve end points), in order.
    pub fn anchor_points(&self) -> Vec<Point> {
        let mut out = Vec::new();
        for cmd in &self.commands {
            match cmd {
                PathCommand::MoveTo(p) | PathCommand::LineTo(p) => out.push(*p),
                PathCommand::QuadTo { end, .. } => out.push(*end),
                PathCommand::CubicTo { end, .. } => out.push(*end),
                PathCommand::Close => {}
            }
        }
        out
    }

    /// Move the anchor point at `index` to `p`, taking the control handles that hang from it
    /// along. Returns whether it existed.
    pub fn move_anchor_point(&mut self, index: usize, p: Point) -> bool {
        let Some(i) = self.anchor_command(index) else {
            return false;
        };
        let old = match &self.commands[i] {
            PathCommand::MoveTo(q) | PathCommand::LineTo(q) => *q,
            PathCommand::QuadTo { end, .. } | PathCommand::CubicTo { end, .. } => *end,
            PathCommand::Close => return false,
        };
        let (dx, dy) = (p.x - old.x, p.y - old.y);
        let shift = |q: &mut Point| {
            q.x += dx;
            q.y += dy;
        };
        match &mut self.commands[i] {
            PathCommand::MoveTo(q) | PathCommand::LineTo(q) => *q = p,
            PathCommand::QuadTo { end, .. } => *end = p,
            PathCommand::CubicTo { control2, end, .. } => {
                shift(control2);
                *end = p;
            }
            PathCommand::Close => {}
        }
        match self.commands.get_mut(i + 1) {
            Some(PathCommand::CubicTo { control1, .. }) => shift(control1),
            Some(PathCommand::QuadTo { control, .. }) => shift(control),
            _ => {}
        }
        true
    }
}

/// A fluent builder for [`Path`].
pub struct PathBuilder {
    path: Path,
}

impl PathBuilder {
    /// Create a new path builder.
    pub fn new() -> Self {
        Self { path: Path::new() }
    }

    /// Move to a point.
    pub fn move_to(mut self, x: f64, y: f64) -> Self {
        self.path.move_to(x, y);
        self
    }

    /// Draw a line to a point.
    pub fn line_to(mut self, x: f64, y: f64) -> Self {
        self.path.line_to(x, y);
        self
    }

    /// Draw a quadratic Bézier curve.
    pub fn quad_to(mut self, cx: f64, cy: f64, x: f64, y: f64) -> Self {
        self.path.quad_to(cx, cy, x, y);
        self
    }

    /// Draw a cubic Bézier curve.
    pub fn cubic_to(mut self, c1x: f64, c1y: f64, c2x: f64, c2y: f64, x: f64, y: f64) -> Self {
        self.path.cubic_to(c1x, c1y, c2x, c2y, x, y);
        self
    }

    /// Close the current subpath.
    pub fn close(mut self) -> Self {
        self.path.close();
        self
    }

    /// Finish building the path.
    pub fn build(self) -> Path {
        self.path
    }
}

impl Default for PathBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// The geometry of an auto shape: a primitive outline or a custom path.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeGeometry {
    /// Rectangle with optional corner radius.
    Rectangle {
        /// Corner radius (0 for sharp corners).
        corner_radius: f64,
    },
    /// Ellipse (a circle when width equals height).
    Ellipse,
    /// Regular polygon.
    Polygon {
        /// Number of sides (3 = triangle, 5 = pentagon, …).
        sides: u32,
    },
    /// Star shape.
    Star {
        /// Number of points.
        points: u32,
        /// Inner radius as a ratio of the outer radius (0–1).
        inner_ratio: f64,
    },
    /// A custom Bézier path.
    Path(Path),
}

impl Default for ShapeGeometry {
    fn default() -> Self {
        ShapeGeometry::Rectangle { corner_radius: 0.0 }
    }
}

impl ShapeGeometry {
    /// Whether this geometry is already an editable path.
    pub fn is_path(&self) -> bool {
        matches!(self, ShapeGeometry::Path(_))
    }

    /// Convert the geometry to an editable path for the given `bounds`.
    ///
    /// Primitive shapes are approximated: rectangles use their corners,
    /// ellipses four cubic segments, and polygons/stars their vertices.
    pub fn to_editable_path(&self, bounds: Rect) -> Path {
        if let ShapeGeometry::Path(path) = self {
            return path.clone();
        }
        let (x0, y0) = (bounds.origin.x, bounds.origin.y);
        let (x1, y1) = (x0 + bounds.size.width, y0 + bounds.size.height);
        let cx = (x0 + x1) / 2.0;
        let cy = (y0 + y1) / 2.0;
        let rx = (x1 - x0) / 2.0;
        let ry = (y1 - y0) / 2.0;
        let mut path = Path::new();
        match self {
            ShapeGeometry::Rectangle { .. } => {
                path.move_to(x0, y0);
                path.line_to(x1, y0);
                path.line_to(x1, y1);
                path.line_to(x0, y1);
                path.close();
            }
            ShapeGeometry::Ellipse => {
                const K: f64 = 0.552_284_749_830_793_6;
                path.move_to(cx + rx, cy);
                path.cubic_to(cx + rx, cy + K * ry, cx + K * rx, cy + ry, cx, cy + ry);
                path.cubic_to(cx - K * rx, cy + ry, cx - rx, cy + K * ry, cx - rx, cy);
                path.cubic_to(cx - rx, cy - K * ry, cx - K * rx, cy - ry, cx, cy - ry);
                path.cubic_to(cx + K * rx, cy - ry, cx + rx, cy - K * ry, cx + rx, cy);
                path.close();
            }
            ShapeGeometry::Polygon { sides } => {
                let n = (*sides).max(3) as f64;
                for i in 0..(*sides).max(3) {
                    let a = -std::f64::consts::FRAC_PI_2 + i as f64 * std::f64::consts::TAU / n;
                    let (px, py) = (cx + rx * a.cos(), cy + ry * a.sin());
                    if i == 0 {
                        path.move_to(px, py);
                    } else {
                        path.line_to(px, py);
                    }
                }
                path.close();
            }
            ShapeGeometry::Star {
                points,
                inner_ratio,
            } => {
                let n = (*points).max(3);
                let total = n * 2;
                for i in 0..total {
                    let ratio = if i % 2 == 0 { 1.0 } else { *inner_ratio };
                    let a = -std::f64::consts::FRAC_PI_2
                        + i as f64 * std::f64::consts::TAU / total as f64;
                    let (px, py) = (cx + rx * ratio * a.cos(), cy + ry * ratio * a.sin());
                    if i == 0 {
                        path.move_to(px, py);
                    } else {
                        path.line_to(px, py);
                    }
                }
                path.close();
            }
            ShapeGeometry::Path(_) => unreachable!(),
        }
        path
    }
}

/// The payload for an auto shape: its geometry and optional Office preset.
///
/// When [`AutoShape::preset`] is set, [`AutoShape::geometry`] holds the
/// preset's outline for the current size and adjust values. The guide engine
/// that rebuilds the outline lives in the `repub-escher` crate.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct AutoShape {
    /// The shape's outline geometry.
    pub geometry: ShapeGeometry,
    /// The Office preset this shape was built from, if any.
    pub preset: Option<PresetGeometry>,
}

impl AutoShape {
    /// Create an auto shape with the given geometry and no preset.
    pub fn new(geometry: ShapeGeometry) -> Self {
        Self {
            geometry,
            preset: None,
        }
    }

    /// Whether this auto shape is already an editable path.
    pub fn is_path(&self) -> bool {
        self.geometry.is_path()
    }

    /// Convert the geometry to an editable path for the given `bounds`.
    pub fn to_editable_path(&self, bounds: Rect) -> Path {
        self.geometry.to_editable_path(bounds)
    }

    /// Convert this auto shape's geometry to an editable path in place, using
    /// `bounds` to approximate primitives.
    pub fn convert_to_path(&mut self, bounds: Rect) {
        if !self.geometry.is_path() {
            self.geometry = ShapeGeometry::Path(self.geometry.to_editable_path(bounds));
        }
    }

    /// Anchor points of the editable path (approximating primitives for `bounds`).
    pub fn anchor_points(&self, bounds: Rect) -> Vec<Point> {
        self.geometry.to_editable_path(bounds).anchor_points()
    }

    /// Move an anchor point of the editable path. Converts primitives first.
    pub fn move_anchor_point(&mut self, bounds: Rect, index: usize, p: Point) -> bool {
        self.convert_to_path(bounds);
        if let ShapeGeometry::Path(path) = &mut self.geometry {
            return path.move_anchor_point(index, p);
        }
        false
    }
}

/// A shape built from an Office preset: its template type and adjust values.
///
/// This names one of Office's legacy ("Escher") shape templates and the
/// adjust values that parameterize it. It is the model's plain-data reference
/// to a preset; the guide-evaluation engine that turns it into an outline,
/// adjust handles, text rectangle and connection sites lives in the
/// `repub-escher` crate, which operates on this type.
///
/// `adjust` holds per-index overrides; `None` (or a missing entry) uses the
/// preset's own default for that index.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PresetGeometry {
    /// Escher shape type (MSOSPT, 1..=202).
    pub spt: u16,
    /// Adjust-value overrides by index.
    pub adjust: Vec<Option<i32>>,
}

impl PresetGeometry {
    /// A preset of type `spt` using its default adjust values.
    pub fn new(spt: u16) -> Self {
        Self {
            spt,
            adjust: Vec::new(),
        }
    }

    /// The override for adjust value `index`, if one is set.
    pub fn adjust_override(&self, index: usize) -> Option<i32> {
        self.adjust.get(index).copied().flatten()
    }

    /// Set an adjust-value override, growing the list as needed.
    pub fn set_adjust(&mut self, index: usize, value: i32) {
        if self.adjust.len() <= index {
            self.adjust.resize(index + 1, None);
        }
        self.adjust[index] = Some(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rectangle_editable_path_has_four_corners() {
        let geo = ShapeGeometry::Rectangle { corner_radius: 0.0 };
        let path = geo.to_editable_path(Rect::new(0.0, 0.0, 10.0, 20.0));
        assert_eq!(path.anchor_points().len(), 4);
    }

    #[test]
    fn path_anchor_move() {
        let mut path = Path::new();
        path.move_to(0.0, 0.0);
        path.line_to(5.0, 5.0);
        assert!(path.move_anchor_point(1, Point::new(9.0, 9.0)));
        assert_eq!(path.anchor_points()[1], Point::new(9.0, 9.0));
        assert!(!path.move_anchor_point(9, Point::new(0.0, 0.0)));
    }

    #[test]
    fn builder_builds() {
        let path = Path::builder()
            .move_to(0.0, 0.0)
            .line_to(1.0, 1.0)
            .close()
            .build();
        assert_eq!(path.commands.len(), 3);
    }
}
