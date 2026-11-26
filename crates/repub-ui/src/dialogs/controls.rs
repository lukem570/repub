//! Small controls shared by the property dialogs.

use egui::{Color32, DragValue, Ui};
use repub_color::Color;

use crate::theme::colors;

pub fn to_color32(c: &Color) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (c.r * 255.0).round() as u8,
        (c.g * 255.0).round() as u8,
        (c.b * 255.0).round() as u8,
        (c.a * 255.0).round() as u8,
    )
}

pub fn from_color32(c: Color32) -> Color {
    let [r, g, b, a] = c.to_srgba_unmultiplied();
    Color::from_rgba8(r, g, b, a)
}

/// A color swatch that opens egui's picker (with alpha). Returns true when it changed.
pub fn color_edit(ui: &mut Ui, color: &mut Color) -> bool {
    let mut c = to_color32(color);
    let changed = egui::color_picker::color_edit_button_srgba(
        ui,
        &mut c,
        egui::color_picker::Alpha::OnlyBlend,
    )
    .changed();
    if changed {
        *color = from_color32(c);
    }
    changed
}

/// A labeled row: the label in a fixed-width column, then the control.
pub fn row<R>(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui) -> R) -> R {
    ui.horizontal(|ui| {
        ui.add_sized([110.0, 20.0], egui::Label::new(label));
        add(ui)
    })
    .inner
}

/// A number field with a suffix and range. Returns true when it changed.
pub fn number(
    ui: &mut Ui,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    speed: f64,
    suffix: &str,
) -> bool {
    ui.add(
        DragValue::new(value)
            .range(range)
            .speed(speed)
            .max_decimals(2)
            .suffix(suffix),
    )
    .changed()
}

/// A labeled number field.
pub fn number_row(
    ui: &mut Ui,
    label: &str,
    value: &mut f64,
    range: std::ops::RangeInclusive<f64>,
    speed: f64,
    suffix: &str,
) -> bool {
    row(ui, label, |ui| number(ui, value, range, speed, suffix))
}

/// A labeled combo box over `options` (value, label). Returns true when the choice changed.
pub fn choice<T: Copy + PartialEq>(
    ui: &mut Ui,
    id: &str,
    label: &str,
    value: &mut T,
    options: &[(T, &str)],
) -> bool {
    row(ui, label, |ui| {
        let selected = options
            .iter()
            .find(|(v, _)| v == value)
            .map_or("", |(_, n)| *n);
        let mut changed = false;
        egui::ComboBox::from_id_salt(id)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                for (v, name) in options {
                    if ui.selectable_label(value == v, *name).clicked() && value != v {
                        *value = *v;
                        changed = true;
                    }
                }
            });
        changed
    })
}

/// A row of tab buttons; returns the chosen index.
pub fn tab_strip(ui: &mut Ui, current: usize, names: &[&str]) -> usize {
    let mut chosen = current;
    ui.horizontal(|ui| {
        for (i, name) in names.iter().enumerate() {
            if ui.selectable_label(i == current, *name).clicked() {
                chosen = i;
            }
        }
    });
    ui.separator();
    chosen
}

/// OK / Cancel buttons. `Some(true)` for OK, `Some(false)` for Cancel.
pub fn ok_cancel(ui: &mut Ui) -> Option<bool> {
    let mut result = None;
    ui.add_space(8.0);
    ui.separator();
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        if ui.button("Cancel").clicked() {
            result = Some(false);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new("OK").fill(colors::accent()))
                .clicked()
            {
                result = Some(true);
            }
        });
    });
    result
}
