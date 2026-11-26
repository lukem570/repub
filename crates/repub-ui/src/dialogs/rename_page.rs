//! Rename Page dialog.

use crate::state::AppState;
use crate::theme::{colors, fonts};

/// Dialog for renaming the current page.
#[derive(Debug, Clone, Default)]
pub struct RenamePageDialog {
    /// Whether the dialog is open
    pub open: bool,
    /// Index of the page being renamed
    pub page_index: usize,
    /// Working name
    pub name: String,
}

impl RenamePageDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog for a page.
    pub fn open_for_page(&mut self, state: &AppState, page_index: usize) {
        self.page_index = page_index;
        self.name = state
            .document
            .as_ref()
            .and_then(|d| d.page(page_index))
            .and_then(|p| p.name.clone())
            .unwrap_or_default();
        self.open = true;
    }

    /// Show the dialog. Returns Some(true) on OK, Some(false) on Cancel.
    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) -> Option<bool> {
        if !self.open {
            return None;
        }
        let mut result = None;

        egui::Window::new("Rename Page")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(280.0);
                ui.label(egui::RichText::new("Page name").font(fonts::body()));
                let resp = ui.add(egui::TextEdit::singleline(&mut self.name).desired_width(f32::INFINITY));

                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.open = false;
                        result = Some(false);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("OK").fill(colors::accent())).clicked() {
                            let name = self.name.trim().to_string();
                            state.record_undo("Rename Page");
                            if let Some(doc) = state.document.as_mut() {
                                doc.rename_page(self.page_index, if name.is_empty() { None } else { Some(name) });
                            }
                            state.mark_dirty();
                            self.open = false;
                            result = Some(true);
                        }
                    });
                });

                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    let name = self.name.trim().to_string();
                    state.record_undo("Rename Page");
                    if let Some(doc) = state.document.as_mut() {
                        doc.rename_page(self.page_index, if name.is_empty() { None } else { Some(name) });
                    }
                    state.mark_dirty();
                    self.open = false;
                    result = Some(true);
                }
            });

        result
    }
}
