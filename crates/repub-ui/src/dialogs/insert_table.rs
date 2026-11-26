//! Insert Table dialog for specifying table dimensions.

use egui::DragValue;

use crate::theme::colors;

/// Insert Table dialog state.
#[derive(Debug, Clone)]
pub struct InsertTableDialog {
    /// Whether the dialog is open
    pub open: bool,
    /// Number of rows
    pub rows: u32,
    /// Number of columns
    pub cols: u32,
}

impl Default for InsertTableDialog {
    fn default() -> Self {
        Self {
            open: false,
            rows: 3,
            cols: 3,
        }
    }
}

impl InsertTableDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog with default values.
    pub fn open(&mut self) {
        self.rows = 3;
        self.cols = 3;
        self.open = true;
    }

    /// Show the dialog.
    /// Returns `Some((rows, cols))` if OK was clicked, `None` otherwise.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<(u32, u32)> {
        if !self.open {
            return None;
        }

        let mut result = None;

        egui::Window::new("Insert Table")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(250.0);

                ui.add_space(8.0);

                // Table size section
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label("Table size:");
                });

                ui.add_space(8.0);

                // Number of columns
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label("Columns:");
                    ui.add(
                        DragValue::new(&mut self.cols)
                            .speed(0.1)
                            .range(1..=50),
                    );
                });

                ui.add_space(4.0);

                // Number of rows
                ui.horizontal(|ui| {
                    ui.add_space(24.0);
                    ui.label("Rows:    ");
                    ui.add(
                        DragValue::new(&mut self.rows)
                            .speed(0.1)
                            .range(1..=100),
                    );
                });

                ui.add_space(8.0);

                // Visual grid preview
                self.draw_preview(ui);

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                // Buttons
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.open = false;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(egui::Button::new("OK").fill(colors::accent()))
                            .clicked()
                        {
                            result = Some((self.rows, self.cols));
                            self.open = false;
                        }
                    });
                });

                // Handle Enter key to submit
                if ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    result = Some((self.rows, self.cols));
                    self.open = false;
                }

                // Handle Escape key to cancel
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.open = false;
                }
            });

        result
    }

    /// Draw a small preview grid showing the table dimensions.
    fn draw_preview(&self, ui: &mut egui::Ui) {
        let preview_size = egui::Vec2::new(200.0, 100.0);
        let (response, painter) = ui.allocate_painter(preview_size, egui::Sense::hover());
        let rect = response.rect;

        // Background
        painter.rect_filled(rect, egui::Rounding::same(2.0), colors::bg_elevated());

        // Calculate cell dimensions
        let padding = 8.0;
        let grid_rect = rect.shrink(padding);
        let cell_width = grid_rect.width() / self.cols as f32;
        let cell_height = grid_rect.height() / self.rows as f32;

        // Draw cells
        let cell_stroke = egui::Stroke::new(1.0, colors::border());
        for row in 0..self.rows {
            for col in 0..self.cols {
                let cell_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(
                        grid_rect.min.x + col as f32 * cell_width,
                        grid_rect.min.y + row as f32 * cell_height,
                    ),
                    egui::Vec2::new(cell_width, cell_height),
                );
                painter.rect_stroke(cell_rect, egui::Rounding::ZERO, cell_stroke);
            }
        }

        // Label showing dimensions
        let label = format!("{}×{}", self.cols, self.rows);
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            label,
            egui::FontId::proportional(14.0),
            colors::text_secondary(),
        );
    }
}
