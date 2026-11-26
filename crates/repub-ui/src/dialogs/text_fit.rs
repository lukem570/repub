//! Text Fit dialog for auto-sizing text to fit the frame.

use repub_model::ObjectId;
use repub_model::shape::{Shape, ShapeKind, TextFitMode};

use crate::state::AppState;
use crate::theme::{colors, fonts, section_header};

/// Text Fit dialog state.
#[derive(Debug, Clone)]
pub struct TextFitDialog {
    pub open: bool,
    pub object_id: Option<ObjectId>,
    pub fit_mode: TextFitMode,
    original_fit_mode: TextFitMode,
}

impl Default for TextFitDialog {
    fn default() -> Self {
        Self {
            open: false,
            object_id: None,
            fit_mode: TextFitMode::None,
            original_fit_mode: TextFitMode::None,
        }
    }
}

impl TextFitDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_for_text_frame(&mut self, state: &AppState, obj_id: ObjectId) {
        if let Some(Shape { kind: ShapeKind::TextFrame(tf), .. }) = Self::get_object(state, obj_id) {
            self.object_id = Some(obj_id);
            self.fit_mode = tf.text_fit;
            self.original_fit_mode = self.fit_mode;
            self.open = true;
        }
    }

    fn get_object(state: &AppState, obj_id: ObjectId) -> Option<Shape> {
        state.current_page()?.get_shape(obj_id).cloned()
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) -> Option<bool> {
        if !self.open {
            return None;
        }

        let mut result = None;

        egui::Window::new("Text Fit")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(320.0);

                section_header(ui, "Auto Fit");
                ui.add_space(4.0);

                let modes = [
                    (TextFitMode::None, "Do not Autofit", "Text may overflow the frame boundaries."),
                    (TextFitMode::ShrinkOnOverflow, "Shrink text on overflow", "Reduce font size only if text doesn't fit."),
                    (TextFitMode::GrowToFit, "Grow text to fill", "Increase font size to fill the frame."),
                    (TextFitMode::BestFit, "Best fit", "Find optimal size that fills without overflow."),
                ];

                for (mode, label, description) in modes {
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        if ui.radio(self.fit_mode == mode, "").clicked() {
                            self.fit_mode = mode;
                        }
                        ui.vertical(|ui| {
                            ui.add(egui::Label::new(
                                egui::RichText::new(label)
                                    .font(fonts::body())
                                    .color(colors::text_primary()),
                            ));
                            ui.add(egui::Label::new(
                                egui::RichText::new(description)
                                    .font(fonts::caption())
                                    .color(colors::text_secondary()),
                            ));
                        });
                    });
                    ui.add_space(4.0);
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
                                self.apply_changes(state, obj_id);
                            }
                            self.open = false;
                            result = Some(true);
                        }
                    });
                });
            });

        result
    }

    fn apply_changes(&self, state: &mut AppState, obj_id: ObjectId) {
        state.record_undo("Text Fit");
        if let Some(page) = state.current_page_mut() {
            if let Some(Shape { kind: ShapeKind::TextFrame(tf), .. }) = page.get_shape_mut(obj_id) {
                tf.text_fit = self.fit_mode;
            }
        }
        state.mark_dirty();
    }
}
