//! The Office legacy ("Escher", MS-ODRAW) preset shape geometry engine.
//!
//! Each template ([`PresetDef`], data in `presets.rs`) describes a shape in a
//! 21600 x 21600 coordinate space: guides (formulas over the adjust values and
//! earlier guides), points (constants or guide references), and segment codes
//! saying how the points are joined. This evaluates the guides for a set of
//! adjust values and builds the outline for a given box, following MS-ODRAW's
//! definitions of the guide operations and path segments.

use repub_geometry::{Point, Rect};
use repub_model::geometry::{Path, PathCommand, SubpathStyle};

use crate::presets::PRESETS;

/// Size of the template coordinate space.
pub const UNITS: f64 = 21600.0;

/// One legacy shape template.
#[derive(Debug)]
pub struct PresetDef {
    /// Escher shape type (MSOSPT)
    pub spt: u16,
    /// Shape name (libmspub / MS-ODRAW naming, e.g. "RoundRectangle")
    pub name: &'static str,
    /// A WordArt text path (its outline guides text, it isn't drawn)
    pub text_path: bool,
    /// Filled by default (false for the arc, lines, connectors and brackets)
    pub filled: bool,
    /// Points; `0x8000_0000 | n` refers to guide `n`
    pub vertices: &'static [(i32, i32)],
    /// Segment codes (MS-ODRAW MSOPATHINFO); empty: one closed polygon
    pub segments: &'static [u16],
    /// Guides: operation and flags, then three parameters
    pub guides: &'static [[u16; 4]],
    /// Default adjust values
    pub adjust: &'static [i32],
    /// Text rectangles (l, t, r, b)
    pub inscribe: &'static [[i32; 4]],
    /// Connection sites
    pub connections: &'static [(i32, i32)],
    /// Adjust handles (flags, x, y, center x, center y, x min, x max, y min, y max)
    pub handles: &'static [[i32; 9]],
}

/// The template for shape type `spt` (1..=202).
pub fn preset(spt: u16) -> Option<&'static PresetDef> {
    PRESETS.get(usize::from(spt).checked_sub(1)?)
}

/// Every template, in shape type order.
pub fn presets() -> &'static [PresetDef] {
    &PRESETS
}

/// Handle flags (MS-ODRAW ADJH; LibreOffice's MSDFF_HANDLE_FLAGS)
pub(crate) const H_POLAR: i32 = 0x8;
pub(crate) const H_X_MIN_REF: i32 = 0x80;
pub(crate) const H_X_MAX_REF: i32 = 0x100;
pub(crate) const H_Y_MIN_REF: i32 = 0x200;
pub(crate) const H_Y_MAX_REF: i32 = 0x400;
pub(crate) const H_CENTER_X_REF: i32 = 0x800;
pub(crate) const H_CENTER_Y_REF: i32 = 0x1000;
pub(crate) const H_RADIUS_RANGE: i32 = 0x2000;

/// What a handle parameter refers to.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum HandleParam {
    /// Adjust value `n` (0-based): the handle moves it
    Adjust(usize),
    /// A fixed or computed position
    Value(f64),
}

impl Guides {
    /// A handle parameter: references (`special`) are adjust values
    /// (0x100+n), guides (3..=0x82), or the edges (0 left/top, 1 right/bottom,
    /// 2 centre); otherwise a constant.
    fn handle_param(&self, raw: i32, special: bool) -> HandleParam {
        if !special {
            return HandleParam::Value(f64::from(raw));
        }
        match raw {
            0x100..=0x109 => HandleParam::Adjust((raw - 0x100) as usize),
            3..=0x82 => {
                HandleParam::Value(self.values.get((raw - 3) as usize).copied().unwrap_or(0.0))
            }
            0 => HandleParam::Value(0.0),
            1 => HandleParam::Value(UNITS),
            2 => HandleParam::Value(UNITS / 2.0),
            v => HandleParam::Value(f64::from(v)),
        }
    }

    fn value(&self, p: HandleParam) -> f64 {
        match p {
            HandleParam::Adjust(n) => self.adjust.get(n).copied().unwrap_or(0.0),
            HandleParam::Value(v) => v,
        }
    }

    /// A range limit, or `None` when unbounded.
    fn limit(&self, raw: i32, special: bool) -> Option<f64> {
        if raw == i32::MIN || raw == i32::MAX {
            return None;
        }
        Some(self.value(self.handle_param(raw, special)))
    }
}

/// An adjust handle of a preset shape, in template units.
pub(crate) struct Handle {
    flags: i32,
    x: HandleParam,
    y: HandleParam,
    center: (f64, f64),
    x_range: (Option<f64>, Option<f64>),
    y_range: (Option<f64>, Option<f64>),
}

impl Handle {
    pub(crate) fn of(raw: &[i32; 9], g: &Guides) -> Self {
        let f = raw[0];
        Handle {
            flags: f,
            x: g.handle_param(raw[1], true),
            y: g.handle_param(raw[2], true),
            center: (
                g.value(g.handle_param(raw[3], f & H_CENTER_X_REF != 0)),
                g.value(g.handle_param(raw[4], f & H_CENTER_Y_REF != 0)),
            ),
            x_range: (
                g.limit(raw[5], f & H_X_MIN_REF != 0),
                g.limit(raw[6], f & H_X_MAX_REF != 0),
            ),
            y_range: (
                g.limit(raw[7], f & H_Y_MIN_REF != 0),
                g.limit(raw[8], f & H_Y_MAX_REF != 0),
            ),
        }
    }

    pub(crate) fn polar(&self) -> bool {
        self.flags & H_POLAR != 0
    }

    /// Position in template units. Polar handles: x is the radius, y the
    /// angle (16.16 degrees, counter-clockwise) around the centre.
    pub(crate) fn position(&self, g: &Guides) -> Point {
        let (a, b) = (g.value(self.x), g.value(self.y));
        if self.polar() {
            let t = fixed_rad(b);
            Point::new(self.center.0 + a * t.cos(), self.center.1 - a * t.sin())
        } else {
            Point::new(a, b)
        }
    }
}

pub(crate) fn clamp_range(v: f64, (lo, hi): (Option<f64>, Option<f64>)) -> f64 {
    let v = lo.map_or(v, |lo| v.max(lo));
    hi.map_or(v, |hi| v.min(hi.max(lo.unwrap_or(f64::MIN))))
}

/// Template coordinates -> page coordinates in `bounds`.
pub(crate) fn scale(p: Point, bounds: Rect) -> Point {
    Point::new(
        bounds.origin.x + p.x / UNITS * bounds.size.width,
        bounds.origin.y + p.y / UNITS * bounds.size.height,
    )
}

/// Evaluated guides and adjust values.
pub(crate) struct Guides {
    pub(crate) adjust: Vec<f64>,
    pub(crate) values: Vec<f64>,
}

/// 16.16 fixed-point degrees -> radians.
pub(crate) fn fixed_rad(v: f64) -> f64 {
    (v / 65536.0).to_radians()
}

impl Guides {
    /// A guide parameter: a reference (when its flag is set) or a constant.
    fn param(&self, raw: u16, is_ref: bool) -> f64 {
        if !is_ref {
            return f64::from(raw);
        }
        match raw {
            0x140 | 0x141 => 0.0,   // geoLeft, geoTop
            0x142 | 0x143 => UNITS, // geoRight, geoBottom
            0x147..=0x150 => self.adjust[usize::from(raw - 0x147)],
            0x400..=0x7ff => self
                .values
                .get(usize::from(raw - 0x400))
                .copied()
                .unwrap_or(0.0),
            _ => 0.0, // e.g. line width (0x1fc), not modelled
        }
    }

    pub(crate) fn eval(&self, g: &[u16; 4]) -> f64 {
        let flags = g[0];
        let a = self.param(g[1], flags & 0x2000 != 0);
        let b = self.param(g[2], flags & 0x4000 != 0);
        let c = self.param(g[3], flags & 0x8000 != 0);
        match flags & 0x1fff {
            0 => a + b - c,
            1 => {
                if c == 0.0 {
                    0.0
                } else {
                    a * b / c
                }
            }
            2 => (a + b) / 2.0,
            3 => a.abs(),
            4 => a.min(b),
            5 => a.max(b),
            6 => {
                if a > 0.0 {
                    b
                } else {
                    c
                }
            }
            7 => (a * a + b * b + c * c).sqrt(),
            8 => b.atan2(a).to_degrees() * 65536.0,
            9 => a * fixed_rad(b).sin(),
            10 => a * fixed_rad(b).cos(),
            11 => a * c.atan2(b).cos(),
            12 => a * c.atan2(b).sin(),
            13 => a.max(0.0).sqrt(),
            14 => a + (b - c) * 65536.0,
            15 => {
                if b == 0.0 {
                    0.0
                } else {
                    c * (1.0 - (a / b) * (a / b)).max(0.0).sqrt()
                }
            }
            16 => a * fixed_rad(b).tan(),
            _ => 0.0,
        }
    }

    /// A point coordinate: a guide reference or a constant.
    pub(crate) fn coord(&self, v: i32) -> f64 {
        let u = v as u32;
        if u & 0x8000_0000 != 0 && u & 0x7fff_ffff < 0x1_0000 {
            self.values
                .get((u & 0x7fff_ffff) as usize)
                .copied()
                .unwrap_or(0.0)
        } else {
            f64::from(v)
        }
    }
}

/// Kappa for approximating a quarter ellipse with one cubic.
const KAPPA: f64 = 0.552_284_749_8;

/// Builds a [`Path`] in template units, scaled to the box at the end.
struct Builder {
    path: Path,
    current: Option<Point>,
    start: Option<Point>,
    /// Subpaths of the current group (up to the next `end`) share no-fill/no-line
    group_start: usize,
    group_style: SubpathStyle,
    subpath_count: usize,
}

impl Builder {
    fn new() -> Self {
        Self {
            path: Path::new(),
            current: None,
            start: None,
            group_start: 0,
            group_style: SubpathStyle::default(),
            subpath_count: 0,
        }
    }

    fn move_to(&mut self, p: Point) {
        self.path.commands.push(PathCommand::MoveTo(p));
        self.subpath_count += 1;
        self.current = Some(p);
        self.start = Some(p);
    }

    fn line_to(&mut self, p: Point) {
        if self.current.is_none() {
            return self.move_to(p);
        }
        self.path.commands.push(PathCommand::LineTo(p));
        self.current = Some(p);
    }

    fn cubic(&mut self, c1: Point, c2: Point, end: Point) {
        if self.current.is_none() {
            self.move_to(c1);
        }
        self.path.commands.push(PathCommand::CubicTo {
            control1: c1,
            control2: c2,
            end,
        });
        self.current = Some(end);
    }

    fn close(&mut self) {
        if self.current.is_some() {
            self.path.commands.push(PathCommand::Close);
            self.current = self.start;
        }
    }

    /// `end` segment: the group's flags apply to its subpaths.
    fn end_group(&mut self) {
        if self.group_style != SubpathStyle::default() {
            if self.path.subpaths.len() < self.subpath_count {
                self.path
                    .subpaths
                    .resize(self.subpath_count, SubpathStyle::default());
            }
            for s in &mut self.path.subpaths[self.group_start..self.subpath_count] {
                s.no_fill |= self.group_style.no_fill;
                s.no_stroke |= self.group_style.no_stroke;
            }
        }
        self.group_start = self.subpath_count;
        self.group_style = SubpathStyle::default();
        self.current = None;
    }

    /// Quarter ellipse from the current point to `p`, leaving horizontally
    /// (`x_first`) or vertically.
    fn quadrant(&mut self, p: Point, x_first: bool) {
        let Some(c) = self.current else {
            return self.move_to(p);
        };
        let (dx, dy) = (p.x - c.x, p.y - c.y);
        let (c1, c2) = if x_first {
            (
                Point::new(c.x + dx * KAPPA, c.y),
                Point::new(p.x, p.y - dy * KAPPA),
            )
        } else {
            (
                Point::new(c.x, c.y + dy * KAPPA),
                Point::new(p.x - dx * KAPPA, p.y),
            )
        };
        self.cubic(c1, c2, p);
    }

    /// Elliptical arc on the ellipse with centre `c` and radii `rx`, `ry`, from
    /// parametric angle `a0` sweeping `sweep` (radians, y down: positive is
    /// clockwise on screen). `connect`: line from the current point (`...To`
    /// segments), else start a new subpath.
    fn arc(&mut self, c: Point, rx: f64, ry: f64, a0: f64, sweep: f64, connect: bool) {
        let at = |t: f64| Point::new(c.x + rx * t.cos(), c.y + ry * t.sin());
        let start = at(a0);
        if connect && self.current.is_some() {
            self.line_to(start);
        } else {
            self.move_to(start);
        }
        let n = ((sweep.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
        let step = sweep / n as f64;
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        for i in 0..n {
            let t0 = a0 + step * i as f64;
            let t1 = t0 + step;
            let (p0, p1) = (at(t0), at(t1));
            let c1 = Point::new(p0.x - k * rx * t0.sin(), p0.y + k * ry * t0.cos());
            let c2 = Point::new(p1.x + k * rx * t1.sin(), p1.y - k * ry * t1.cos());
            self.cubic(c1, c2, p1);
        }
    }

    /// Windows-style arc in box (l, t)-(r, b) from the ray through `from` to the
    /// ray through `to`, counter-clockwise on screen unless `clockwise`.
    fn box_arc(&mut self, corners: [Point; 4], clockwise: bool, connect: bool) {
        let [p0, p1, from, to] = corners;
        let c = Point::new((p0.x + p1.x) / 2.0, (p0.y + p1.y) / 2.0);
        let (rx, ry) = ((p1.x - p0.x).abs() / 2.0, (p1.y - p0.y).abs() / 2.0);
        if rx == 0.0 || ry == 0.0 {
            return;
        }
        let param = |p: Point| ((p.y - c.y) / ry).atan2((p.x - c.x) / rx);
        let (a0, a1) = (param(from), param(to));
        let tau = std::f64::consts::TAU;
        let mut sweep = a1 - a0;
        if clockwise {
            while sweep <= 0.0 {
                sweep += tau;
            }
        } else {
            while sweep >= 0.0 {
                sweep -= tau;
            }
        }
        self.arc(c, rx, ry, a0, sweep, connect);
    }

    fn finish(mut self, bounds: Rect) -> Path {
        self.end_group();
        let s = |p: Point| scale(p, bounds);
        for c in &mut self.path.commands {
            *c = match *c {
                PathCommand::MoveTo(p) => PathCommand::MoveTo(s(p)),
                PathCommand::LineTo(p) => PathCommand::LineTo(s(p)),
                PathCommand::QuadTo { control, end } => PathCommand::QuadTo {
                    control: s(control),
                    end: s(end),
                },
                PathCommand::CubicTo {
                    control1,
                    control2,
                    end,
                } => PathCommand::CubicTo {
                    control1: s(control1),
                    control2: s(control2),
                    end: s(end),
                },
                PathCommand::Close => PathCommand::Close,
            };
        }
        self.path.subpaths.truncate(self.subpath_count);
        // Office fills these with the nonzero rule: holes come from subpaths
        // drawn the other way round (the donut's inner circle)
        self.path.even_odd = false;
        self.path
    }
}

impl PresetDef {
    pub(crate) fn build(&self, g: &Guides, bounds: Rect) -> Path {
        let pts: Vec<Point> = self
            .vertices
            .iter()
            .map(|&(x, y)| Point::new(g.coord(x), g.coord(y)))
            .collect();
        let mut b = Builder::new();
        let mut next = 0usize;
        let mut take = |n: usize| -> Vec<Point> {
            let out: Vec<Point> = pts.iter().skip(next).take(n).copied().collect();
            next += n;
            out
        };

        if self.segments.is_empty() {
            // One closed polygon through every point
            for (i, p) in take(pts.len()).into_iter().enumerate() {
                if i == 0 {
                    b.move_to(p)
                } else {
                    b.line_to(p)
                }
            }
            b.close();
            return b.finish(bounds);
        }

        for &seg in self.segments {
            let count = usize::from(seg & 0x1fff);
            match seg >> 13 {
                0 => take(count.max(1)).into_iter().for_each(|p| b.line_to(p)),
                1 => {
                    for c in take(3 * count.max(1)).chunks_exact(3) {
                        b.cubic(c[0], c[1], c[2]);
                    }
                }
                2 => {
                    if let Some(p) = take(1).pop() {
                        b.move_to(p);
                    }
                }
                3 => b.close(),
                4 => b.end_group(),
                5 => {
                    let code = (seg >> 8) & 0x1f;
                    let n = usize::from(seg & 0xff);
                    match code {
                        // angleEllipseTo / angleEllipse: centre, radii, (start, sweep) in 16.16 degrees
                        // (only the first arc of an angleEllipse starts a new figure;
                        // later ones join it with a line)
                        1 | 2 => {
                            for (i, e) in take(n).chunks_exact(3).enumerate() {
                                let (start, sweep) = (fixed_rad(e[2].x), fixed_rad(e[2].y));
                                // Office angles run counter-clockwise; y is down here
                                b.arc(
                                    e[0],
                                    e[1].x.abs(),
                                    e[1].y.abs(),
                                    -start,
                                    -sweep,
                                    code == 1 || i > 0,
                                );
                            }
                        }
                        // arcTo / arc / clockwiseArcTo / clockwiseArc: box corners, from, to
                        // (as above: one figure per command)
                        3..=6 => {
                            for (i, a) in take(n).chunks_exact(4).enumerate() {
                                b.box_arc(
                                    [a[0], a[1], a[2], a[3]],
                                    code >= 5,
                                    code == 3 || code == 5 || i > 0,
                                );
                            }
                        }
                        // ellipticalQuadrantX / Y, alternating direction
                        7 | 8 => {
                            let mut x_first = code == 7;
                            for p in take(n) {
                                b.quadrant(p, x_first);
                                x_first = !x_first;
                            }
                        }
                        // quadraticBezier: control, end
                        9 => {
                            for q in take(n).chunks_exact(2) {
                                if let Some(c) = b.current {
                                    let c1 = Point::new(
                                        c.x + 2.0 / 3.0 * (q[0].x - c.x),
                                        c.y + 2.0 / 3.0 * (q[0].y - c.y),
                                    );
                                    let c2 = Point::new(
                                        q[1].x + 2.0 / 3.0 * (q[0].x - q[1].x),
                                        q[1].y + 2.0 / 3.0 * (q[0].y - q[1].y),
                                    );
                                    b.cubic(c1, c2, q[1]);
                                }
                            }
                        }
                        10 => b.group_style.no_fill = true,
                        11 => b.group_style.no_stroke = true,
                        // autoLine, cornerLine, smoothLine, symmetricLine...: straight lines
                        12 | 14 | 16 | 18 => take(n).into_iter().for_each(|p| b.line_to(p)),
                        // autoCurve and friends: curves
                        13 | 15 | 17 | 19 => {
                            for c in take(n).chunks_exact(3) {
                                b.cubic(c[0], c[1], c[2]);
                            }
                        }
                        _ => {
                            take(n);
                        }
                    }
                }
                _ => {}
            }
        }
        b.finish(bounds)
    }
}

/// The preset definition for `spt`, if known.
fn def_of(spt: u16) -> Option<&'static PresetDef> {
    preset(spt)
}

/// The effective adjust value at `index`: the override if set, else the
/// preset's default, else 0.
pub(crate) fn adjust_value(spt: u16, adjust: &[Option<i32>], index: usize) -> i32 {
    adjust
        .get(index)
        .copied()
        .flatten()
        .or_else(|| def_of(spt).and_then(|d| d.adjust.get(index).copied()))
        .unwrap_or(0)
}

/// Evaluate the guides of preset `spt` for the given adjust values.
fn eval(spt: u16, adjust: &[Option<i32>]) -> Guides {
    let def = def_of(spt);
    let adjust: Vec<f64> = (0..10)
        .map(|i| f64::from(adjust_value(spt, adjust, i)))
        .collect();
    let mut g = Guides {
        adjust,
        values: Vec::new(),
    };
    if let Some(def) = def {
        for guide in def.guides {
            let v = g.eval(guide);
            g.values.push(v);
        }
    }
    g
}

/// The outline of preset `spt` for `bounds`, or `None` for an unknown type.
pub(crate) fn build_path(spt: u16, adjust: &[Option<i32>], bounds: Rect) -> Option<Path> {
    let def = def_of(spt)?;
    Some(def.build(&eval(spt, adjust), bounds))
}

/// The text rectangle of preset `spt` for `bounds` (the whole box if none).
pub(crate) fn text_rect(spt: u16, adjust: &[Option<i32>], bounds: Rect) -> Rect {
    let Some(def) = def_of(spt) else {
        return bounds;
    };
    let Some(r) = def.inscribe.first() else {
        return bounds;
    };
    let g = eval(spt, adjust);
    let p0 = scale(Point::new(g.coord(r[0]), g.coord(r[1])), bounds);
    let p1 = scale(Point::new(g.coord(r[2]), g.coord(r[3])), bounds);
    Rect::new(
        p0.x.min(p1.x),
        p0.y.min(p1.y),
        (p1.x - p0.x).abs(),
        (p1.y - p0.y).abs(),
    )
}

/// The connection sites of preset `spt` on the page for `bounds`.
pub(crate) fn connection_sites(spt: u16, adjust: &[Option<i32>], bounds: Rect) -> Vec<Point> {
    let Some(def) = def_of(spt) else {
        return Vec::new();
    };
    let g = eval(spt, adjust);
    def.connections
        .iter()
        .map(|&(x, y)| scale(Point::new(g.coord(x), g.coord(y)), bounds))
        .collect()
}

/// The positions of the adjust handles of preset `spt` on the page for
/// `bounds` (before the shape's flips and rotation).
pub(crate) fn handles(spt: u16, adjust: &[Option<i32>], bounds: Rect) -> Vec<Point> {
    let Some(def) = def_of(spt) else {
        return Vec::new();
    };
    let g = eval(spt, adjust);
    def.handles
        .iter()
        .map(|raw| scale(Handle::of(raw, &g).position(&g), bounds))
        .collect()
}

/// Drag adjust handle `index` of preset `spt` to page point `to` within
/// `bounds`. Returns the adjust-value overrides to apply (index, value), empty
/// if nothing changed or the handle/bounds are invalid.
pub(crate) fn drag_handle(
    spt: u16,
    adjust: &[Option<i32>],
    index: usize,
    bounds: Rect,
    to: Point,
) -> Vec<(usize, i32)> {
    let Some(raw) = def_of(spt).and_then(|d| d.handles.get(index)) else {
        return Vec::new();
    };
    if bounds.size.width <= 0.0 || bounds.size.height <= 0.0 {
        return Vec::new();
    }
    let g = eval(spt, adjust);
    let h = Handle::of(raw, &g);
    // Page -> template units.
    let p = Point::new(
        (to.x - bounds.origin.x) / bounds.size.width * UNITS,
        (to.y - bounds.origin.y) / bounds.size.height * UNITS,
    );
    let mut changes: Vec<(usize, f64)> = Vec::new();
    if h.polar() {
        let (dx, dy) = (p.x - h.center.0, p.y - h.center.1);
        if let HandleParam::Adjust(n) = h.x {
            let r = (dx * dx + dy * dy).sqrt();
            let range = if h.flags & H_RADIUS_RANGE != 0 {
                h.x_range
            } else {
                (None, None)
            };
            changes.push((n, clamp_range(r, range)));
        }
        if let HandleParam::Adjust(n) = h.y {
            let angle = (-dy).atan2(dx).to_degrees() * 65536.0;
            changes.push((n, clamp_range(angle, h.y_range)));
        }
    } else {
        if let HandleParam::Adjust(n) = h.x {
            changes.push((n, clamp_range(p.x, h.x_range)));
        }
        if let HandleParam::Adjust(n) = h.y {
            changes.push((n, clamp_range(p.y, h.y_range)));
        }
    }
    let mut out = Vec::new();
    for (n, v) in changes {
        let v = v.round() as i32;
        if adjust_value(spt, adjust, n) != v {
            out.push((n, v));
        }
    }
    out
}
