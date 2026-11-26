//! Text Wrap dialog for configuring how text flows around objects.

use egui::{Color32, DragValue, Vec2};
use repub_model::ObjectId;
use repub_model::styles::{TextWrap, TextWrapSide, TextWrapStyle};

use crate::state::AppState;
use crate::theme::{colors, fonts, section_header};

/// Text Wrap dialog state.
#[derive(Debug, Clone)]
pub struct TextWrapDialog {
    pub open: bool,
    pub object_id: Option<ObjectId>,
    pub wrap_style: TextWrapStyle,
    pub wrap_side: TextWrapSide,
    pub margin_top: f64,
    pub margin_bottom: f64,
    pub margin_left: f64,
    pub margin_right: f64,
}

impl Default for TextWrapDialog {
    fn default() -> Self {
        Self {
            open: false,
            object_id: None,
            wrap_style: TextWrapStyle::None,
            wrap_side: TextWrapSide::Both,
            margin_top: 4.0,
            margin_bottom: 4.0,
            margin_left: 4.0,
            margin_right: 4.0,
        }
    }
}

impl TextWrapDialog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open_with_object(&mut self, state: &AppState, obj_id: ObjectId) {
        if let Some(obj) = state.current_page().and_then(|p| p.get_shape(obj_id)) {
            let wrap = &obj.common.text_wrap;
            self.object_id = Some(obj_id);
            self.wrap_style = wrap.style;
            self.wrap_side = wrap.side;
            self.margin_top = wrap.margin_top;
            self.margin_bottom = wrap.margin_bottom;
            self.margin_left = wrap.margin_left;
            self.margin_right = wrap.margin_right;
            self.open = true;
        }
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) -> Option<bool> {
        if !self.open {
            return None;
        }

        let mut result = None;

        egui::Window::new("Text Wrap")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(320.0);

                // Wrap Style section
                section_header(ui, "Wrapping Style");
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    for style in TextWrapStyle::ALL {
                        let selected = self.wrap_style == style;
                        let response = self.wrap_style_button(ui, style, selected);
                        if response.clicked() {
                            self.wrap_style = style;
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.add_space(12.0);
                    ui.label(
                        egui::RichText::new(self.wrap_style.description())
                            .font(fonts::caption())
                            .color(colors::text_secondary()),
                    );
                });

                ui.add_space(8.0);

                // Wrap Side and Margins (only if not "None")
                if self.wrap_style != TextWrapStyle::None {
                    section_header(ui, "Wrap Text");
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.label("Side");
                        egui::ComboBox::from_id_salt("wrap_side")
                            .selected_text(self.wrap_side.name())
                            .show_ui(ui, |ui| {
                                for side in TextWrapSide::ALL {
                                    ui.selectable_value(&mut self.wrap_side, side, side.name());
                                }
                            });
                    });

                    ui.add_space(8.0);
                    section_header(ui, "Distance from Text");
                    for (label, value) in [
                        ("Top", &mut self.margin_top),
                        ("Bottom", &mut self.margin_bottom),
                        ("Left", &mut self.margin_left),
                        ("Right", &mut self.margin_right),
                    ] {
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.add_sized(Vec2::new(60.0, 20.0), egui::Label::new(label));
                            ui.add(DragValue::new(value).speed(0.5).suffix(" pt").range(0.0..=100.0));
                        });
                    }
                }

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

    fn wrap_style_button(&self, ui: &mut egui::Ui, style: TextWrapStyle, selected: bool) -> egui::Response {
        let size = Vec2::new(50.0, 50.0);
        let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let bg_color = if selected {
                colors::accent()
            } else if response.hovered() {
                colors::hover()
            } else {
                colors::bg_elevated()
            };
            painter.rect_filled(rect, 4.0, bg_color);

            let border_color = if selected { colors::accent_hover() } else { colors::border() };
            painter.rect_stroke(rect, 4.0, egui::Stroke::new(1.0, border_color));

            // Draw wrap style icon
            let icon_rect = rect.shrink(8.0);
            let text_color = if selected { Color32::WHITE } else { colors::text_primary() };
            self.draw_wrap_style_icon(painter, icon_rect, style, text_color);
        }

        response.on_hover_text(style.name())
    }

    fn draw_wrap_style_icon(&self, painter: &egui::Painter, rect: egui::Rect, style: TextWrapStyle, color: Color32) {
        let center = rect.center();
        let obj_rect = egui::Rect::from_center_size(center, Vec2::splat(12.0));
        painter.rect_filled(obj_rect, 2.0, color);

        let line_color = color.gamma_multiply(0.5);
        let line_height = 2.0;
        let line_spacing = 4.0;

        match style {
            TextWrapStyle::None => {
                for i in 0..4 {
                    let y = rect.top() + 4.0 + (i as f32) * line_spacing;
                    painter.hline(rect.x_range(), y, egui::Stroke::new(line_height, line_color));
                }
            }
            TextWrapStyle::Square | TextWrapStyle::Tight => {
                for i in 0..4 {
                    let y = rect.top() + 4.0 + (i as f32) * line_spacing;
                    if y > obj_rect.top() - 2.0 && y < obj_rect.bottom() + 2.0 {
                        painter.hline(rect.left()..=obj_rect.left() - 2.0, y, egui::Stroke::new(line_height, line_color));
                        painter.hline(obj_rect.right() + 2.0..=rect.right(), y, egui::Stroke::new(line_height, line_color));
                    } else {
                        painter.hline(rect.x_range(), y, egui::Stroke::new(line_height, line_color));
                    }
                }
            }
            TextWrapStyle::Through => {
                for i in 0..4 {
                    let y = rect.top() + 4.0 + (i as f32) * line_spacing;
                    painter.hline(rect.x_range(), y, egui::Stroke::new(line_height, line_color));
                }
            }
            TextWrapStyle::TopAndBottom => {
                painter.hline(rect.x_range(), rect.top() + 2.0, egui::Stroke::new(line_height, line_color));
                painter.hline(rect.x_range(), rect.top() + 6.0, egui::Stroke::new(line_height, line_color));
                painter.hline(rect.x_range(), rect.bottom() - 4.0, egui::Stroke::new(line_height, line_color));
            }
        }
    }

    fn apply_changes(&self, state: &mut AppState, obj_id: ObjectId) {
        state.record_undo("Text Wrap");
        if let Some(page) = state.current_page_mut() {
            if let Some(obj) = page.get_shape_mut(obj_id) {
                let wrap = &mut obj.common.text_wrap;
                wrap.style = self.wrap_style;
                wrap.side = self.wrap_side;
                wrap.margin_top = self.margin_top;
                wrap.margin_bottom = self.margin_bottom;
                wrap.margin_left = self.margin_left;
                wrap.margin_right = self.margin_right;
            }
        }
        state.mark_dirty();
    }

    /// Set wrap style directly from toolbar dropdown.
    pub fn set_wrap_style(state: &mut AppState, style: TextWrapStyle) {
        if let Some(&obj_id) = state.selection.first() {
            state.record_undo("Text Wrap");
            if let Some(page) = state.current_page_mut() {
                if let Some(obj) = page.get_shape_mut(obj_id) {
                    let wrap = &mut obj.common.text_wrap;
                    wrap.style = style;
                    if style != TextWrapStyle::None && wrap.margin_top == 0.0 {
                        wrap.margin_top = 4.0;
                        wrap.margin_bottom = 4.0;
                        wrap.margin_left = 4.0;
                        wrap.margin_right = 4.0;
                    }
                }
            }
            state.mark_dirty();
        }
    }
}
