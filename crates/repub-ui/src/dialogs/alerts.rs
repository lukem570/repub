//! Small modal alerts: the "save changes?" prompt and error messages.

use crate::theme::{colors, fonts};

/// The user's answer to the save-changes prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveChoice {
    Save,
    DontSave,
    Cancel,
}

/// "Do you want to save changes to …?" shown before unsaved work would be lost.
#[derive(Debug, Clone, Default)]
pub struct SaveChangesPrompt {
    /// Document name shown in the question; `None` when closed.
    document: Option<String>,
}

impl SaveChangesPrompt {
    pub fn open(&mut self, document_name: impl Into<String>) {
        self.document = Some(document_name.into());
    }

    pub fn is_open(&self) -> bool {
        self.document.is_some()
    }

    pub fn close(&mut self) {
        self.document = None;
    }

    /// Show the prompt; returns the answer on the frame it is given.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<SaveChoice> {
        let name = self.document.clone()?;
        let mut choice = None;
        egui::Window::new("Repub")
            .id(egui::Id::new("save_changes_prompt"))
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(340.0);
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new(format!("Do you want to save changes to \"{name}\"?"))
                        .font(fonts::body())
                        .strong(),
                );
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Your changes will be lost if you don't save them.")
                        .font(fonts::body())
                        .color(colors::text_secondary()),
                );
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if ui.button("Don't Save").clicked() {
                        choice = Some(SaveChoice::DontSave);
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Save").color(egui::Color32::WHITE),
                                )
                                .fill(colors::accent()),
                            )
                            .clicked()
                        {
                            choice = Some(SaveChoice::Save);
                        }
                        if ui.button("Cancel").clicked() {
                            choice = Some(SaveChoice::Cancel);
                        }
                    });
                });
                if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                    choice = Some(SaveChoice::Cancel);
                }
            });
        if choice.is_some() {
            self.document = None;
        }
        choice
    }
}

/// A modal error message with an OK button.
#[derive(Debug, Clone, Default)]
pub struct ErrorAlert {
    message: Option<(String, String)>,
}

impl ErrorAlert {
    /// Show `title` (what failed) and `detail` (why).
    pub fn open(&mut self, title: impl Into<String>, detail: impl Into<String>) {
        self.message = Some((title.into(), detail.into()));
    }

    pub fn message(&self) -> Option<&(String, String)> {
        self.message.as_ref()
    }

    pub fn is_open(&self) -> bool {
        self.message.is_some()
    }

    /// Title of the open alert, if any (for tests).
    pub fn title(&self) -> Option<&str> {
        self.message.as_ref().map(|(t, _)| t.as_str())
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        let Some((title, detail)) = self.message.clone() else {
            return;
        };
        let mut close = false;
        egui::Window::new(title.as_str())
            .id(egui::Id::new("error_alert"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(360.0);
                ui.label(egui::RichText::new(detail).font(fonts::body()));
                ui.add_space(12.0);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                egui::RichText::new("OK").color(egui::Color32::WHITE),
                            )
                            .fill(colors::accent()),
                        )
                        .clicked()
                    {
                        close = true;
                    }
                });
                if ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape))
                {
                    close = true;
                }
            });
        if close {
            self.message = None;
        }
    }
}
