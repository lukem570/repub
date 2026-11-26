//! Font dialog: every character property of the selected text.

use egui::Ui;
use repub_color::Color;
use repub_model::styles::{
    BaselineShift, CharacterStyle, FontStyle, FontWeight, StrikethroughStyle, UnderlineStyle,
};

use super::controls::{choice, color_edit, number_row, ok_cancel, row};
use crate::state::AppState;
use crate::text_format::{self, CharChange, CharPatch};

/// Weight and slant together, as a Font style list shows them.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Face {
    Regular,
    Italic,
    Bold,
    BoldItalic,
}

impl Face {
    fn of(style: &CharacterStyle) -> Self {
        let bold = style.font.weight.weight() >= 600;
        let italic = style.font.style != FontStyle::Normal;
        match (bold, italic) {
            (false, false) => Self::Regular,
            (false, true) => Self::Italic,
            (true, false) => Self::Bold,
            (true, true) => Self::BoldItalic,
        }
    }

    fn set(self, style: &mut CharacterStyle) {
        let (bold, italic) = match self {
            Self::Regular => (false, false),
            Self::Italic => (false, true),
            Self::Bold => (true, false),
            Self::BoldItalic => (true, true),
        };
        style.font.weight = if bold {
            FontWeight::Bold
        } else {
            FontWeight::Normal
        };
        style.font.style = if italic {
            FontStyle::Italic
        } else {
            FontStyle::Normal
        };
    }
}

/// The Font dialog.
#[derive(Default)]
pub struct FontDialog {
    pub open: bool,
    initial: CharacterStyle,
    work: CharacterStyle,
    mixed: bool,
    face: Option<Face>,
}

impl FontDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open on the text being edited or the selected text boxes.
    pub fn open_for(&mut self, state: &AppState) {
        let Some((style, mixed)) = text_format::character_snapshot(state) else {
            return;
        };
        self.initial = style.clone();
        self.work = style;
        self.mixed = mixed;
        self.face = None;
        self.open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) {
        if !self.open {
            return;
        }
        let mut result = None;
        egui::Window::new("Font")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(400.0);
                if self.mixed {
                    ui.label(
                        egui::RichText::new(
                            "The selection mixes styles; only what you change is applied.",
                        )
                        .weak()
                        .small(),
                    );
                }
                self.fields(ui);
                result = ok_cancel(ui);
            });
        match result {
            Some(true) => {
                let patch = self.patch();
                if !patch.is_empty() {
                    text_format::apply(state, CharChange::Patch(patch));
                }
                self.open = false;
            }
            Some(false) => self.open = false,
            None => {}
        }
    }

    fn fields(&mut self, ui: &mut Ui) {
        let style = &mut self.work;
        row(ui, "Font", |ui| family_box(ui, &mut style.font.family));
        let mut face = self.face.unwrap_or_else(|| Face::of(style));
        if choice(
            ui,
            "font_face",
            "Font style",
            &mut face,
            &[
                (Face::Regular, "Regular"),
                (Face::Italic, "Italic"),
                (Face::Bold, "Bold"),
                (Face::BoldItalic, "Bold italic"),
            ],
        ) {
            face.set(style);
            self.face = Some(face);
        }
        number_row(ui, "Size", &mut style.font_size, 1.0..=1638.0, 0.5, " pt");
        row(ui, "Font color", |ui| color_edit(ui, &mut style.color));
        optional_color(
            ui,
            "Highlight",
            &mut style.background_color,
            Color::from_rgb8(255, 255, 0),
        );
        ui.add_space(6.0);
        choice(
            ui,
            "font_underline",
            "Underline",
            &mut style.underline,
            &[
                (UnderlineStyle::None, "None"),
                (UnderlineStyle::Single, "Single"),
                (UnderlineStyle::Double, "Double"),
                (UnderlineStyle::Dotted, "Dotted"),
                (UnderlineStyle::Dashed, "Dashed"),
                (UnderlineStyle::Wavy, "Wavy"),
            ],
        );
        if style.underline != UnderlineStyle::None {
            optional_color(
                ui,
                "Underline color",
                &mut style.underline_color,
                style.color,
            );
        }
        choice(
            ui,
            "font_strike",
            "Strikethrough",
            &mut style.strikethrough,
            &[
                (StrikethroughStyle::None, "None"),
                (StrikethroughStyle::Single, "Single"),
                (StrikethroughStyle::Double, "Double"),
            ],
        );
        choice(
            ui,
            "font_position",
            "Position",
            &mut style.baseline_shift,
            &[
                (BaselineShift::Normal, "Normal"),
                (BaselineShift::Superscript, "Superscript"),
                (BaselineShift::Subscript, "Subscript"),
            ],
        );
        number_row(
            ui,
            "Letter spacing",
            &mut style.letter_spacing,
            -10.0..=100.0,
            0.1,
            " pt",
        );
        row(ui, "Capitals", |ui| {
            ui.checkbox(&mut style.all_caps, "All caps");
            ui.checkbox(&mut style.small_caps, "Small caps");
        });
    }

    /// What the dialog changed from the style it opened with.
    fn patch(&self) -> CharPatch {
        let (a, b) = (&self.initial, &self.work);
        CharPatch {
            family: (a.font.family != b.font.family).then(|| b.font.family.clone()),
            weight: (a.font.weight != b.font.weight).then_some(b.font.weight),
            style: (a.font.style != b.font.style).then_some(b.font.style),
            stretch: None,
            size: (a.font_size != b.font_size).then_some(b.font_size),
            color: (a.color != b.color).then_some(b.color),
            background: (a.background_color != b.background_color).then_some(b.background_color),
            underline: (a.underline != b.underline).then_some(b.underline),
            underline_color: (a.underline_color != b.underline_color).then_some(b.underline_color),
            strikethrough: (a.strikethrough != b.strikethrough).then_some(b.strikethrough),
            baseline: (a.baseline_shift != b.baseline_shift).then_some(b.baseline_shift),
            letter_spacing: (a.letter_spacing != b.letter_spacing).then_some(b.letter_spacing),
            all_caps: (a.all_caps != b.all_caps).then_some(b.all_caps),
            small_caps: (a.small_caps != b.small_caps).then_some(b.small_caps),
        }
    }
}

/// A checkbox that turns an optional color on (with `default`) or off, and its swatch when on.
fn optional_color(ui: &mut Ui, label: &str, slot: &mut Option<Color>, default: Color) {
    row(ui, label, |ui| {
        let mut on = slot.is_some();
        if ui.checkbox(&mut on, "").changed() {
            *slot = on.then_some(default);
        }
        if let Some(color) = slot {
            color_edit(ui, color);
        }
    });
}

/// The family: a field with the installed families to pick from.
pub(crate) fn family_box(ui: &mut Ui, family: &mut String) {
    let filter_id = egui::Id::new("font_dialog_filter");
    egui::ComboBox::from_id_salt("font_dialog_family")
        .width(220.0)
        .height(320.0)
        .selected_text(family.clone())
        .show_ui(ui, |ui| {
            let Some(registry) = crate::sdf_text::fonts::try_registry() else {
                ui.label("Finding fonts…");
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(200));
                return;
            };
            let mut filter: String = ui.data_mut(|d| d.get_temp(filter_id).unwrap_or_default());
            ui.add(
                egui::TextEdit::singleline(&mut filter)
                    .hint_text("Search fonts")
                    .desired_width(200.0),
            );
            ui.data_mut(|d| d.insert_temp(filter_id, filter.clone()));
            let needle = filter.to_lowercase();
            for name in registry
                .family_names()
                .into_iter()
                .filter(|f| f.to_lowercase().contains(&needle))
            {
                if ui.selectable_label(*family == name, &name).clicked() {
                    *family = name;
                    ui.data_mut(|d| d.remove::<String>(filter_id));
                    ui.close_menu();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_changed_properties_enter_the_patch() {
        let mut dialog = FontDialog::new();
        dialog.initial = CharacterStyle::default();
        dialog.work = CharacterStyle::default();
        assert!(dialog.patch().is_empty());
        dialog.work.strikethrough = StrikethroughStyle::Single;
        Face::BoldItalic.set(&mut dialog.work);
        let patch = dialog.patch();
        assert_eq!(patch.strikethrough, Some(StrikethroughStyle::Single));
        assert_eq!(patch.weight, Some(FontWeight::Bold));
        assert_eq!(patch.style, Some(FontStyle::Italic));
        assert_eq!(patch.size, None);
        assert_eq!(patch.family, None);
    }

    #[test]
    fn faces_round_trip() {
        for face in [Face::Regular, Face::Italic, Face::Bold, Face::BoldItalic] {
            let mut style = CharacterStyle::default();
            face.set(&mut style);
            assert!(Face::of(&style) == face);
        }
    }
}
