//! Paragraph dialog: alignment, indents, spacing, lists and keep options.

use egui::Ui;
use repub_model::styles::{LineSpacing, ListStyle, NumberFormat, ParagraphStyle, TextAlignment};

use super::controls::{choice, number, number_row, ok_cancel, row};
use crate::state::{AppState, DisplayUnit};
use crate::text_format::{self, ParaPatch};

/// How the line spacing is given.
#[derive(Clone, Copy, PartialEq, Eq)]
enum SpacingMode {
    Single,
    OneAndHalf,
    Double,
    Multiple,
    Exactly,
    AtLeast,
}

impl SpacingMode {
    fn of(spacing: &LineSpacing) -> Self {
        match spacing {
            LineSpacing::Single => Self::Single,
            LineSpacing::OneAndHalf => Self::OneAndHalf,
            LineSpacing::Double => Self::Double,
            LineSpacing::Multiple(_) => Self::Multiple,
            LineSpacing::Exact(_) => Self::Exactly,
            LineSpacing::AtLeast(_) => Self::AtLeast,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ListKind {
    None,
    Bullets,
    Numbering,
}

const BULLETS: [char; 6] = ['•', '◦', '▪', '–', '*', '>'];

/// The Paragraph dialog.
#[derive(Default)]
pub struct ParagraphDialog {
    pub open: bool,
    initial: ParagraphStyle,
    work: ParagraphStyle,
    mixed: bool,
    units: DisplayUnit,
}

impl ParagraphDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_for(&mut self, state: &AppState) {
        let Some((style, mixed)) = text_format::paragraph_snapshot(state) else { return };
        self.initial = style.clone();
        self.work = style;
        self.mixed = mixed;
        self.units = state.units;
        self.open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) {
        if !self.open {
            return;
        }
        let mut result = None;
        egui::Window::new("Paragraph").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
            ui.set_min_width(400.0);
            if self.mixed {
                ui.label(egui::RichText::new("The selection mixes paragraph styles; only what you change is applied.").weak().small());
            }
            self.fields(ui);
            result = ok_cancel(ui);
        });
        match result {
            Some(true) => {
                let patch = self.patch();
                text_format::apply_paragraph_patch(state, &patch);
                self.open = false;
            }
            Some(false) => self.open = false,
            None => {}
        }
    }

    fn fields(&mut self, ui: &mut Ui) {
        let units = self.units;
        let p = &mut self.work;
        choice(
            ui,
            "para_align",
            "Alignment",
            &mut p.alignment,
            &[(TextAlignment::Left, "Left"), (TextAlignment::Center, "Center"), (TextAlignment::Right, "Right"), (TextAlignment::Justify, "Justify")],
        );
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Indentation").strong());
        for (label, value) in [("Left", &mut p.left_indent), ("Right", &mut p.right_indent)] {
            let mut shown = units.from_points(*value);
            if number_row(ui, label, &mut shown, 0.0..=units.from_points(1440.0), units.from_points(0.5), units.suffix()) {
                *value = units.to_points(shown);
            }
        }
        let mut first = units.from_points(p.first_line_indent);
        let big = units.from_points(1440.0);
        if number_row(ui, "First line", &mut first, -big..=big, units.from_points(0.5), units.suffix()) {
            p.first_line_indent = units.to_points(first);
        }
        row(ui, "", |ui| ui.label(egui::RichText::new("Negative hangs the first line out to the left.").weak().small()));
        ui.add_space(6.0);
        ui.label(egui::RichText::new("Spacing").strong());
        number_row(ui, "Before", &mut p.space_before, 0.0..=720.0, 0.5, " pt");
        number_row(ui, "After", &mut p.space_after, 0.0..=720.0, 0.5, " pt");
        let mut mode = SpacingMode::of(&p.line_spacing);
        let before = mode;
        choice(
            ui,
            "para_line_spacing",
            "Line spacing",
            &mut mode,
            &[
                (SpacingMode::Single, "Single"),
                (SpacingMode::OneAndHalf, "1.5 lines"),
                (SpacingMode::Double, "Double"),
                (SpacingMode::Multiple, "Multiple"),
                (SpacingMode::Exactly, "Exactly"),
                (SpacingMode::AtLeast, "At least"),
            ],
        );
        if mode != before {
            p.line_spacing = match mode {
                SpacingMode::Single => LineSpacing::Single,
                SpacingMode::OneAndHalf => LineSpacing::OneAndHalf,
                SpacingMode::Double => LineSpacing::Double,
                SpacingMode::Multiple => LineSpacing::Multiple(1.15),
                SpacingMode::Exactly => LineSpacing::Exact(14.0),
                SpacingMode::AtLeast => LineSpacing::AtLeast(12.0),
            };
        }
        match &mut p.line_spacing {
            LineSpacing::Multiple(m) => {
                number_row(ui, "At", m, 0.1..=10.0, 0.05, "×");
            }
            LineSpacing::Exact(v) | LineSpacing::AtLeast(v) => {
                number_row(ui, "At", v, 1.0..=1000.0, 0.5, " pt");
            }
            _ => {}
        }
        ui.add_space(6.0);
        ui.label(egui::RichText::new("List").strong());
        let mut kind = match p.list_style {
            ListStyle::None => ListKind::None,
            ListStyle::Bullet { .. } => ListKind::Bullets,
            ListStyle::Numbered { .. } => ListKind::Numbering,
        };
        let before = kind;
        choice(ui, "para_list", "Type", &mut kind, &[(ListKind::None, "None"), (ListKind::Bullets, "Bullets"), (ListKind::Numbering, "Numbering")]);
        if kind != before {
            p.list_style = match kind {
                ListKind::None => ListStyle::None,
                ListKind::Bullets => ListStyle::Bullet { character: '•', indent: 18.0 },
                ListKind::Numbering => ListStyle::Numbered { format: NumberFormat::Arabic, start: 1, indent: 18.0 },
            };
        }
        match &mut p.list_style {
            ListStyle::Bullet { character, indent } => {
                row(ui, "Bullet", |ui| {
                    egui::ComboBox::from_id_salt("para_bullet").selected_text(character.to_string()).show_ui(ui, |ui| {
                        for b in BULLETS {
                            ui.selectable_value(character, b, b.to_string());
                        }
                    });
                });
                number_row(ui, "Indent", indent, 0.0..=360.0, 0.5, " pt");
            }
            ListStyle::Numbered { format, start, indent } => {
                choice(
                    ui,
                    "para_number_format",
                    "Format",
                    format,
                    &[
                        (NumberFormat::Arabic, "1, 2, 3"),
                        (NumberFormat::LowerAlpha, "a, b, c"),
                        (NumberFormat::UpperAlpha, "A, B, C"),
                        (NumberFormat::LowerRoman, "i, ii, iii"),
                        (NumberFormat::UpperRoman, "I, II, III"),
                    ],
                );
                row(ui, "Start at", |ui| {
                    let mut n = f64::from(*start);
                    if number(ui, &mut n, 1.0..=9999.0, 0.2, "") {
                        *start = n.round() as u32;
                    }
                });
                number_row(ui, "Indent", indent, 0.0..=360.0, 0.5, " pt");
            }
            ListStyle::None => {}
        }
        ui.add_space(6.0);
        row(ui, "Keep", |ui| {
            ui.checkbox(&mut p.keep_with_next, "With next");
            ui.checkbox(&mut p.keep_together, "Lines together");
        });
    }

    /// What the dialog changed from the style it opened with.
    fn patch(&self) -> ParaPatch {
        let (a, b) = (&self.initial, &self.work);
        ParaPatch {
            alignment: (a.alignment != b.alignment).then_some(b.alignment),
            line_spacing: (a.line_spacing != b.line_spacing).then_some(b.line_spacing),
            space_before: (a.space_before != b.space_before).then_some(b.space_before),
            space_after: (a.space_after != b.space_after).then_some(b.space_after),
            first_line_indent: (a.first_line_indent != b.first_line_indent).then_some(b.first_line_indent),
            left_indent: (a.left_indent != b.left_indent).then_some(b.left_indent),
            right_indent: (a.right_indent != b.right_indent).then_some(b.right_indent),
            list_style: (a.list_style != b.list_style).then(|| b.list_style.clone()),
            keep_with_next: (a.keep_with_next != b.keep_with_next).then_some(b.keep_with_next),
            keep_together: (a.keep_together != b.keep_together).then_some(b.keep_together),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_changed_properties_enter_the_patch() {
        let mut dialog = ParagraphDialog::new();
        dialog.initial = ParagraphStyle::default();
        dialog.work = ParagraphStyle::default();
        assert!(dialog.patch().is_empty());
        dialog.work.space_after = 6.0;
        dialog.work.line_spacing = LineSpacing::Exact(20.0);
        dialog.work.list_style = ListStyle::Numbered { format: NumberFormat::UpperRoman, start: 4, indent: 24.0 };
        let patch = dialog.patch();
        assert_eq!(patch.space_after, Some(6.0));
        assert_eq!(patch.line_spacing, Some(LineSpacing::Exact(20.0)));
        assert!(matches!(patch.list_style, Some(ListStyle::Numbered { start: 4, .. })));
        assert_eq!(patch.alignment, None);
    }
}
