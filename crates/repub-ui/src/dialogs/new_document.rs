//! New document dialog.

use repub_model::{PageOrientation, PageSize};

use crate::theme::colors;

/// New document dialog state.
#[derive(Debug)]
pub struct NewDocumentDialog {
    pub open: bool,
    pub size_preset: String,
    pub custom_width: f64,
    pub custom_height: f64,
    pub orientation: PageOrientation,
    pub pages: u32,
}

impl Default for NewDocumentDialog {
    fn default() -> Self {
        Self {
            open: false,
            size_preset: "letter".to_string(),
            custom_width: 612.0,
            custom_height: 792.0,
            orientation: PageOrientation::Portrait,
            pages: 1,
        }
    }
}

impl NewDocumentDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn show(&mut self, ctx: &egui::Context) -> Option<NewDocumentResult> {
        let mut result = None;

        if !self.open {
            return None;
        }

        let (enter, escape) = ctx.input(|i| {
            (
                i.key_pressed(egui::Key::Enter),
                i.key_pressed(egui::Key::Escape),
            )
        });
        if escape {
            self.open = false;
            return None;
        }
        let mut create = enter;

        egui::Window::new("New Document")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(colors::bg_window())
                    .stroke(egui::Stroke::new(1.0, colors::border_strong()))
                    .rounding(egui::Rounding::ZERO)
                    .inner_margin(egui::Margin::symmetric(20.0, 14.0)),
            )
            .show(ctx, |ui| {
                ui.set_width(360.0);

                // Title row: left-aligned title, close box on the right
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("New Publication")
                            .size(15.0)
                            .color(colors::text_primary()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (rect, resp) =
                            ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
                        if resp.hovered() {
                            ui.painter().rect_filled(rect, 0.0, colors::hover());
                        }
                        let c = rect.center();
                        let stroke = egui::Stroke::new(1.0, colors::text_secondary());
                        ui.painter().line_segment(
                            [c + egui::vec2(-5.0, -5.0), c + egui::vec2(5.0, 5.0)],
                            stroke,
                        );
                        ui.painter().line_segment(
                            [c + egui::vec2(5.0, -5.0), c + egui::vec2(-5.0, 5.0)],
                            stroke,
                        );
                        if resp.clicked() {
                            self.open = false;
                        }
                    });
                });
                ui.add_space(12.0);

                egui::Grid::new("new_doc_grid")
                    .num_columns(2)
                    .spacing(egui::vec2(16.0, 10.0))
                    .show(ui, |ui| {
                        ui.label("Page size:");
                        egui::ComboBox::from_id_salt("page_size")
                            .width(200.0)
                            .selected_text(self.size_name())
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.size_preset,
                                    "letter".to_string(),
                                    "Letter (8.5 × 11 in)",
                                );
                                ui.selectable_value(
                                    &mut self.size_preset,
                                    "legal".to_string(),
                                    "Legal (8.5 × 14 in)",
                                );
                                ui.selectable_value(
                                    &mut self.size_preset,
                                    "tabloid".to_string(),
                                    "Tabloid (11 × 17 in)",
                                );
                                ui.selectable_value(
                                    &mut self.size_preset,
                                    "a5".to_string(),
                                    "A5 (148 × 210 mm)",
                                );
                                ui.selectable_value(
                                    &mut self.size_preset,
                                    "a4".to_string(),
                                    "A4 (210 × 297 mm)",
                                );
                                ui.selectable_value(
                                    &mut self.size_preset,
                                    "a3".to_string(),
                                    "A3 (297 × 420 mm)",
                                );
                                ui.selectable_value(
                                    &mut self.size_preset,
                                    "custom".to_string(),
                                    "Custom...",
                                );
                            });
                        ui.end_row();

                        if self.size_preset == "custom" {
                            ui.label("Width:");
                            ui.add(
                                egui::DragValue::new(&mut self.custom_width)
                                    .speed(1.0)
                                    .suffix(" pt"),
                            );
                            ui.end_row();
                            ui.label("Height:");
                            ui.add(
                                egui::DragValue::new(&mut self.custom_height)
                                    .speed(1.0)
                                    .suffix(" pt"),
                            );
                            ui.end_row();
                        }

                        ui.label("Orientation:");
                        ui.horizontal(|ui| {
                            ui.radio_value(
                                &mut self.orientation,
                                PageOrientation::Portrait,
                                "Portrait",
                            );
                            ui.radio_value(
                                &mut self.orientation,
                                PageOrientation::Landscape,
                                "Landscape",
                            );
                        });
                        ui.end_row();

                        ui.label("Pages:");
                        ui.add(
                            egui::DragValue::new(&mut self.pages)
                                .speed(1.0)
                                .range(1..=100),
                        );
                        ui.end_row();
                    });

                ui.add_space(18.0);

                // Office order: primary button first, both right-aligned
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let size = egui::vec2(80.0, 26.0);
                    if ui
                        .add(
                            egui::Button::new("Cancel")
                                .min_size(size)
                                .fill(colors::bg_window())
                                .stroke(egui::Stroke::new(1.0, colors::border_strong())),
                        )
                        .clicked()
                    {
                        self.open = false;
                    }
                    let primary = egui::Button::new(
                        egui::RichText::new("Create").color(colors::text_inverted()),
                    )
                    .min_size(size)
                    .fill(colors::accent())
                    .stroke(egui::Stroke::new(1.0, colors::accent()));
                    if ui.add(primary).clicked() {
                        create = true;
                    }
                });
            });

        if create && self.open {
            result = Some(NewDocumentResult {
                page_size: self.page_size(),
                orientation: self.orientation,
                pages: self.pages,
            });
            self.open = false;
        }

        result
    }

    fn size_name(&self) -> &str {
        match self.size_preset.as_str() {
            "letter" => "Letter",
            "legal" => "Legal",
            "tabloid" => "Tabloid",
            "a3" => "A3",
            "a4" => "A4",
            "a5" => "A5",
            "custom" => "Custom",
            _ => "Letter",
        }
    }

    fn page_size(&self) -> PageSize {
        match self.size_preset.as_str() {
            "letter" => PageSize::Letter,
            "legal" => PageSize::Legal,
            "tabloid" => PageSize::Tabloid,
            "a3" => PageSize::A3,
            "a4" => PageSize::A4,
            "a5" => PageSize::A5,
            "custom" => PageSize::Custom {
                width: self.custom_width,
                height: self.custom_height,
            },
            _ => PageSize::Letter,
        }
    }
}

/// Result from the new document dialog.
pub struct NewDocumentResult {
    pub page_size: PageSize,
    pub orientation: PageOrientation,
    pub pages: u32,
}
