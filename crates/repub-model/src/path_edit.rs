//! Point editing of a [`Path`]: adding, removing and converting anchors, and
//! moving the Bézier control handles that belong to an anchor.

use repub_geometry::Point;

use crate::geometry::{Path, PathCommand};

/// One Bézier control handle of a path: command index and which control of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlRef {
    /// Index into [`Path::commands`].
    pub command: usize,
    /// 0 for a quadratic's control, 1 or 2 for a cubic's first or second control.
    pub which: u8,
}

fn lerp(a: Point, b: Point, t: f64) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

fn end_of(cmd: &PathCommand) -> Option<Point> {
    match cmd {
        PathCommand::MoveTo(p) | PathCommand::LineTo(p) => Some(*p),
        PathCommand::QuadTo { end, .. } | PathCommand::CubicTo { end, .. } => Some(*end),
        PathCommand::Close => None,
    }
}

impl Path {
    /// Index of the command that holds anchor `index`.
    pub fn anchor_command(&self, index: usize) -> Option<usize> {
        self.commands.iter().enumerate().filter(|(_, c)| !matches!(c, PathCommand::Close)).nth(index).map(|(i, _)| i)
    }

    /// The point a command starts from, and where its subpath started.
    fn start_of(&self, command: usize) -> Option<(Point, Point)> {
        let (mut current, mut subpath) = (None, None);
        for cmd in &self.commands[..command.min(self.commands.len())] {
            match cmd {
                PathCommand::MoveTo(p) => {
                    current = Some(*p);
                    subpath = Some(*p);
                }
                PathCommand::Close => current = subpath,
                other => current = end_of(other),
            }
        }
        Some((current?, subpath?))
    }

    /// The control handles attached to anchor `index`: those of its incoming segment and of the
    /// segment leaving it, each with the anchor it hangs from.
    pub fn control_handles(&self, index: usize) -> Vec<(ControlRef, Point, Point)> {
        let mut out = Vec::new();
        let Some(i) = self.anchor_command(index) else { return out };
        let anchor = end_of(&self.commands[i]).unwrap_or_default();
        match self.commands[i] {
            PathCommand::QuadTo { control, .. } => out.push((ControlRef { command: i, which: 0 }, control, anchor)),
            PathCommand::CubicTo { control2, .. } => out.push((ControlRef { command: i, which: 2 }, control2, anchor)),
            _ => {}
        }
        if let Some(next) = self.commands.get(i + 1) {
            match *next {
                PathCommand::QuadTo { control, .. } => out.push((ControlRef { command: i + 1, which: 0 }, control, anchor)),
                PathCommand::CubicTo { control1, .. } => out.push((ControlRef { command: i + 1, which: 1 }, control1, anchor)),
                _ => {}
            }
        }
        out
    }

    /// Move one control handle to `p`.
    pub fn move_control(&mut self, handle: ControlRef, p: Point) -> bool {
        match (self.commands.get_mut(handle.command), handle.which) {
            (Some(PathCommand::QuadTo { control, .. }), 0) => *control = p,
            (Some(PathCommand::CubicTo { control1, .. }), 1) => *control1 = p,
            (Some(PathCommand::CubicTo { control2, .. }), 2) => *control2 = p,
            _ => return false,
        }
        true
    }

    /// Whether the segment arriving at anchor `index` is a curve.
    pub fn segment_is_curved(&self, index: usize) -> bool {
        self.anchor_command(index).is_some_and(|i| matches!(self.commands[i], PathCommand::QuadTo { .. } | PathCommand::CubicTo { .. }))
    }

    /// Make the segment arriving at anchor `index` a curve (handles a third of the way along it) or
    /// a straight line.
    pub fn set_segment_curved(&mut self, index: usize, curved: bool) -> bool {
        let Some(i) = self.anchor_command(index) else { return false };
        let Some((start, _)) = self.start_of(i) else { return false };
        let replacement = match (self.commands[i], curved) {
            (PathCommand::LineTo(end), true) => PathCommand::CubicTo { control1: lerp(start, end, 1.0 / 3.0), control2: lerp(start, end, 2.0 / 3.0), end },
            (PathCommand::QuadTo { end, .. } | PathCommand::CubicTo { end, .. }, false) => PathCommand::LineTo(end),
            _ => return false,
        };
        self.commands[i] = replacement;
        true
    }

    /// Remove anchor `index`, joining its neighbors. Refuses to leave fewer than two anchors.
    pub fn remove_anchor_point(&mut self, index: usize) -> bool {
        if self.anchor_points().len() <= 2 {
            return false;
        }
        let Some(i) = self.anchor_command(index) else { return false };
        if matches!(self.commands[i], PathCommand::MoveTo(_)) {
            // The next anchor becomes the start of the subpath
            let Some(next_end) = self.commands.get(i + 1).and_then(end_of) else { return false };
            if matches!(self.commands[i + 1], PathCommand::MoveTo(_)) {
                return false;
            }
            self.commands[i] = PathCommand::MoveTo(next_end);
            self.commands.remove(i + 1);
        } else {
            self.commands.remove(i);
        }
        true
    }

    /// The segment nearest `p` within `tolerance`: its command index, position along it (0-1) and the
    /// point on it.
    pub fn nearest_segment(&self, p: Point, tolerance: f64) -> Option<(usize, f64, Point)> {
        const STEPS: usize = 32;
        let mut best: Option<(usize, f64, Point, f64)> = None;
        for (i, cmd) in self.commands.iter().enumerate() {
            if matches!(cmd, PathCommand::MoveTo(_)) {
                continue;
            }
            let Some((start, subpath_start)) = self.start_of(i) else { continue };
            let at = |t: f64| -> Point {
                match *cmd {
                    PathCommand::LineTo(end) => lerp(start, end, t),
                    PathCommand::Close => lerp(start, subpath_start, t),
                    PathCommand::QuadTo { control, end } => lerp(lerp(start, control, t), lerp(control, end, t), t),
                    PathCommand::CubicTo { control1, control2, end } => {
                        let (a, b, c) = (lerp(start, control1, t), lerp(control1, control2, t), lerp(control2, end, t));
                        lerp(lerp(a, b, t), lerp(b, c, t), t)
                    }
                    PathCommand::MoveTo(q) => q,
                }
            };
            for s in 0..STEPS {
                let (t0, t1) = (s as f64 / STEPS as f64, (s + 1) as f64 / STEPS as f64);
                let (a, b) = (at(t0), at(t1));
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let len_sq = dx * dx + dy * dy;
                let k = if len_sq > 0.0 { (((p.x - a.x) * dx + (p.y - a.y) * dy) / len_sq).clamp(0.0, 1.0) } else { 0.0 };
                let q = Point::new(a.x + k * dx, a.y + k * dy);
                let d = ((p.x - q.x).powi(2) + (p.y - q.y).powi(2)).sqrt();
                if d <= tolerance && best.as_ref().map_or(true, |b| d < b.3) {
                    best = Some((i, t0 + k * (t1 - t0), q, d));
                }
            }
        }
        best.map(|(i, t, q, _)| (i, t, q))
    }

    /// Split the segment of `command` at `t`, adding an anchor there. Returns the new anchor's index.
    pub fn split_segment(&mut self, command: usize, t: f64) -> Option<usize> {
        let (start, subpath_start) = self.start_of(command)?;
        let t = t.clamp(0.01, 0.99);
        let (first, second) = match *self.commands.get(command)? {
            PathCommand::LineTo(end) => (PathCommand::LineTo(lerp(start, end, t)), PathCommand::LineTo(end)),
            PathCommand::Close => (PathCommand::LineTo(lerp(start, subpath_start, t)), PathCommand::Close),
            PathCommand::QuadTo { control, end } => {
                let (a, b) = (lerp(start, control, t), lerp(control, end, t));
                let m = lerp(a, b, t);
                (PathCommand::QuadTo { control: a, end: m }, PathCommand::QuadTo { control: b, end })
            }
            PathCommand::CubicTo { control1, control2, end } => {
                let (a, b, c) = (lerp(start, control1, t), lerp(control1, control2, t), lerp(control2, end, t));
                let (d, e) = (lerp(a, b, t), lerp(b, c, t));
                let m = lerp(d, e, t);
                (PathCommand::CubicTo { control1: a, control2: d, end: m }, PathCommand::CubicTo { control1: e, control2: c, end })
            }
            PathCommand::MoveTo(_) => return None,
        };
        self.commands[command] = second;
        self.commands.insert(command, first);
        self.commands[..=command].iter().filter(|c| !matches!(c, PathCommand::Close)).count().checked_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Path {
        Path::builder().move_to(0.0, 0.0).line_to(10.0, 0.0).line_to(10.0, 10.0).line_to(0.0, 10.0).close().build()
    }

    #[test]
    fn a_point_can_be_added_on_a_segment_and_removed_again() {
        let mut path = square();
        let (command, t, on) = path.nearest_segment(Point::new(5.0, 1.0), 3.0).expect("near the top edge");
        assert!((on.y).abs() < 1e-9 && (t - 0.5).abs() < 0.05);
        let index = path.split_segment(command, t).expect("anchor");
        assert_eq!(path.anchor_points().len(), 5);
        assert!((path.anchor_points()[index].x - 5.0).abs() < 0.5);
        assert!(path.remove_anchor_point(index));
        assert_eq!(path.anchor_points().len(), 4);
    }

    #[test]
    fn the_closing_segment_can_be_split() {
        let mut path = square();
        let (command, t, _) = path.nearest_segment(Point::new(0.5, 5.0), 3.0).expect("left edge");
        assert!(matches!(path.commands[command], PathCommand::Close));
        path.split_segment(command, t).unwrap();
        assert_eq!(path.anchor_points().len(), 5);
        assert!(matches!(path.commands.last(), Some(PathCommand::Close)));
    }

    #[test]
    fn segments_turn_into_curves_and_back_with_handles_to_drag() {
        let mut path = square();
        assert!(!path.segment_is_curved(1));
        assert!(path.set_segment_curved(1, true));
        assert!(path.segment_is_curved(1));
        let handles = path.control_handles(1);
        assert_eq!(handles.len(), 1, "the segment arriving at anchor 1, and a line leaves it");
        assert!(path.move_control(handles[0].0, Point::new(9.0, -4.0)));
        assert!(path.control_handles(1)[0].1.y < 0.0);
        assert!(path.set_segment_curved(1, false));
        assert!(!path.segment_is_curved(1));
    }

    #[test]
    fn splitting_a_curve_keeps_its_shape_through_the_new_point() {
        let mut path = Path::builder().move_to(0.0, 0.0).cubic_to(0.0, 10.0, 10.0, 10.0, 10.0, 0.0).close().build();
        let index = path.split_segment(1, 0.5).unwrap();
        let mid = path.anchor_points()[index];
        assert!((mid.x - 5.0).abs() < 1e-9 && (mid.y - 7.5).abs() < 1e-9);
    }

    #[test]
    fn deleting_the_first_point_makes_the_next_one_the_start() {
        let mut path = square();
        assert!(path.remove_anchor_point(0));
        assert_eq!(path.anchor_points()[0], Point::new(10.0, 0.0));
        assert!(matches!(path.commands[0], PathCommand::MoveTo(_)));
        // Two anchors are the least a path keeps
        let mut line = Path::builder().move_to(0.0, 0.0).line_to(1.0, 1.0).build();
        assert!(!line.remove_anchor_point(0));
    }
}
