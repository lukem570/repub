//! Move Page dialog: reorders the current page.

use crate::state::AppState;
use crate::theme::{colors, fonts};

/// Dialog for moving the current page to a new position.
#[derive(Debug, Clone, Default)]
pub struct MovePageDialog {
    /// Whether the dialog is open
    pub open: bool,
    /// Index of the page being moved
    pub from: usize,
    /// Target position (0-based)
    pub to: usize,
    /// Total page count when opened
    pub page_count: usize,
}

impl MovePageDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog for a page.
    pub fn open_for_page(&mut self, state: &AppState, page_index: usize) {
        self.from = page_index;
        self.to = page_index;
        self.page_count = state.page_count().max(1);
        self.open = true;
    }

    /// Show the dialog. Returns Some(true) on OK, Some(false) on Cancel.
    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) -> Option<bool> {
        if !self.open {
            return None;
        }
        let mut result = None;

        egui::Window::new("Move Page")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(300.0);
                ui.label(
                    egui::RichText::new(format!("Move page {} to position:", self.from + 1))
                        .font(fonts::body()),
                );
                ui.add_space(6.0);
                let mut one_based = self.to + 1;
                ui.add(
                    egui::DragValue::new(&mut one_based)
                        .speed(1.0)
                        .range(1..=self.page_count.max(1)),
                );
                self.to = one_based.saturating_sub(1).min(self.page_count.saturating_sub(1));

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.open = false;
                        result = Some(false);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("OK").fill(colors::accent())).clicked() {
                            state.record_undo("Move Page");
                            if let Some(doc) = state.document.as_mut() {
                                doc.move_page(self.from, self.to);
                            }
                            state.current_page = self.to;
                            state.mark_dirty();
                            self.open = false;
                            result = Some(true);
                        }
                    });
                });
            });

        result
    }
}
