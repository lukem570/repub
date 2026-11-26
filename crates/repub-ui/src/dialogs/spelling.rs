//! Spelling dialog.

use egui::TextEdit;
use repub_model::ObjectId;

use crate::spelling::SpellSession;
use crate::state::AppState;
use crate::theme::{colors, fonts, section_header};

/// Spelling check dialog state.
#[derive(Debug, Default)]
pub struct SpellingDialog {
    /// Whether the dialog is open
    pub open: bool,
    /// Scan results and session ignores
    pub session: SpellSession,
    /// Text in the "Change to" field
    pub change_to: String,
    /// Object the checker last navigated to
    pub object_id: Option<ObjectId>,
    /// A replacement was applied during the last frame
    pub changed: bool,
}

impl SpellingDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog and scan the document.
    pub fn open(&mut self, state: &AppState) {
        self.session.scan(state);
        self.session.current = 0;
        self.change_to.clear();
        self.changed = false;
        self.open = true;
        if let Some(err) = self.session.current_error() {
            self.object_id = Some(err.object);
        }
    }

    /// Whether the checker has finished (no errors left).
    pub fn is_complete(&self) -> bool {
        self.session.errors.is_empty()
    }

    /// Show the dialog. Returns true when a correction was applied.
    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) -> bool {
        if !self.open {
            return false;
        }
        let mut applied = false;
        let mut close = false;
        let now = ctx.input(|i| i.time);

        egui::Window::new("Spelling")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(400.0);
                section_header(ui, "Not in Dictionary");
                ui.add_space(4.0);

                let current = self.session.current_error().cloned();
                match current {
                    Some(err) => {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Not in dictionary:").font(fonts::body()));
                            ui.label(
                                egui::RichText::new(&err.word)
                                    .font(fonts::body())
                                    .strong()
                                    .color(colors::accent()),
                            );
                        });
                        ui.add_space(4.0);

                        section_header(ui, "Suggestions");
                        let suggestions = self.session.suggestions();
                        egui::ScrollArea::vertical()
                            .max_height(110.0)
                            .show(ui, |ui| {
                                if suggestions.is_empty() {
                                    ui.label(
                                        egui::RichText::new("No suggestions.")
                                            .font(fonts::caption())
                                            .color(colors::text_secondary()),
                                    );
                                }
                                for s in suggestions {
                                    if ui.selectable_label(false, s.clone()).clicked() {
                                        self.change_to = s;
                                    }
                                }
                            });

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Change to").font(fonts::body()));
                            ui.add(TextEdit::singleline(&mut self.change_to).desired_width(220.0));
                        });

                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui.button("Ignore Once").clicked() {
                                self.session.ignore_current();
                                if let Some(err) = self.session.current_error() {
                                    self.object_id = Some(err.object);
                                    self.session.select_current(state, now);
                                }
                                self.change_to.clear();
                            }
                            if ui.button("Ignore All").clicked() {
                                self.session.ignore_all_current();
                                self.change_to.clear();
                            }
                            if ui.button("Add to Dictionary").clicked() {
                                self.session.add_current();
                                self.change_to.clear();
                            }
                        });
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            let enabled = !self.change_to.trim().is_empty();
                            if ui
                                .add_enabled(enabled, egui::Button::new("Change"))
                                .clicked()
                            {
                                let replacement = self.change_to.trim().to_string();
                                if self.session.change_current(state, &replacement) {
                                    applied = true;
                                    self.changed = true;
                                }
                                self.change_to.clear();
                            }
                            if ui
                                .add_enabled(enabled, egui::Button::new("Change All"))
                                .clicked()
                            {
                                let replacement = self.change_to.trim().to_string();
                                if self.session.change_all_current(state, &replacement) {
                                    applied = true;
                                    self.changed = true;
                                }
                                self.change_to.clear();
                            }
                        });
                    }
                    None => {
                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new("Spelling check complete.")
                                .font(fonts::body())
                                .color(colors::success()),
                        );
                        ui.add_space(8.0);
                    }
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(egui::Button::new("Close").fill(colors::accent()))
                        .clicked()
                    {
                        close = true;
                    }
                    if !self.session.errors.is_empty() {
                        ui.label(
                            egui::RichText::new(format!("{} remaining", self.session.errors.len()))
                                .font(fonts::caption())
                                .color(colors::text_secondary()),
                        );
                    }
                });
            });

        if close {
            self.open = false;
        }
        applied
    }
}
