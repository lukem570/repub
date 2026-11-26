//! About dialog.

use crate::theme::colors;

/// About dialog state.
#[derive(Debug, Default)]
pub struct AboutDialog {
    pub open: bool,
}

impl AboutDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        egui::Window::new("About Repub")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(300.0);

                ui.vertical_centered(|ui| {
                    ui.heading("Repub");
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.add_space(8.0);
                    ui.label("A desktop publishing application");
                    ui.label("inspired by Microsoft Publisher");
                    ui.add_space(16.0);
                    ui.label(
                        egui::RichText::new("Built with Rust and egui")
                            .color(colors::text_secondary()),
                    );
                    ui.add_space(8.0);

                    ui.hyperlink_to("GitHub Repository", "https://github.com/lukem570/repub");

                    ui.add_space(16.0);

                    if ui.button("Close").clicked() {
                        self.open = false;
                    }
                });
            });
    }
}
