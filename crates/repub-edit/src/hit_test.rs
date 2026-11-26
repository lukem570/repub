//! Which shape a click on the canvas means (document-tree selection).
//!
//! Clicks favour what the user can see. Each shape is tested in two strengths:
//!
//! - **Strong** hits are on visible content: the inside of a filled shape, the
//!   outline of any shape, a line, a picture or table, or a filled text box.
//! - **Weak** hits are on empty space: the inside of an unfilled shape, or a
//!   transparent text box.
//!
//! The top-most strong hit wins; only when nothing is hit strongly does the
//! top-most weak hit win. So an empty bordered box in front of a text box lets
//! a click on content through, but is still selectable when clicked inside
//! over blank page.
//!
//! Every test allows `tolerance` document points of slack, so clicks near an
//! edge or a thin line still count.
//!
//! This is a different concern from the geometric hit-testing in `repub-hit`:
//! here we resolve the document tree's strong/weak selection semantics against
//! the [`Shape`]/[`ShapeKind`] model, not a resolved render scene. Text-line
//! band testing, which needs a laid-out frame, is a rendering concern and is
//! left to the GUI; an unfilled text box is treated as a weak hit here.

use repub_geometry::{Point, Rect};
use repub_model::geometry::{PathCommand, ShapeGeometry};
use repub_model::id::ObjectId;
use repub_model::shape::{GroupData, Shape, ShapeKind};
use repub_model::styles::Fill;
use repub_model::{Layer, Page};

/// Screen pixels of slack around shapes when clicking.
pub const HIT_TOLERANCE_PX: f32 = 5.0;

/// How strongly a point hits a shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Hit {
    /// On empty space that belongs to the shape.
    Weak,
    /// On the shape's visible content.
    Strong,
}

/// The shape a click at `pos` (document points) selects on `page`, if any.
/// `tolerance` is in document points (screen slack divided by zoom).
pub fn shape_at(page: &Page, pos: Point, tolerance: f64) -> Option<ObjectId> {
    let mut weak = None;
    for layer in page.layers().iter().rev() {
        if !layer.visible {
            continue;
        }
        for shape in layer.shapes().iter().rev() {
            match hit(shape, pos, tolerance) {
                Some(Hit::Strong) => return Some(shape.id()),
                Some(Hit::Weak) if weak.is_none() => weak = Some(shape.id()),
                _ => {}
            }
        }
    }
    weak
}

/// The shape a click selects, searching all layers of `page` (kept for API
/// symmetry with [`shape_at`]).
pub fn shape_at_in_layers(layers: &[Layer], pos: Point, tolerance: f64) -> Option<ObjectId> {
    let mut weak = None;
    for layer in layers.iter().rev() {
        if !layer.visible {
            continue;
        }
        for shape in layer.shapes().iter().rev() {
            match hit(shape, pos, tolerance) {
                Some(Hit::Strong) => return Some(shape.id()),
                Some(Hit::Weak) if weak.is_none() => weak = Some(shape.id()),
                _ => {}
            }
        }
    }
    weak
}

/// The member of `group` a click at `pos` lands on (visible content first, as
/// with [`shape_at`]).
pub fn member_at(group: &GroupData, pos: Point, tolerance: f64) -> Option<&Shape> {
    let hits: Vec<(&Shape, Hit)> = group
        .children
        .iter()
        .rev()
        .filter_map(|c| hit(c, pos, tolerance).map(|h| (c, h)))
        .collect();
    hits.iter()
        .find(|(_, h)| *h == Hit::Strong)
        .or(hits.first())
        .map(|(c, _)| *c)
}

/// How `pos` hits `shape`, if at all.
pub fn hit(shape: &Shape, pos: Point, tolerance: f64) -> Option<Hit> {
    if !shape.common.visible {
        return None;
    }
    match &shape.kind {
        ShapeKind::AutoShape(_) => hit_auto_shape(shape, pos, tolerance),
        ShapeKind::Line(line) => {
            let reach = tolerance + shape.line.as_ref().map_or(0.0, |s| s.width) / 2.0;
            (distance_to_segment(pos, line.start, line.end) <= reach).then_some(Hit::Strong)
        }
        ShapeKind::TextFrame(_) => hit_text_frame(shape, pos, tolerance),
        // A group is hit wherever one of its members is
        ShapeKind::Group(group) => group
            .children
            .iter()
            .filter_map(|c| hit(c, pos, tolerance))
            .max(),
        // Pictures, tables, text effects are solid rectangles
        _ => {
            let local = unrotate(pos, shape);
            inflated_contains(shape.bounds(), local, tolerance).then_some(Hit::Strong)
        }
    }
}

fn hit_auto_shape(shape: &Shape, pos: Point, tolerance: f64) -> Option<Hit> {
    let ShapeKind::AutoShape(auto) = &shape.kind else {
        return None;
    };
    let stroke = shape.line.as_ref().map_or(0.0, |s| s.width.max(0.0));
    let local = unrotate(pos, shape);

    if let ShapeGeometry::Path(path) = &auto.geometry {
        // Subpath by subpath: near an outlined one, or inside the filled area
        // (the path's winding rule, so a donut's hole is empty).
        let subpaths = polyline_subpaths(path);
        let near = subpaths.iter().filter(|sp| !sp.no_stroke).any(|sp| {
            distance_to_polyline(local, &sp.points, sp.closed) <= tolerance + stroke / 2.0
        });
        if near {
            return Some(Hit::Strong);
        }
        let filled: Vec<&Vec<Point>> = subpaths
            .iter()
            .filter(|sp| !sp.no_fill)
            .map(|sp| &sp.points)
            .collect();
        let winding: i32 = filled.iter().map(|pts| winding_number(local, pts)).sum();
        let crossings: usize = filled
            .iter()
            .filter(|pts| point_in_polygon(local, pts))
            .count();
        let inside = if path.even_odd {
            crossings % 2 == 1
        } else {
            winding != 0
        };
        if inside {
            return Some(if is_filled(&shape.fill) {
                Hit::Strong
            } else {
                Hit::Weak
            });
        }
        return None;
    }

    // A primitive outline: approximate it as a polygon.
    let outline = auto.to_editable_path(shape.bounds()).anchor_points();
    if outline.len() < 2 {
        return None;
    }
    if distance_to_polyline(local, &outline, true) <= tolerance + stroke / 2.0 {
        return Some(Hit::Strong);
    }
    if point_in_polygon(local, &outline) {
        return Some(if is_filled(&shape.fill) {
            Hit::Strong
        } else {
            Hit::Weak
        });
    }
    None
}

/// One subpath flattened to a polyline, with its drawing flags.
struct SubpathLine {
    points: Vec<Point>,
    closed: bool,
    no_fill: bool,
    no_stroke: bool,
}

/// Each subpath of `path` flattened to a polyline (control points included as
/// an approximation of the curve), with its per-subpath drawing flags.
fn polyline_subpaths(path: &repub_model::geometry::Path) -> Vec<SubpathLine> {
    let mut out: Vec<SubpathLine> = Vec::new();
    let mut current: Vec<Point> = Vec::new();
    let mut closed = false;
    let mut index = 0;
    for cmd in &path.commands {
        match cmd {
            PathCommand::MoveTo(p) => {
                if !current.is_empty() {
                    let style = path.subpath_style(index);
                    out.push(SubpathLine {
                        points: std::mem::take(&mut current),
                        closed,
                        no_fill: style.no_fill,
                        no_stroke: style.no_stroke,
                    });
                    index += 1;
                    closed = false;
                }
                current.push(*p);
            }
            PathCommand::LineTo(p) => current.push(*p),
            PathCommand::QuadTo { control, end } => {
                current.push(*control);
                current.push(*end);
            }
            PathCommand::CubicTo {
                control1,
                control2,
                end,
            } => {
                current.push(*control1);
                current.push(*control2);
                current.push(*end);
            }
            PathCommand::Close => closed = true,
        }
    }
    if !current.is_empty() {
        let style = path.subpath_style(index);
        out.push(SubpathLine {
            points: current,
            closed,
            no_fill: style.no_fill,
            no_stroke: style.no_stroke,
        });
    }
    out
}

/// Winding number of the closed polygon `points` around `p`.
fn winding_number(p: Point, points: &[Point]) -> i32 {
    let mut wn = 0;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[(i + 1) % points.len()]);
        let cross = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
        if a.y <= p.y {
            if b.y > p.y && cross > 0.0 {
                wn += 1;
            }
        } else if b.y <= p.y && cross < 0.0 {
            wn -= 1;
        }
    }
    wn
}

fn hit_text_frame(shape: &Shape, pos: Point, tolerance: f64) -> Option<Hit> {
    let local = unrotate(pos, shape);
    let inside = inflated_contains(shape.bounds(), local, tolerance);
    if !inside {
        return None;
    }
    if is_filled(&shape.fill) {
        return Some(Hit::Strong);
    }
    // A laid-out text-line band would make a click on the text a strong hit;
    // that needs text layout and is handled by the renderer. An unfilled text
    // box over blank page is a weak hit.
    Some(Hit::Weak)
}

/// Whether a fill paints anything.
fn is_filled(fill: &Fill) -> bool {
    match fill {
        Fill::None => false,
        Fill::Solid(c) => c.a > 0.0,
        _ => true,
    }
}

/// `pos` rotated into `shape`'s unrotated frame (about its bounds' center).
fn unrotate(pos: Point, shape: &Shape) -> Point {
    let rotation = shape.common.rotation;
    if rotation == 0.0 {
        return pos;
    }
    let c = shape.bounds().center();
    let (sin, cos) = (-rotation).to_radians().sin_cos();
    let (dx, dy) = (pos.x - c.x, pos.y - c.y);
    Point::new(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos)
}

fn inflated_contains(r: Rect, p: Point, tolerance: f64) -> bool {
    p.x >= r.origin.x - tolerance
        && p.x <= r.origin.x + r.size.width + tolerance
        && p.y >= r.origin.y - tolerance
        && p.y <= r.origin.y + r.size.height + tolerance
}

fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len_sq = dx * dx + dy * dy;
    let t = if len_sq > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (cx, cy) = (a.x + t * dx, a.y + t * dy);
    ((p.x - cx).powi(2) + (p.y - cy).powi(2)).sqrt()
}

fn distance_to_polyline(p: Point, points: &[Point], closed: bool) -> f64 {
    if points.is_empty() {
        return f64::INFINITY;
    }
    let open = points
        .windows(2)
        .map(|w| distance_to_segment(p, w[0], w[1]));
    let closing = closed.then(|| distance_to_segment(p, points[points.len() - 1], points[0]));
    open.chain(closing).fold(f64::INFINITY, f64::min)
}

/// Even-odd point-in-polygon test.
fn point_in_polygon(p: Point, points: &[Point]) -> bool {
    if points.len() < 3 {
        return false;
    }
    let mut inside = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (a, b) = (points[i], points[j]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
        j = i;
    }
    inside
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_color::Color;
    use repub_geometry::Rect;
    use repub_model::shape::{Shape, ShapeKind};
    use repub_model::styles::Fill;
    use repub_model::text::{Paragraph, StyledText};
    use repub_model::{Document, PageOrientation, PageSize};

    const TOL: f64 = 5.0;

    fn page_with(shapes: Vec<Shape>) -> Page {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        let page = &mut doc.pages_mut()[0];
        for s in shapes {
            page.add_shape(s);
        }
        page.clone()
    }

    fn empty_box(r: Rect) -> Shape {
        let mut s = Shape::rectangle(r);
        s.fill = Fill::None;
        s
    }

    fn filled_box(r: Rect) -> Shape {
        let mut s = Shape::rectangle(r);
        s.fill = Fill::Solid(Color::from_rgb8(255, 0, 0));
        s
    }

    fn text(r: Rect, s: &str) -> Shape {
        let mut tf = Shape::text_frame(r);
        if let ShapeKind::TextFrame(data) = &mut tf.kind {
            data.content = StyledText {
                paragraphs: vec![Paragraph::plain(s)],
            };
        }
        tf.fill = Fill::None;
        tf
    }

    #[test]
    fn empty_box_lets_clicks_on_a_filled_shape_behind_through() {
        // A filled shape behind, an unfilled box in front: clicking inside the
        // box over the filled shape selects the shape (strong beats weak).
        let shape = filled_box(Rect::new(100.0, 100.0, 300.0, 300.0));
        let bx = empty_box(Rect::new(50.0, 50.0, 400.0, 400.0));
        let (shape_id, box_id) = (shape.id(), bx.id());
        let page = page_with(vec![shape, bx]);
        assert_eq!(
            shape_at(&page, Point::new(200.0, 200.0), TOL),
            Some(shape_id)
        );
        // On the box's border: strong hit on the box
        assert_eq!(shape_at(&page, Point::new(50.0, 200.0), TOL), Some(box_id));
        // Inside the box over blank page (below the filled shape): the box
        assert_eq!(shape_at(&page, Point::new(60.0, 430.0), TOL), Some(box_id));
    }

    #[test]
    fn filled_box_in_front_still_covers_text() {
        let tf = text(Rect::new(100.0, 100.0, 300.0, 200.0), "Hello world");
        let bx = filled_box(Rect::new(50.0, 50.0, 400.0, 300.0));
        let box_id = bx.id();
        let page = page_with(vec![tf, bx]);
        assert_eq!(shape_at(&page, Point::new(110.0, 110.0), TOL), Some(box_id));
    }

    #[test]
    fn empty_text_area_loses_to_strong_hits_behind() {
        let shape = filled_box(Rect::new(100.0, 100.0, 300.0, 300.0));
        let tf = text(Rect::new(100.0, 100.0, 300.0, 300.0), "Hi");
        let shape_id = shape.id();
        let page = page_with(vec![shape, tf]);
        assert_eq!(
            shape_at(&page, Point::new(250.0, 350.0), TOL),
            Some(shape_id)
        );
        // Blank page inside an unfilled text box with nothing behind: the box
        let page = page_with(vec![text(Rect::new(100.0, 100.0, 300.0, 300.0), "Hi")]);
        assert!(shape_at(&page, Point::new(250.0, 350.0), TOL).is_some());
    }

    #[test]
    fn clicks_near_edges_count() {
        let bx = filled_box(Rect::new(100.0, 100.0, 50.0, 50.0));
        let id = bx.id();
        let page = page_with(vec![bx]);
        assert_eq!(shape_at(&page, Point::new(97.0, 120.0), TOL), Some(id));
        assert_eq!(shape_at(&page, Point::new(90.0, 120.0), TOL), None);
    }

    #[test]
    fn thin_lines_are_easy_to_hit() {
        let line = Shape::line(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        let id = line.id();
        let page = page_with(vec![line]);
        assert_eq!(shape_at(&page, Point::new(52.0, 48.0), TOL), Some(id));
        assert_eq!(shape_at(&page, Point::new(80.0, 20.0), TOL), None);
    }

    #[test]
    fn unfilled_ellipse_interior_is_weak_and_corners_miss() {
        let mut s = Shape::ellipse(Rect::new(0.0, 0.0, 100.0, 100.0));
        s.fill = Fill::None;
        assert_eq!(hit(&s, Point::new(50.0, 50.0), TOL), Some(Hit::Weak));
        assert_eq!(hit(&s, Point::new(50.0, 1.0), TOL), Some(Hit::Strong));
        // Bounding-box corner is outside the ellipse
        assert_eq!(hit(&s, Point::new(3.0, 3.0), TOL), None);
    }
}
