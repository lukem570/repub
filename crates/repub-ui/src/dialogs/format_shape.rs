//! Format Shape dialog: fill, line, transparency, effects, size and position,
//! and the properties of the selected objects.

use egui::Ui;
use repub_color::{Color, GradientStop, LinearGradient, Pattern, PatternType, RadialGradient};
use repub_geometry::{Point, Rect};
use repub_model::geometry::ShapeGeometry;
use repub_model::shape::{
    ConnectorType, ImageFit, LineEnding, LineEndingStyle, Margins, Shape, ShapeKind, TextFitMode,
    VerticalAlignment,
};
use repub_model::styles::{
    Bevel, BevelStyle, BevelTechnique, DropShadow, Effects, Fill, Glow, InnerGlow, InnerGlowSource,
    InnerShadow, LineCap, LineJoin, Reflection, SoftEdges, Stroke, TexturePattern, TileMode,
};
use repub_model::ObjectId;

use super::controls::{choice, color_edit, number, number_row, ok_cancel, row, tab_strip};
use crate::state::AppState;

const TABS: [&str; 6] = [
    "Fill",
    "Line",
    "Effects",
    "Text box",
    "Size & position",
    "Properties",
];

/// Which kind of fill is being edited.
#[derive(Clone, Copy, PartialEq, Eq)]
enum FillKind {
    None,
    Solid,
    Linear,
    Radial,
    Pattern,
    Picture,
}

impl FillKind {
    fn of(fill: &Fill) -> Self {
        match fill {
            Fill::None => Self::None,
            Fill::Solid(_) => Self::Solid,
            Fill::LinearGradient(_) => Self::Linear,
            Fill::RadialGradient(_) => Self::Radial,
            Fill::Pattern(_) => Self::Pattern,
            Fill::Texture(_) => Self::Picture,
        }
    }

    /// A starting fill of this kind, keeping `from`'s main color.
    fn make(self, from: &Fill) -> Fill {
        let base = if from.is_none() {
            Color::from_rgb8(0x44, 0x72, 0xC4)
        } else {
            from.primary_color()
        };
        let darker = Color::new(base.r * 0.55, base.g * 0.55, base.b * 0.55, base.a);
        match self {
            Self::None => Fill::None,
            Self::Solid => Fill::Solid(base),
            Self::Linear => Fill::LinearGradient(LinearGradient::horizontal(base, darker)),
            Self::Radial => Fill::RadialGradient(RadialGradient::centered(base, darker)),
            Self::Pattern => {
                Fill::Pattern(Pattern::new(PatternType::Percent25, base, Color::WHITE))
            }
            Self::Picture => Fill::Texture(TexturePattern::new("")),
        }
    }
}

/// The Format Shape dialog.
#[derive(Default)]
pub struct FormatShapeDialog {
    pub open: bool,
    ids: Vec<ObjectId>,
    tab: usize,
    original: Option<Shape>,
    work: Option<Shape>,
    lock_aspect: bool,
}

impl FormatShapeDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open on the selected objects, starting from the first one's look.
    pub fn open_for_selection(&mut self, state: &AppState) {
        let objects = state.selected_objects();
        let Some(first) = objects.first() else { return };
        self.ids = state.selection.clone();
        self.original = Some((*first).clone());
        self.work = Some((*first).clone());
        self.tab = 0;
        self.lock_aspect = false;
        self.open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) {
        if !self.open {
            return;
        }
        let (Some(work), Some(original)) = (self.work.as_mut(), self.original.as_ref()) else {
            self.open = false;
            return;
        };
        let single = self.ids.len() == 1;
        let has_fill = matches!(
            work.kind,
            ShapeKind::AutoShape(_)
                | ShapeKind::TextFrame(_)
                | ShapeKind::Picture(_)
                | ShapeKind::TextEffect(_)
        );
        let has_line = has_fill || matches!(work.kind, ShapeKind::Line(_));
        let has_effects = matches!(
            work.kind,
            ShapeKind::AutoShape(_) | ShapeKind::TextFrame(_) | ShapeKind::Picture(_)
        );

        let mut tabs: Vec<usize> = Vec::new();
        if has_fill {
            tabs.push(0);
        }
        if has_line {
            tabs.push(1);
        }
        if has_effects {
            tabs.push(2);
        }
        if matches!(work.kind, ShapeKind::TextFrame(_)) {
            tabs.push(3);
        }
        tabs.push(4);
        tabs.push(5);
        if !tabs.contains(&self.tab) {
            self.tab = tabs[0];
        }

        let mut result = None;
        let units = state.units;
        let lock_aspect = &mut self.lock_aspect;
        let tab = &mut self.tab;
        egui::Window::new("Format Shape")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(420.0);
                let names: Vec<&str> = tabs.iter().map(|&t| TABS[t]).collect();
                let current = tabs.iter().position(|&t| t == *tab).unwrap_or(0);
                *tab = tabs[tab_strip(ui, current, &names)];
                egui::ScrollArea::vertical()
                    .max_height(440.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| match *tab {
                        0 => fill_tab(ui, work),
                        1 => line_tab(ui, work),
                        2 => effects_tab(ui, work),
                        3 => text_box_tab(ui, work),
                        4 => size_tab(ui, work, single, units, lock_aspect),
                        _ => properties_tab(ui, work),
                    });
                result = ok_cancel(ui);
            });

        match result {
            Some(true) => {
                let (original, work) = (original.clone(), work.clone());
                self.apply(state, &original, &work);
                self.open = false;
            }
            Some(false) => self.open = false,
            None => {}
        }
    }

    /// Write what changed in the dialog to every selected object.
    fn apply(&self, state: &mut AppState, original: &Shape, work: &Shape) {
        state.begin_edit();
        let single = self.ids.len() == 1;
        if let Some(page) = state.current_page_mut() {
            for id in &self.ids {
                let Some(shape) = page.get_shape_mut(*id) else {
                    continue;
                };
                apply_to(shape, original, work, single);
            }
        }
        state.end_edit("Format Shape");
    }
}

fn apply_to(shape: &mut Shape, original: &Shape, work: &Shape, single: bool) {
    if work.fill != original.fill {
        shape.fill = work.fill.clone();
    }
    if work.line != original.line {
        shape.line = work.line.clone();
    }
    if work.effects != original.effects {
        shape.effects = work.effects.clone();
    }
    if work.opacity != original.opacity {
        shape.opacity = work.opacity;
    }
    if work.common.locked != original.common.locked {
        shape.common.locked = work.common.locked;
    }
    if work.common.visible != original.common.visible {
        shape.common.visible = work.common.visible;
    }
    if single {
        if work.common.name != original.common.name {
            shape.common.name = work.common.name.clone();
        }
        if work.common.bounds != original.common.bounds {
            shape.set_bounds(work.common.bounds);
        }
        shape.common.rotation = work.common.rotation;
        shape.common.flip_h = work.common.flip_h;
        shape.common.flip_v = work.common.flip_v;
    }
    match (&mut shape.kind, &original.kind, &work.kind) {
        (ShapeKind::AutoShape(s), ShapeKind::AutoShape(o), ShapeKind::AutoShape(w)) => {
            if w.geometry != o.geometry && !matches!(w.geometry, ShapeGeometry::Path(_)) {
                s.geometry = w.geometry.clone();
            }
        }
        (ShapeKind::Line(s), ShapeKind::Line(o), ShapeKind::Line(w)) => {
            if w.start_ending != o.start_ending {
                s.start_ending = w.start_ending;
            }
            if w.end_ending != o.end_ending {
                s.end_ending = w.end_ending;
            }
            if w.connector_type != o.connector_type {
                s.connector_type = w.connector_type;
            }
        }
        (ShapeKind::TextFrame(s), ShapeKind::TextFrame(o), ShapeKind::TextFrame(w)) => {
            if w.vertical_align != o.vertical_align {
                s.vertical_align = w.vertical_align;
            }
            if w.effective_margins() != o.effective_margins() {
                s.margins = w.effective_margins();
                // All-zero margins read as unset, so the inset goes to zero with them
                s.inset = if margins_are_zero(&s.margins) {
                    0.0
                } else {
                    s.inset
                };
            }
            if w.columns.count != o.columns.count {
                s.columns.count = w.columns.count;
            }
            if w.columns.gap != o.columns.gap {
                s.columns.gap = w.columns.gap;
            }
            if w.text_fit != o.text_fit {
                s.text_fit = w.text_fit;
            }
        }
        (ShapeKind::Picture(s), ShapeKind::Picture(o), ShapeKind::Picture(w)) => {
            if w.fit != o.fit {
                s.fit = w.fit;
            }
            if w.crop != o.crop {
                s.crop = w.crop;
            }
            if w.brightness != o.brightness {
                s.brightness = w.brightness;
            }
            if w.contrast != o.contrast {
                s.contrast = w.contrast;
            }
            if w.alt_text != o.alt_text {
                s.alt_text = w.alt_text.clone();
            }
        }
        _ => {}
    }
}

/// Editor for any kind of fill, shared with the table cell fills.
pub(crate) fn fill_editor(ui: &mut Ui, fill: &mut Fill) {
    let mut kind = FillKind::of(fill);
    let before = kind;
    ui.horizontal_wrapped(|ui| {
        for (k, name) in [
            (FillKind::None, "No fill"),
            (FillKind::Solid, "Solid"),
            (FillKind::Linear, "Linear gradient"),
            (FillKind::Radial, "Radial gradient"),
            (FillKind::Pattern, "Pattern"),
            (FillKind::Picture, "Picture"),
        ] {
            ui.radio_value(&mut kind, k, name);
        }
    });
    if kind != before {
        *fill = kind.make(fill);
    }
    ui.add_space(6.0);
    match fill {
        Fill::None => {}
        Fill::Solid(color) => {
            row(ui, "Color", |ui| color_edit(ui, color));
        }
        Fill::LinearGradient(g) => {
            let (dx, dy) = (g.end.x - g.start.x, g.end.y - g.start.y);
            let mut angle = dy.atan2(dx).to_degrees().rem_euclid(360.0);
            if number_row(ui, "Angle", &mut angle, 0.0..=359.0, 1.0, "°") {
                let (sin, cos) = angle.to_radians().sin_cos();
                // Reach the corners of the unit box along the axis
                let reach = 0.5 * (sin.abs() + cos.abs());
                g.start = Point::new(0.5 - cos * reach, 0.5 - sin * reach);
                g.end = Point::new(0.5 + cos * reach, 0.5 + sin * reach);
            }
            stops_editor(ui, &mut g.stops);
        }
        Fill::RadialGradient(g) => {
            let (mut cx, mut cy, mut r) =
                (g.center.x * 100.0, g.center.y * 100.0, g.radius * 100.0);
            if number_row(ui, "Center x", &mut cx, 0.0..=100.0, 1.0, "%") {
                g.center.x = cx / 100.0;
            }
            if number_row(ui, "Center y", &mut cy, 0.0..=100.0, 1.0, "%") {
                g.center.y = cy / 100.0;
            }
            if number_row(ui, "Radius", &mut r, 1.0..=300.0, 1.0, "%") {
                g.radius = r / 100.0;
            }
            stops_editor(ui, &mut g.stops);
        }
        Fill::Pattern(p) => {
            row(ui, "Pattern", |ui| {
                egui::ComboBox::from_id_salt("fill_pattern")
                    .selected_text(p.pattern_type.name())
                    .height(280.0)
                    .show_ui(ui, |ui| {
                        for t in PatternType::all() {
                            ui.selectable_value(&mut p.pattern_type, *t, t.name());
                        }
                    });
            });
            row(ui, "Foreground", |ui| color_edit(ui, &mut p.foreground));
            row(ui, "Background", |ui| color_edit(ui, &mut p.background));
            number_row(ui, "Scale", &mut p.scale, 0.1..=20.0, 0.05, "×");
            number_row(ui, "Rotation", &mut p.rotation, -360.0..=360.0, 1.0, "°");
        }
        Fill::Texture(t) => {
            row(ui, "Picture", |ui| {
                let name = std::path::Path::new(&t.source)
                    .file_name()
                    .map_or("(none)".to_string(), |n| n.to_string_lossy().into_owned());
                ui.label(name);
                if ui.button("Choose…").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter(
                            "Pictures",
                            &["png", "jpg", "jpeg", "gif", "bmp", "webp", "tif", "tiff"],
                        )
                        .pick_file()
                    {
                        t.source = path.to_string_lossy().into_owned();
                    }
                }
            });
            choice(
                ui,
                "fill_tile",
                "Mode",
                &mut t.tile,
                &[
                    (TileMode::Tile, "Tile"),
                    (TileMode::TileHorizontal, "Tile horizontally"),
                    (TileMode::TileVertical, "Tile vertically"),
                    (TileMode::Stretch, "Stretch"),
                    (TileMode::Center, "Center"),
                ],
            );
            number_row(ui, "Scale x", &mut t.scale_x, 0.05..=20.0, 0.05, "×");
            number_row(ui, "Scale y", &mut t.scale_y, 0.05..=20.0, 0.05, "×");
            let (mut ox, mut oy) = (t.offset_x * 100.0, t.offset_y * 100.0);
            if number_row(ui, "Offset x", &mut ox, -100.0..=100.0, 1.0, "%") {
                t.offset_x = ox / 100.0;
            }
            if number_row(ui, "Offset y", &mut oy, -100.0..=100.0, 1.0, "%") {
                t.offset_y = oy / 100.0;
            }
            number_row(ui, "Rotation", &mut t.rotation, -360.0..=360.0, 1.0, "°");
            let mut transparency = t.transparency * 100.0;
            if number_row(ui, "Transparency", &mut transparency, 0.0..=100.0, 1.0, "%") {
                t.transparency = transparency / 100.0;
            }
        }
    }
}

fn fill_tab(ui: &mut Ui, shape: &mut Shape) {
    fill_editor(ui, &mut shape.fill);
    ui.add_space(8.0);
    let mut transparency = (1.0 - shape.opacity) * 100.0;
    if number_row(
        ui,
        "Object transparency",
        &mut transparency,
        0.0..=100.0,
        1.0,
        "%",
    ) {
        shape.opacity = 1.0 - transparency / 100.0;
    }
}

fn stops_editor(ui: &mut Ui, stops: &mut Vec<GradientStop>) {
    ui.add_space(4.0);
    ui.label("Color stops");
    let mut remove = None;
    let count = stops.len();
    for (i, stop) in stops.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            let mut pos = stop.position * 100.0;
            if number(ui, &mut pos, 0.0..=100.0, 1.0, "%") {
                stop.position = pos / 100.0;
            }
            color_edit(ui, &mut stop.color);
            if ui
                .add_enabled(count > 2, egui::Button::new("Remove"))
                .clicked()
            {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        stops.remove(i);
    }
    if ui.button("Add stop").clicked() {
        let last = stops.last().map_or(Color::WHITE, |s| s.color);
        stops.push(GradientStop::new(1.0, last));
    }
    stops.sort_by(|a, b| a.position.total_cmp(&b.position));
}

const DASHES: [(&str, &[f64]); 6] = [
    ("Solid", &[]),
    ("Dots", &[1.0, 1.0]),
    ("Dashes", &[4.0, 3.0]),
    ("Dash dot", &[4.0, 3.0, 1.0, 3.0]),
    ("Long dashes", &[8.0, 3.0]),
    ("Long dash dot", &[8.0, 3.0, 1.0, 3.0]),
];

fn dash_name(stroke: &Stroke) -> &'static str {
    let Some(pattern) = stroke.dash_pattern.as_ref().filter(|p| !p.is_empty()) else {
        return "Solid";
    };
    DASHES
        .iter()
        .find(|(_, d)| {
            d.len() == pattern.len()
                && d.iter()
                    .zip(pattern)
                    .all(|(a, b)| (a * stroke.width - b).abs() < 1e-6)
        })
        .map_or("Custom", |(name, _)| *name)
}

fn line_tab(ui: &mut Ui, shape: &mut Shape) {
    let mut on = shape.line.is_some();
    if ui.checkbox(&mut on, "Draw a line").changed() {
        shape.line = on.then(|| Stroke::new(Color::BLACK, 1.0));
    }
    if let Some(stroke) = &mut shape.line {
        ui.add_space(6.0);
        row(ui, "Color", |ui| color_edit(ui, &mut stroke.color));
        let old_width = stroke.width;
        let was = dash_name(stroke);
        if number_row(ui, "Weight", &mut stroke.width, 0.0..=100.0, 0.25, " pt")
            && was != "Solid"
            && was != "Custom"
        {
            // Dashes stay in proportion to the weight
            let ratio = if old_width > 0.0 {
                stroke.width / old_width
            } else {
                1.0
            };
            if let Some(p) = &mut stroke.dash_pattern {
                p.iter_mut().for_each(|d| *d *= ratio);
            }
        }
        row(ui, "Dash", |ui| {
            let current = dash_name(stroke);
            egui::ComboBox::from_id_salt("line_dash")
                .selected_text(current)
                .show_ui(ui, |ui| {
                    for (name, multiples) in DASHES {
                        if ui.selectable_label(current == name, name).clicked() {
                            stroke.dash_pattern = (!multiples.is_empty()).then(|| {
                                multiples
                                    .iter()
                                    .map(|m| m * stroke.width.max(0.25))
                                    .collect()
                            });
                            stroke.dash_offset = 0.0;
                        }
                    }
                });
        });
        if stroke.dash_pattern.is_some() {
            number_row(
                ui,
                "Dash offset",
                &mut stroke.dash_offset,
                0.0..=1000.0,
                0.25,
                " pt",
            );
        }
        choice(
            ui,
            "line_cap",
            "Cap",
            &mut stroke.cap,
            &[
                (LineCap::Butt, "Flat"),
                (LineCap::Round, "Round"),
                (LineCap::Square, "Square"),
            ],
        );
        choice(
            ui,
            "line_join",
            "Join",
            &mut stroke.join,
            &[
                (LineJoin::Miter, "Miter"),
                (LineJoin::Round, "Round"),
                (LineJoin::Bevel, "Bevel"),
            ],
        );
        if stroke.join == LineJoin::Miter {
            number_row(
                ui,
                "Miter limit",
                &mut stroke.miter_limit,
                1.0..=20.0,
                0.1,
                "",
            );
        }
    }
    shape_specific_line(ui, shape);
}

/// Line endings and routing for a line; corner radius and point counts for auto shapes.
fn shape_specific_line(ui: &mut Ui, shape: &mut Shape) {
    let (w, h) = (shape.common.bounds.width(), shape.common.bounds.height());
    match &mut shape.kind {
        ShapeKind::Line(line) => {
            ui.add_space(10.0);
            ui.label(egui::RichText::new("Line ends").strong());
            choice(
                ui,
                "connector",
                "Type",
                &mut line.connector_type,
                &[
                    (ConnectorType::Straight, "Straight"),
                    (ConnectorType::Elbow, "Elbow"),
                    (ConnectorType::Curved, "Curved"),
                ],
            );
            ending_editor(ui, "start_ending", "Start", &mut line.start_ending);
            ending_editor(ui, "end_ending", "End", &mut line.end_ending);
        }
        ShapeKind::AutoShape(auto) => match &mut auto.geometry {
            ShapeGeometry::Rectangle { corner_radius } => {
                ui.add_space(10.0);
                number_row(
                    ui,
                    "Corner radius",
                    corner_radius,
                    0.0..=(w.min(h) / 2.0).max(0.0),
                    0.5,
                    " pt",
                );
            }
            ShapeGeometry::Polygon { sides } => {
                ui.add_space(10.0);
                let mut n = *sides as f64;
                if number_row(ui, "Sides", &mut n, 3.0..=64.0, 0.1, "") {
                    *sides = n.round() as u32;
                }
            }
            ShapeGeometry::Star {
                points,
                inner_ratio,
            } => {
                ui.add_space(10.0);
                let mut n = *points as f64;
                if number_row(ui, "Points", &mut n, 3.0..=64.0, 0.1, "") {
                    *points = n.round() as u32;
                }
                let mut ratio = *inner_ratio * 100.0;
                if number_row(ui, "Inner radius", &mut ratio, 5.0..=95.0, 1.0, "%") {
                    *inner_ratio = ratio / 100.0;
                }
            }
            _ => {}
        },
        _ => {}
    }
}

fn ending_editor(ui: &mut Ui, id: &str, label: &str, ending: &mut LineEnding) {
    choice(
        ui,
        id,
        label,
        &mut ending.style,
        &[
            (LineEndingStyle::None, "None"),
            (LineEndingStyle::Arrow, "Arrow"),
            (LineEndingStyle::OpenArrow, "Open arrow"),
            (LineEndingStyle::StealthArrow, "Stealth arrow"),
            (LineEndingStyle::Diamond, "Diamond"),
            (LineEndingStyle::Circle, "Circle"),
            (LineEndingStyle::Square, "Square"),
        ],
    );
    if ending.style != LineEndingStyle::None {
        row(ui, "", |ui| {
            number(ui, &mut ending.size, 1.0..=10.0, 0.1, "× width");
            ui.checkbox(&mut ending.filled, "Filled");
        });
    }
}

/// An optional effect: a checkbox that creates it from its default, and its fields when on.
fn effect<T: Default>(
    ui: &mut Ui,
    title: &str,
    slot: &mut Option<T>,
    fields: impl FnOnce(&mut Ui, &mut T),
) {
    let mut on = slot.is_some();
    if ui
        .checkbox(&mut on, egui::RichText::new(title).strong())
        .changed()
    {
        *slot = on.then(T::default);
    }
    if let Some(value) = slot {
        ui.indent(title, |ui| fields(ui, value));
    }
    ui.add_space(4.0);
}

fn effects_tab(ui: &mut Ui, shape: &mut Shape) {
    let e: &mut Effects = &mut shape.effects;
    effect(
        ui,
        "Shadow",
        &mut e.drop_shadow,
        |ui, s: &mut DropShadow| {
            row(ui, "Color", |ui| color_edit(ui, &mut s.color));
            number_row(ui, "Offset x", &mut s.offset_x, -200.0..=200.0, 0.5, " pt");
            number_row(ui, "Offset y", &mut s.offset_y, -200.0..=200.0, 0.5, " pt");
            number_row(ui, "Blur", &mut s.blur_radius, 0.0..=100.0, 0.5, " pt");
            number_row(ui, "Spread", &mut s.spread, -50.0..=100.0, 0.5, " pt");
        },
    );
    effect(ui, "Glow", &mut e.outer_glow, |ui, g: &mut Glow| {
        row(ui, "Color", |ui| color_edit(ui, &mut g.color));
        number_row(ui, "Radius", &mut g.radius, 0.0..=100.0, 0.5, " pt");
        number_row(ui, "Spread", &mut g.spread, 0.0..=100.0, 0.5, " pt");
    });
    effect(
        ui,
        "Inner shadow",
        &mut e.inner_shadow,
        |ui, s: &mut InnerShadow| {
            row(ui, "Color", |ui| color_edit(ui, &mut s.color));
            number_row(ui, "Offset x", &mut s.offset_x, -100.0..=100.0, 0.5, " pt");
            number_row(ui, "Offset y", &mut s.offset_y, -100.0..=100.0, 0.5, " pt");
            number_row(ui, "Blur", &mut s.blur_radius, 0.0..=100.0, 0.5, " pt");
            number_row(ui, "Choke", &mut s.choke, 0.0..=100.0, 0.5, " pt");
        },
    );
    effect(
        ui,
        "Inner glow",
        &mut e.inner_glow,
        |ui, g: &mut InnerGlow| {
            row(ui, "Color", |ui| color_edit(ui, &mut g.color));
            number_row(ui, "Radius", &mut g.radius, 0.0..=100.0, 0.5, " pt");
            choice(
                ui,
                "inner_glow_source",
                "Source",
                &mut g.source,
                &[
                    (InnerGlowSource::Edge, "Edge"),
                    (InnerGlowSource::Center, "Center"),
                ],
            );
            number_row(ui, "Choke", &mut g.choke, 0.0..=100.0, 0.5, " pt");
        },
    );
    effect(
        ui,
        "Reflection",
        &mut e.reflection,
        |ui, r: &mut Reflection| {
            let mut opacity = r.opacity * 100.0;
            if number_row(ui, "Opacity", &mut opacity, 0.0..=100.0, 1.0, "%") {
                r.opacity = opacity / 100.0;
            }
            let mut size = r.size * 100.0;
            if number_row(ui, "Size", &mut size, 0.0..=100.0, 1.0, "%") {
                r.size = size / 100.0;
            }
            number_row(ui, "Distance", &mut r.gap, 0.0..=100.0, 0.5, " pt");
            number_row(ui, "Blur", &mut r.blur, 0.0..=100.0, 0.5, " pt");
            number_row(
                ui,
                "Fade length",
                &mut r.fade_length,
                0.0..=100.0,
                0.5,
                " pt",
            );
            number_row(ui, "Direction", &mut r.direction, 0.0..=360.0, 1.0, "°");
        },
    );
    effect(
        ui,
        "Soft edges",
        &mut e.soft_edges,
        |ui, s: &mut SoftEdges| {
            number_row(ui, "Radius", &mut s.radius, 0.0..=100.0, 0.5, " pt");
        },
    );
    effect(ui, "Bevel", &mut e.bevel, |ui, b: &mut Bevel| {
        choice(
            ui,
            "bevel_style",
            "Style",
            &mut b.style,
            &[
                (BevelStyle::Outer, "Outer"),
                (BevelStyle::Inner, "Inner"),
                (BevelStyle::Emboss, "Emboss"),
                (BevelStyle::PillowEmboss, "Pillow emboss"),
            ],
        );
        choice(
            ui,
            "bevel_technique",
            "Technique",
            &mut b.technique,
            &[
                (BevelTechnique::Smooth, "Smooth"),
                (BevelTechnique::ChiselHard, "Chisel hard"),
                (BevelTechnique::ChiselSoft, "Chisel soft"),
            ],
        );
        number_row(ui, "Depth", &mut b.depth, 0.0..=100.0, 0.5, " pt");
        number_row(ui, "Size", &mut b.size, 0.0..=100.0, 0.5, " pt");
        number_row(ui, "Soften", &mut b.soften, 0.0..=100.0, 0.5, " pt");
        number_row(ui, "Light angle", &mut b.light_angle, 0.0..=360.0, 1.0, "°");
        number_row(
            ui,
            "Light altitude",
            &mut b.light_altitude,
            0.0..=90.0,
            1.0,
            "°",
        );
        row(ui, "Highlight", |ui| color_edit(ui, &mut b.highlight_color));
        row(ui, "Shadow", |ui| color_edit(ui, &mut b.shadow_color));
    });
    ui.label(
        egui::RichText::new(
            "Shadow and glow are drawn on the canvas; the other effects are saved with the shape.",
        )
        .weak()
        .small(),
    );
}

fn size_tab(
    ui: &mut Ui,
    shape: &mut Shape,
    single: bool,
    units: crate::state::DisplayUnit,
    lock_aspect: &mut bool,
) {
    if !single {
        ui.label("Select one object to change its size and position.");
        return;
    }
    let suffix = units.suffix();
    let b = shape.common.bounds;
    let (mut x, mut y) = (units.from_points(b.origin.x), units.from_points(b.origin.y));
    let (mut w, mut h) = (units.from_points(b.width()), units.from_points(b.height()));
    let ratio = if b.height() > 1e-9 {
        b.width() / b.height()
    } else {
        1.0
    };
    let speed = units.from_points(0.5);
    let big = units.from_points(20000.0);
    let mut bounds = b;
    if number_row(ui, "X", &mut x, -big..=big, speed, suffix) {
        bounds.origin.x = units.to_points(x);
    }
    if number_row(ui, "Y", &mut y, -big..=big, speed, suffix) {
        bounds.origin.y = units.to_points(y);
    }
    let min = units.from_points(0.0);
    if number_row(ui, "Width", &mut w, min..=big, speed, suffix) {
        bounds.size.width = units.to_points(w);
        if *lock_aspect && ratio > 0.0 {
            bounds.size.height = bounds.size.width / ratio;
        }
    }
    if number_row(ui, "Height", &mut h, min..=big, speed, suffix) {
        bounds.size.height = units.to_points(h);
        if *lock_aspect {
            bounds.size.width = bounds.size.height * ratio;
        }
    }
    shape.common.bounds = Rect::new(
        bounds.origin.x,
        bounds.origin.y,
        bounds.size.width,
        bounds.size.height,
    );
    row(ui, "", |ui| ui.checkbox(lock_aspect, "Lock aspect ratio"));
    ui.add_space(6.0);
    number_row(
        ui,
        "Rotation",
        &mut shape.common.rotation,
        -360.0..=360.0,
        1.0,
        "°",
    );
    row(ui, "Flip", |ui| {
        ui.checkbox(&mut shape.common.flip_h, "Horizontal");
        ui.checkbox(&mut shape.common.flip_v, "Vertical");
    });
}

fn margins_are_zero(m: &Margins) -> bool {
    m.top == 0.0 && m.bottom == 0.0 && m.left == 0.0 && m.right == 0.0
}

fn text_box_tab(ui: &mut Ui, shape: &mut Shape) {
    let ShapeKind::TextFrame(tf) = &mut shape.kind else {
        return;
    };
    choice(
        ui,
        "text_vertical",
        "Vertical alignment",
        &mut tf.vertical_align,
        &[
            (VerticalAlignment::Top, "Top"),
            (VerticalAlignment::Center, "Middle"),
            (VerticalAlignment::Bottom, "Bottom"),
        ],
    );
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Margins").strong());
    let mut m = tf.effective_margins();
    let mut changed = false;
    changed |= number_row(ui, "Top", &mut m.top, 0.0..=200.0, 0.5, " pt");
    changed |= number_row(ui, "Bottom", &mut m.bottom, 0.0..=200.0, 0.5, " pt");
    changed |= number_row(ui, "Left", &mut m.left, 0.0..=200.0, 0.5, " pt");
    changed |= number_row(ui, "Right", &mut m.right, 0.0..=200.0, 0.5, " pt");
    if changed {
        tf.margins = m;
        if margins_are_zero(&m) {
            tf.inset = 0.0;
        }
    }
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Columns").strong());
    let mut count = f64::from(tf.columns.count);
    if number_row(ui, "Number", &mut count, 1.0..=10.0, 0.1, "") {
        tf.columns.count = count.round() as u32;
    }
    number_row(ui, "Spacing", &mut tf.columns.gap, 0.0..=200.0, 0.5, " pt");
    ui.add_space(6.0);
    choice(
        ui,
        "text_fit",
        "Text fit",
        &mut tf.text_fit,
        &[
            (TextFitMode::None, "Do not autofit"),
            (TextFitMode::ShrinkOnOverflow, "Shrink on overflow"),
            (TextFitMode::GrowToFit, "Grow to fit"),
            (TextFitMode::BestFit, "Best fit"),
        ],
    );
}

fn properties_tab(ui: &mut Ui, shape: &mut Shape) {
    row(ui, "Name", |ui| {
        let mut name = shape.common.name.clone().unwrap_or_default();
        if ui
            .add(egui::TextEdit::singleline(&mut name).desired_width(240.0))
            .changed()
        {
            shape.common.name = (!name.is_empty()).then_some(name);
        }
    });
    row(ui, "", |ui| {
        ui.checkbox(&mut shape.common.visible, "Visible")
    });
    row(ui, "", |ui| {
        ui.checkbox(
            &mut shape.common.locked,
            "Locked (can't be selected or edited)",
        )
    });
    if let ShapeKind::Picture(pic) = &mut shape.kind {
        ui.add_space(10.0);
        ui.label(egui::RichText::new("Picture").strong());
        row(ui, "Alternative text", |ui| {
            let mut alt = pic.alt_text.clone().unwrap_or_default();
            if ui
                .add(
                    egui::TextEdit::multiline(&mut alt)
                        .desired_rows(2)
                        .desired_width(240.0),
                )
                .changed()
            {
                pic.alt_text = (!alt.is_empty()).then_some(alt);
            }
        });
        choice(
            ui,
            "picture_fit",
            "Fit",
            &mut pic.fit,
            &[
                (ImageFit::Stretch, "Stretch"),
                (ImageFit::Fill, "Fill (crop to frame)"),
                (ImageFit::Fit, "Fit inside frame"),
                (ImageFit::Tile, "Tile"),
                (ImageFit::Center, "Center at natural size"),
            ],
        );
        let crop = pic.crop.unwrap_or_else(|| Rect::new(0.0, 0.0, 1.0, 1.0));
        let (mut left, mut top) = (crop.origin.x * 100.0, crop.origin.y * 100.0);
        let (mut right, mut bottom) = (
            (1.0 - crop.origin.x - crop.width()) * 100.0,
            (1.0 - crop.origin.y - crop.height()) * 100.0,
        );
        let mut changed = false;
        changed |= number_row(ui, "Crop left", &mut left, 0.0..=95.0, 0.5, "%");
        changed |= number_row(ui, "Crop top", &mut top, 0.0..=95.0, 0.5, "%");
        changed |= number_row(ui, "Crop right", &mut right, 0.0..=95.0, 0.5, "%");
        changed |= number_row(ui, "Crop bottom", &mut bottom, 0.0..=95.0, 0.5, "%");
        if changed {
            let width = (100.0 - left - right).max(5.0) / 100.0;
            let height = (100.0 - top - bottom).max(5.0) / 100.0;
            let unchanged = left == 0.0 && top == 0.0 && right == 0.0 && bottom == 0.0;
            pic.crop = (!unchanged).then(|| Rect::new(left / 100.0, top / 100.0, width, height));
        }
        let (mut brightness, mut contrast) = (pic.brightness * 100.0, pic.contrast * 100.0);
        if number_row(ui, "Brightness", &mut brightness, -100.0..=100.0, 1.0, "%") {
            pic.brightness = brightness / 100.0;
        }
        if number_row(ui, "Contrast", &mut contrast, -100.0..=100.0, 1.0, "%") {
            pic.contrast = contrast / 100.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Shape {
        Shape::rectangle(Rect::new(10.0, 10.0, 100.0, 50.0))
    }

    #[test]
    fn only_changed_parts_are_applied() {
        let original = rect();
        let mut work = original.clone();
        work.fill = FillKind::Linear.make(&work.fill);
        work.opacity = 0.5;
        // A second selected shape keeps its own line and name
        let mut other = rect();
        other.line = Some(Stroke::new(Color::from_rgb8(255, 0, 0), 3.0));
        other.common.name = Some("keep".into());
        apply_to(&mut other, &original, &work, false);
        assert!(matches!(other.fill, Fill::LinearGradient(_)));
        assert_eq!(other.opacity, 0.5);
        assert_eq!(other.line.as_ref().map(|l| l.width), Some(3.0));
        assert_eq!(other.common.name.as_deref(), Some("keep"));
    }

    #[test]
    fn size_and_position_apply_to_a_single_shape() {
        let original = rect();
        let mut work = original.clone();
        work.common.bounds = Rect::new(0.0, 0.0, 200.0, 100.0);
        work.common.rotation = 30.0;
        let mut target = rect();
        apply_to(&mut target, &original, &work, true);
        assert_eq!(target.common.bounds, Rect::new(0.0, 0.0, 200.0, 100.0));
        assert_eq!(target.common.rotation, 30.0);
        let mut several = rect();
        apply_to(&mut several, &original, &work, false);
        assert_eq!(several.common.bounds, original.common.bounds);
    }

    #[test]
    fn fill_kinds_round_trip() {
        let solid = Fill::Solid(Color::from_rgb8(10, 20, 30));
        for kind in [
            FillKind::None,
            FillKind::Solid,
            FillKind::Linear,
            FillKind::Radial,
            FillKind::Pattern,
            FillKind::Picture,
        ] {
            assert!(FillKind::of(&kind.make(&solid)) == kind);
        }
        assert_eq!(
            FillKind::Radial.make(&solid).primary_color(),
            solid.primary_color()
        );
    }

    #[test]
    fn dashes_are_recognized_by_weight() {
        let mut stroke = Stroke::new(Color::BLACK, 2.0);
        assert_eq!(dash_name(&stroke), "Solid");
        stroke.dash_pattern = Some(vec![8.0, 6.0]);
        assert_eq!(dash_name(&stroke), "Dashes");
        stroke.dash_pattern = Some(vec![5.0, 1.0]);
        assert_eq!(dash_name(&stroke), "Custom");
    }
}
