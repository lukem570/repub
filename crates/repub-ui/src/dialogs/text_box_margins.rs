//! Text Box Margins dialog.

use egui::DragValue;
use repub_model::ObjectId;
use repub_model::shape::{Margins, Shape, ShapeKind};

use crate::state::AppState;
use crate::theme::{colors, fonts, section_header};

/// Dialog for editing the margins inside a text frame.
#[derive(Debug, Clone)]
pub struct TextBoxMarginsDialog {
    /// Whether the dialog is open
    pub open: bool,
    /// Object being edited
    pub object_id: Option<ObjectId>,
    /// Working copy of the margins
    pub margins: Margins,
    /// Values on open, for Cancel
    original: Margins,
}

impl Default for TextBoxMarginsDialog {
    fn default() -> Self {
        Self {
            open: false,
            object_id: None,
            margins: Margins::uniform(4.0),
            original: Margins::uniform(4.0),
        }
    }
}

impl TextBoxMarginsDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog for a text frame.
    pub fn open_for_text_frame(&mut self, state: &AppState, obj_id: ObjectId) {
        if let Some(Shape { kind: ShapeKind::TextFrame(tf), .. }) = Self::get_object(state, obj_id) {
            self.object_id = Some(obj_id);
            self.margins = tf.effective_margins();
            self.original = self.margins;
            self.open = true;
        }
    }

    fn get_object(state: &AppState, obj_id: ObjectId) -> Option<Shape> {
        state.current_page()?.get_shape(obj_id).cloned()
    }

    /// Show the dialog. Returns Some(true) on OK, Some(false) on Cancel.
    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) -> Option<bool> {
        if !self.open {
            return None;
        }
        let mut result = None;

        egui::Window::new("Text Box Margins")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(300.0);
                section_header(ui, "Margins (points)");
                ui.add_space(4.0);

                egui::Grid::new("text_box_margins_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Left").font(fonts::body()));
                        ui.add(DragValue::new(&mut self.margins.left).speed(0.5).range(0.0..=144.0));
                        ui.end_row();

                        ui.label(egui::RichText::new("Right").font(fonts::body()));
                        ui.add(DragValue::new(&mut self.margins.right).speed(0.5).range(0.0..=144.0));
                        ui.end_row();

                        ui.label(egui::RichText::new("Top").font(fonts::body()));
                        ui.add(DragValue::new(&mut self.margins.top).speed(0.5).range(0.0..=144.0));
                        ui.end_row();

                        ui.label(egui::RichText::new("Bottom").font(fonts::body()));
                        ui.add(DragValue::new(&mut self.margins.bottom).speed(0.5).range(0.0..=144.0));
                        ui.end_row();
                    });

                ui.add_space(8.0);
                if ui.button("Apply to all sides").clicked() {
                    let avg = (self.margins.left + self.margins.right + self.margins.top + self.margins.bottom) / 4.0;
                    self.margins = Margins::uniform(avg);
                }

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.open = false;
                        result = Some(false);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("OK").fill(colors::accent())).clicked() {
                            if let Some(obj_id) = self.object_id {
                                self.apply(state, obj_id);
                            }
                            self.open = false;
                            result = Some(true);
                        }
                    });
                });
            });

        result
    }

    fn apply(&self, state: &mut AppState, obj_id: ObjectId) {
        state.record_undo("Text Box Margins");
        if let Some(page) = state.current_page_mut() {
            if let Some(Shape { kind: ShapeKind::TextFrame(tf), .. }) = page.get_shape_mut(obj_id) {
                tf.margins = self.margins;
                // All-zero margins read as unset (the uniform inset applies), so zero the inset
                // too for a box that really has none
                let m = self.margins;
                if m.top == 0.0 && m.bottom == 0.0 && m.left == 0.0 && m.right == 0.0 {
                    tf.inset = 0.0;
                }
            }
        }
        state.mark_dirty();
    }
}
