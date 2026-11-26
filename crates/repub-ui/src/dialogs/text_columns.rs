//! Text Columns dialog for setting text frame column count and gap.

use egui::{DragValue, Vec2};
use repub_model::shape::{Shape, ShapeKind};
use repub_model::ObjectId;

use crate::state::AppState;
use crate::theme::{colors, fonts, section_header};

/// Text Columns dialog state.
#[derive(Debug, Clone)]
pub struct TextColumnsDialog {
    /// Whether the dialog is open
    pub open: bool,
    /// Object being edited
    pub object_id: Option<ObjectId>,
    /// Number of columns
    pub column_count: u32,
    /// Gap between columns in points
    pub column_gap: f64,
    /// Original values for cancel
    original_column_count: u32,
    original_column_gap: f64,
}

impl Default for TextColumnsDialog {
    fn default() -> Self {
        Self {
            open: false,
            object_id: None,
            column_count: 1,
            column_gap: 12.0,
            original_column_count: 1,
            original_column_gap: 12.0,
        }
    }
}

impl TextColumnsDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog for a text frame.
    pub fn open_for_text_frame(&mut self, state: &AppState, obj_id: ObjectId) {
        if let Some(Shape {
            kind: ShapeKind::TextFrame(tf),
            ..
        }) = Self::get_object(state, obj_id)
        {
            self.object_id = Some(obj_id);
            self.column_count = tf.columns.count;
            self.column_gap = tf.columns.gap;

            // Store originals for cancel
            self.original_column_count = self.column_count;
            self.original_column_gap = self.column_gap;

            self.open = true;
        }
    }

    fn get_object(state: &AppState, obj_id: ObjectId) -> Option<Shape> {
        state.current_page()?.get_shape(obj_id).cloned()
    }

    /// Show the dialog.
    /// Returns Some(true) if OK was clicked, Some(false) if Cancel was clicked, None if still open.
    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) -> Option<bool> {
        if !self.open {
            return None;
        }

        let mut result = None;

        egui::Window::new("Text Columns")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(280.0);

                // Columns section
                section_header(ui, "Columns");

                // Number of columns
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.add_sized(
                        Vec2::new(80.0, 20.0),
                        egui::Label::new(
                            egui::RichText::new("Number")
                                .font(fonts::caption())
                                .color(colors::text_secondary()),
                        ),
                    );
                    ui.add(
                        DragValue::new(&mut self.column_count)
                            .speed(0.1)
                            .range(1..=10),
                    );
                });

                // Gap between columns
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.add_sized(
                        Vec2::new(80.0, 20.0),
                        egui::Label::new(
                            egui::RichText::new("Gap")
                                .font(fonts::caption())
                                .color(colors::text_secondary()),
                        ),
                    );
                    ui.add(
                        DragValue::new(&mut self.column_gap)
                            .speed(0.5)
                            .suffix(" pt")
                            .min_decimals(1)
                            .max_decimals(1)
                            .range(0.0..=200.0),
                    );
                });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.open = false;
                        result = Some(false);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new("OK").fill(colors::accent()))
                            .clicked()
                        {
                            // Apply changes
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
        state.record_undo("Text Columns");
        if let Some(page) = state.current_page_mut() {
            if let Some(Shape {
                kind: ShapeKind::TextFrame(tf),
                ..
            }) = page.get_shape_mut(obj_id)
            {
                tf.columns.count = self.column_count;
                tf.columns.gap = self.column_gap;
            }
        }
        state.mark_dirty();
    }
}
