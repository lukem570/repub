//! Which object a click on the canvas means.
//!
//! Clicks favour what the user can see. Each object is tested in two strengths:
//!
//! - **Strong** hits are on visible content: the inside of a filled shape, the
//!   outline of any shape, a line, an image or table, a filled text box, or the
//!   lines of text inside a text box (not just the glyphs: the whole line band).
//! - **Weak** hits are on empty space: the inside of an unfilled shape, or a
//!   transparent text box away from its text.
//!
//! The top-most strong hit wins; only when nothing is hit strongly does the
//! top-most weak hit win. So an empty bordered box in front of a text box lets
//! a click on the text through, but is still selectable when clicked inside
//! over blank page.
//!
//! Every test allows `tolerance` document points of slack, so clicks near an
//! edge, a thin line or a line of text still count.

use repub_geometry::Point;
use repub_model::geometry::ShapeGeometry;
use repub_model::shape::{GroupData, Shape, ShapeKind};
use repub_model::styles::Fill;
use repub_model::ObjectId;
use repub_model::Page;

/// Screen pixels of slack around objects when clicking.
pub const HIT_TOLERANCE_PX: f32 = 5.0;

/// How strongly a point hits an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Hit {
    /// On empty space that belongs to the object.
    Weak,
    /// On the object's visible content.
    Strong,
}

/// The object a click at `pos` (document points) selects on `page`, if any.
/// `tolerance` is in document points (screen slack divided by zoom).
pub fn object_at(page: &Page, pos: Point, tolerance: f64) -> Option<ObjectId> {
    let mut weak = None;
    for layer in page.layers().iter().rev() {
        if !layer.visible {
            continue;
        }
        for obj in layer.shapes().iter().rev() {
            match hit(obj, pos, tolerance) {
                Some(Hit::Strong) => return Some(obj.id()),
                Some(Hit::Weak) if weak.is_none() => weak = Some(obj.id()),
                _ => {}
            }
        }
    }
    weak
}

/// The member of `group` a click at `pos` lands on (visible content first, as
/// with [`object_at`]).
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

/// How `pos` hits `obj`, if at all.
pub fn hit(obj: &Shape, pos: Point, tolerance: f64) -> Option<Hit> {
    if !obj.common.visible {
        return None;
    }
    match &obj.kind {
        ShapeKind::AutoShape(_) => hit_shape(obj, pos, tolerance),
        ShapeKind::Line(line) => {
            let width = obj.line.as_ref().map_or(0.0, |s| s.width);
            let reach = tolerance + width / 2.0;
            (distance_to_polyline(unrotate(pos, obj), &line_polyline(line), false) <= reach)
                .then_some(Hit::Strong)
        }
        ShapeKind::TextFrame(_) => hit_text_frame(obj, pos, tolerance),
        // A group is hit wherever one of its members is
        ShapeKind::Group(group) => group
            .children
            .iter()
            .filter_map(|c| hit(c, pos, tolerance))
            .max(),
        // Images, tables and the rest are solid rectangles
        _ => {
            let local = unrotate(pos, obj);
            inflated_contains(obj.bounds(), local, tolerance).then_some(Hit::Strong)
        }
    }
}

fn hit_shape(shape: &Shape, pos: Point, tolerance: f64) -> Option<Hit> {
    let stroke = shape.line.as_ref().map_or(0.0, |s| s.width.max(0.0));
    let geometry = match &shape.kind {
        ShapeKind::AutoShape(auto) => &auto.geometry,
        _ => return None,
    };
    if let ShapeGeometry::Path(path) = geometry {
        // Subpath by subpath: near an outlined one, or inside the filled area
        // (the path's winding rule, so a donut's hole is empty)
        let subpaths = crate::canvas::shape_subpaths(shape, 1.0);
        let near = subpaths
            .iter()
            .filter(|(_, _, style)| !style.no_stroke)
            .any(|(pts, closed, _)| {
                distance_to_polyline(pos, pts, *closed) <= tolerance + stroke / 2.0
            });
        if near {
            return Some(Hit::Strong);
        }
        let filled: Vec<&Vec<Point>> = subpaths
            .iter()
            .filter(|(_, _, s)| !s.no_fill)
            .map(|(p, _, _)| p)
            .collect();
        let winding: i32 = filled.iter().map(|pts| winding_number(pos, pts)).sum();
        let crossings: usize = filled
            .iter()
            .filter(|pts| point_in_polygon(pos, pts))
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
    let outline = crate::canvas::shape_outline(shape, 1.0);
    if outline.len() < 2 {
        return None;
    }
    if distance_to_polyline(pos, &outline, true) <= tolerance + stroke / 2.0 {
        return Some(Hit::Strong);
    }
    if point_in_polygon(pos, &outline) {
        return Some(if is_filled(&shape.fill) {
            Hit::Strong
        } else {
            Hit::Weak
        });
    }
    None
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

fn hit_text_frame(tf: &Shape, pos: Point, tolerance: f64) -> Option<Hit> {
    let ShapeKind::TextFrame(data) = &tf.kind else {
        return None;
    };
    // Text layout is in the frame's unrotated space
    let (fx, fy) = crate::text_edit::to_frame_space(tf, pos.x as f32, pos.y as f32);
    let local = Point::new(fx as f64, fy as f64);
    let inside = inflated_contains(tf.common.bounds, local, tolerance);
    if !inside {
        return None;
    }
    if is_filled(&tf.fill) {
        return Some(Hit::Strong);
    }
    // Near a line of text: the band from the first to the last character, so
    // clicks between and around glyphs count, not only on their ink
    if !data.content.plain_text().trim().is_empty() {
        let layout = crate::sdf_text::layout_frame(tf);
        let tol = tolerance as f32;
        let on_text = layout.lines.iter().filter(|l| l.end > l.start).any(|line| {
            let (min_x, max_x) = line
                .xs
                .iter()
                .fold((f32::MAX, f32::MIN), |(a, b), &x| (a.min(x), b.max(x)));
            fx >= min_x - tol
                && fx <= max_x + tol
                && fy >= line.top - tol
                && fy <= line.bottom + tol
        });
        if on_text {
            return Some(Hit::Strong);
        }
    }
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

/// `pos` rotated into `obj`'s unrotated frame (about its bounds' center).
fn unrotate(pos: Point, obj: &Shape) -> Point {
    let rotation = obj.common.rotation;
    if rotation == 0.0 {
        return pos;
    }
    let c = obj.bounds().center();
    let (sin, cos) = (-rotation).to_radians().sin_cos();
    let (dx, dy) = (pos.x - c.x, pos.y - c.y);
    Point::new(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos)
}

fn inflated_contains(r: repub_geometry::Rect, p: Point, tolerance: f64) -> bool {
    p.x >= r.origin.x - tolerance
        && p.x <= r.origin.x + r.size.width + tolerance
        && p.y >= r.origin.y - tolerance
        && p.y <= r.origin.y + r.size.height + tolerance
}

/// The points a line (or connector) runs through, in page space before the shape's rotation:
/// straight, with one elbow pair, or a smooth S-curve (through its control points if it has any).
pub fn line_polyline(line: &repub_model::shape::LineData) -> Vec<Point> {
    use repub_model::shape::ConnectorType;
    let (a, b) = (line.start, line.end);
    let mid_x = (a.x + b.x) / 2.0;
    match line.connector_type {
        ConnectorType::Straight => vec![a, b],
        ConnectorType::Elbow => vec![a, Point::new(mid_x, a.y), Point::new(mid_x, b.y), b],
        ConnectorType::Curved => {
            let (c1, c2) = match line.control_points.as_slice() {
                [] => (Point::new(mid_x, a.y), Point::new(mid_x, b.y)),
                [c] => (*c, *c),
                [c1, c2, ..] => (*c1, *c2),
            };
            const STEPS: usize = 24;
            (0..=STEPS)
                .map(|i| {
                    let t = i as f64 / STEPS as f64;
                    let u = 1.0 - t;
                    let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                    Point::new(
                        w0 * a.x + w1 * c1.x + w2 * c2.x + w3 * b.x,
                        w0 * a.y + w1 * c1.y + w2 * c2.y + w3 * b.y,
                    )
                })
                .collect()
        }
    }
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
    let open = points
        .windows(2)
        .map(|w| distance_to_segment(p, w[0], w[1]));
    let closing = closed.then(|| distance_to_segment(p, points[points.len() - 1], points[0]));
    open.chain(closing).fold(f64::INFINITY, f64::min)
}

/// Even-odd point-in-polygon test.
fn point_in_polygon(p: Point, points: &[Point]) -> bool {
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
    use repub_model::shape::{Shape, ShapeKind, TextFrameData};
    use repub_model::text::{Paragraph, StyledText};
    use repub_model::Document;
    use repub_model::{PageOrientation, PageSize};

    const TOL: f64 = 5.0;

    fn page_with(objects: Vec<Shape>) -> Page {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        let page = &mut doc.pages_mut()[0];
        for o in objects {
            page.add_shape(o);
        }
        page.clone()
    }

    #[test]
    fn connectors_are_hit_along_their_route() {
        use repub_model::shape::ConnectorType;
        let mut elbow = Shape::line(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        if let ShapeKind::Line(line) = &mut elbow.kind {
            line.connector_type = ConnectorType::Elbow;
            assert_eq!(line_polyline(line).len(), 4);
        }
        // The elbow runs along y=0 to x=50, down to y=100, then across: the straight diagonal is empty
        assert!(hit(&elbow, Point::new(25.0, 0.0), TOL).is_some());
        assert!(hit(&elbow, Point::new(50.0, 60.0), TOL).is_some());
        assert!(hit(&elbow, Point::new(25.0, 25.0), TOL).is_none());
        let mut curved = Shape::line(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        if let ShapeKind::Line(line) = &mut curved.kind {
            line.connector_type = ConnectorType::Curved;
            let points = line_polyline(line);
            assert_eq!(points.first(), Some(&Point::new(0.0, 0.0)));
            assert_eq!(points.last(), Some(&Point::new(100.0, 100.0)));
        }
    }

    #[test]
    fn rotated_line_is_hit_where_it_is_drawn() {
        // A horizontal line turned 90 degrees about its center runs vertically
        let mut line = Shape::line(Point::new(0.0, 100.0), Point::new(100.0, 100.0));
        line.common.rotation = 90.0;
        assert!(hit(&line, Point::new(50.0, 60.0), TOL).is_some());
        assert!(hit(&line, Point::new(10.0, 100.0), TOL).is_none());
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
        let mut data = TextFrameData::new();
        data.content = StyledText {
            paragraphs: vec![Paragraph::plain(s)],
        };
        Shape::new(r, ShapeKind::TextFrame(data))
    }

    #[test]
    fn click_on_text_goes_through_an_empty_box_in_front() {
        let tf = text(Rect::new(100.0, 100.0, 300.0, 200.0), "Hello world");
        let bx = empty_box(Rect::new(50.0, 50.0, 400.0, 300.0));
        let (tf_id, box_id) = (tf.id(), bx.id());
        let page = page_with(vec![tf, bx]);
        // On the first line of text
        let layout = crate::sdf_text::layout_frame(match page.get_shape(tf_id) {
            Some(shape) if matches!(shape.kind, ShapeKind::TextFrame(_)) => shape,
            _ => unreachable!(),
        });
        let line = &layout.lines[0];
        let on_text = Point::new(line.xs[2] as f64, ((line.top + line.bottom) / 2.0) as f64);
        assert_eq!(object_at(&page, on_text, TOL), Some(tf_id));
        // Just above the text line, within tolerance, still the text
        assert_eq!(
            object_at(&page, Point::new(on_text.x, line.top as f64 - 3.0), TOL),
            Some(tf_id)
        );
        // On the box's border
        assert_eq!(object_at(&page, Point::new(50.0, 200.0), TOL), Some(box_id));
        // Inside the box over blank page: the box (weak hit, nothing else there)
        assert_eq!(object_at(&page, Point::new(70.0, 330.0), TOL), Some(box_id));
    }

    #[test]
    fn filled_box_in_front_still_covers_text() {
        let tf = text(Rect::new(100.0, 100.0, 300.0, 200.0), "Hello world");
        let bx = filled_box(Rect::new(50.0, 50.0, 400.0, 300.0));
        let box_id = bx.id();
        let page = page_with(vec![tf, bx]);
        assert_eq!(
            object_at(&page, Point::new(110.0, 110.0), TOL),
            Some(box_id)
        );
    }

    #[test]
    fn empty_text_area_loses_to_strong_hits_behind() {
        // A transparent text box over a filled shape: clicking blank space in the
        // text box (far from its one short line) selects the shape behind
        let shape = filled_box(Rect::new(100.0, 100.0, 300.0, 300.0));
        let tf = text(Rect::new(100.0, 100.0, 300.0, 300.0), "Hi");
        let (shape_id, tf_id) = (shape.id(), tf.id());
        let page = page_with(vec![shape, tf]);
        assert_eq!(
            object_at(&page, Point::new(250.0, 350.0), TOL),
            Some(shape_id)
        );
        // Blank page inside the text box with nothing behind: the text box
        let page = page_with(vec![text(Rect::new(100.0, 100.0, 300.0, 300.0), "Hi")]);
        assert!(object_at(&page, Point::new(250.0, 350.0), TOL).is_some());
        let _ = tf_id;
    }

    #[test]
    fn clicks_near_edges_count() {
        let bx = filled_box(Rect::new(100.0, 100.0, 50.0, 50.0));
        let id = bx.id();
        let page = page_with(vec![bx]);
        assert_eq!(object_at(&page, Point::new(97.0, 120.0), TOL), Some(id));
        assert_eq!(object_at(&page, Point::new(90.0, 120.0), TOL), None);
    }

    #[test]
    fn thin_lines_are_easy_to_hit() {
        let line = Shape::line(Point::new(0.0, 0.0), Point::new(100.0, 100.0));
        let id = line.id();
        let page = page_with(vec![line]);
        assert_eq!(object_at(&page, Point::new(52.0, 48.0), TOL), Some(id));
        assert_eq!(object_at(&page, Point::new(80.0, 20.0), TOL), None);
    }

    #[test]
    fn unfilled_ellipse_interior_is_weak_and_corners_miss() {
        let mut s = Shape::ellipse(Rect::new(0.0, 0.0, 100.0, 100.0));
        s.fill = Fill::None;
        let obj = s;
        assert_eq!(hit(&obj, Point::new(50.0, 50.0), TOL), Some(Hit::Weak));
        assert_eq!(hit(&obj, Point::new(50.0, 1.0), TOL), Some(Hit::Strong));
        // Bounding-box corner is outside the ellipse
        assert_eq!(hit(&obj, Point::new(3.0, 3.0), TOL), None);
    }
}
