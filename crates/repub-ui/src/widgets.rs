//! Custom widgets.

use egui::{Color32, Response, Sense, Stroke, Ui, Vec2, Widget};

use crate::icons::Icon;
use crate::theme::colors;

/// Color picker button widget.
pub struct ColorButton {
    color: Color32,
    size: Vec2,
}

impl ColorButton {
    /// Create a new color button.
    pub fn new(color: Color32) -> Self {
        Self {
            color,
            size: Vec2::new(24.0, 24.0),
        }
    }

    /// Set the button size.
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }
}

impl Widget for ColorButton {
    fn ui(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(self.size, Sense::click());

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);
            ui.painter().rect_filled(rect, 4.0, self.color);
            ui.painter()
                .rect_stroke(rect, 4.0, Stroke::new(1.0, visuals.fg_stroke.color));
        }

        response
    }
}

/// Actions that can be triggered from the status bar.
#[derive(Debug, Clone)]
pub enum StatusBarAction {
    /// Set zoom to a specific level (as a fraction, e.g., 1.0 = 100%).
    /// A negative value means "fit the whole page".
    SetZoom(f32),
    /// Show or hide the Page Navigation pane.
    TogglePagesPane,
    /// Switch between single-page and two-page spread view.
    SetTwoPageSpread(bool),
}

/// Publisher-style status bar.
///
/// Left: page indicator (toggles the Page Navigation pane), object position
/// and size (in inches). Right: Single Page / Two-Page Spread, zoom − slider +,
/// zoom %, fit page.
pub struct StatusBar;

/// Status bar information.
pub struct StatusBarInfo {
    /// Pages on view, 0-based: one page, or the two pages of a spread.
    pub pages_on_view: Vec<usize>,
    pub total_pages: usize,
    pub zoom: f32,
    pub two_page_spread: bool,
    /// Top-left of the selection's bounds, else the pointer over the canvas (document points).
    pub position: Option<(f64, f64)>,
    /// Size of the current selection's bounds, points.
    pub selection_size: Option<(f64, f64)>,
    pub units: crate::state::DisplayUnit,
}

impl StatusBar {
    /// Zoom presets for the zoom menu
    const ZOOM_PRESETS: &'static [(f32, &'static str)] = &[
        (4.00, "400%"),
        (2.00, "200%"),
        (1.50, "150%"),
        (1.00, "100%"),
        (0.75, "75%"),
        (0.50, "50%"),
        (0.33, "33%"),
        (0.25, "25%"),
    ];
    const MIN_ZOOM: f32 = 0.1;
    const MAX_ZOOM: f32 = 8.0;

    fn text(s: impl Into<String>) -> egui::RichText {
        egui::RichText::new(s)
            .font(crate::theme::fonts::body_sm())
            .color(colors::text_secondary())
    }

    /// Icon then text, e.g. the pointer position.
    fn icon_label(ui: &mut Ui, icon: Icon, text: &str) {
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
        icon.paint(ui, rect, colors::text_secondary());
        ui.label(Self::text(text));
    }

    /// Flat icon button used on the right side.
    fn icon_button(ui: &mut Ui, icon: Icon, selected: bool, enabled: bool, tip: &str) -> Response {
        let sense = if enabled {
            Sense::click()
        } else {
            Sense::hover()
        };
        let (rect, response) = ui.allocate_exact_size(Vec2::new(22.0, 18.0), sense);
        if selected {
            ui.painter().rect_filled(rect, 2.0, colors::pressed());
        } else if enabled && response.hovered() {
            ui.painter().rect_filled(rect, 2.0, colors::hover());
        }
        icon.paint_state(
            ui,
            egui::Rect::from_center_size(rect.center(), Vec2::splat(14.0)),
            enabled,
            colors::text_primary(),
            colors::text_disabled(),
        );
        response.on_hover_text(tip)
    }

    /// Show the status bar.
    pub fn show_full(ui: &mut Ui, info: StatusBarInfo) -> Option<StatusBarAction> {
        let mut action: Option<StatusBarAction> = None;
        let has_doc = info.total_pages > 0;
        // Narrow windows drop the least important items first (as Publisher does)
        let width = ui.max_rect().width();
        let (show_size, show_position, show_slider) =
            (width >= 760.0, width >= 600.0, width >= 480.0);

        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;

            // Page indicator: shows/hides the Page Navigation pane, as in Publisher
            if has_doc {
                let label = match info.pages_on_view.as_slice() {
                    [a, b] => format!("Pages: {}–{} of {}", a + 1, b + 1, info.total_pages),
                    [a, ..] => format!("Page: {} of {}", a + 1, info.total_pages),
                    [] => String::new(),
                };
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
                Icon::DocumentOnePage.paint(ui, rect, colors::text_secondary());
                if ui
                    .add(egui::Button::new(Self::text(label)).frame(false))
                    .on_hover_text("Show or hide the Page Navigation pane")
                    .clicked()
                {
                    action = Some(StatusBarAction::TogglePagesPane);
                }

                ui.add_space(14.0);
                let position = info
                    .position
                    .filter(|_| show_position)
                    .map(|(x, y)| format!("{}, {}", info.units.format(x), info.units.format(y)))
                    .unwrap_or_default();
                if show_position {
                    Self::icon_label(ui, Icon::Cursor, &position);
                }
                ui.add_space(14.0);
                if let Some((w, h)) = info.selection_size.filter(|_| show_size) {
                    Self::icon_label(
                        ui,
                        Icon::Resize,
                        &format!("{} x {}", info.units.format(w), info.units.format(h)),
                    );
                }
            }

            // Right side, laid out right to left
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.add_space(4.0);
                if Self::icon_button(ui, Icon::PageFit, false, has_doc, "Fit page to the window")
                    .clicked()
                {
                    action = Some(StatusBarAction::SetZoom(-1.0));
                }
                ui.add_space(4.0);

                // Zoom percentage, click for presets
                let zoom_response = ui
                    .add_enabled(
                        has_doc,
                        egui::Button::new(Self::text(format!("{:.0}%", info.zoom * 100.0)))
                            .frame(false)
                            .min_size(Vec2::new(40.0, 18.0)),
                    )
                    .on_hover_text("Zoom level");
                let zoom_popup_id = ui.make_persistent_id("zoom_menu_popup");
                if zoom_response.clicked() {
                    ui.memory_mut(|mem| mem.toggle_popup(zoom_popup_id));
                }
                egui::popup_above_or_below_widget(
                    ui,
                    zoom_popup_id,
                    &zoom_response,
                    egui::AboveOrBelow::Above,
                    egui::PopupCloseBehavior::CloseOnClick,
                    |ui| {
                        ui.set_min_width(120.0);
                        for &(zoom_level, label) in Self::ZOOM_PRESETS {
                            if ui
                                .selectable_label((info.zoom - zoom_level).abs() < 0.01, label)
                                .clicked()
                            {
                                action = Some(StatusBarAction::SetZoom(zoom_level));
                            }
                        }
                        ui.separator();
                        if ui.selectable_label(false, "Whole Page").clicked() {
                            action = Some(StatusBarAction::SetZoom(-1.0));
                        }
                    },
                );

                // − slider +
                if Self::icon_button(ui, Icon::Add, false, has_doc, "Zoom in").clicked() {
                    action = Some(StatusBarAction::SetZoom(
                        (info.zoom * 1.25).min(Self::MAX_ZOOM),
                    ));
                }
                if show_slider {
                    let mut zoom = info.zoom;
                    let slider = egui::Slider::new(&mut zoom, Self::MIN_ZOOM..=Self::MAX_ZOOM)
                        .logarithmic(true)
                        .show_value(false);
                    ui.spacing_mut().slider_width = 110.0;
                    if ui.add_enabled(has_doc, slider).changed() {
                        action = Some(StatusBarAction::SetZoom(zoom));
                    }
                }
                if Self::icon_button(ui, Icon::Subtract, false, has_doc, "Zoom out").clicked() {
                    action = Some(StatusBarAction::SetZoom(
                        (info.zoom / 1.25).max(Self::MIN_ZOOM),
                    ));
                }

                ui.add_space(10.0);
                if Self::icon_button(
                    ui,
                    Icon::BookOpen,
                    info.two_page_spread,
                    has_doc,
                    "Two-Page Spread",
                )
                .clicked()
                {
                    action = Some(StatusBarAction::SetTwoPageSpread(true));
                }
                if Self::icon_button(
                    ui,
                    Icon::DocumentOnePage,
                    !info.two_page_spread,
                    has_doc,
                    "Single Page",
                )
                .clicked()
                {
                    action = Some(StatusBarAction::SetTwoPageSpread(false));
                }
            });
        });

        action
    }
}

/// Ruler orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulerOrientation {
    Horizontal,
    Vertical,
}

/// Result of ruler interaction.
#[derive(Debug, Clone, Copy)]
pub enum RulerInteraction {
    /// User is dragging to create a new guide at this document position.
    CreateGuide(f64),
    /// User released the drag - create guide at this position.
    FinishGuide(f64),
}

/// Ruler widget for displaying measurements and creating guides.
pub struct Ruler {
    orientation: RulerOrientation,
    /// Page origin in screen coordinates (where 0,0 of the page is on screen).
    page_origin: f32,
    /// Page size in screen coordinates.
    page_size: f32,
    /// Current zoom level.
    zoom: f32,
    /// Current mouse position in document coordinates (for position indicator).
    mouse_doc_pos: Option<f32>,
    /// Page origin on the perpendicular axis, used to place guides dragged off the ruler.
    cross_origin: f32,
}

impl Ruler {
    /// Size of the ruler in pixels.
    pub const SIZE: f32 = 20.0;

    /// Points per inch.
    const POINTS_PER_INCH: f64 = 72.0;

    /// Create a new ruler.
    pub fn new(orientation: RulerOrientation) -> Self {
        Self {
            orientation,
            page_origin: 0.0,
            page_size: 0.0,
            zoom: 1.0,
            mouse_doc_pos: None,
            cross_origin: 0.0,
        }
    }

    /// Set the page origin on the perpendicular axis, in screen coordinates.
    pub fn cross_origin(mut self, origin: f32) -> Self {
        self.cross_origin = origin;
        self
    }

    /// Set page origin in screen coordinates.
    pub fn page_origin(mut self, origin: f32) -> Self {
        self.page_origin = origin;
        self
    }

    /// Set page size in screen coordinates.
    pub fn page_size(mut self, size: f32) -> Self {
        self.page_size = size;
        self
    }

    /// Set the zoom level.
    pub fn zoom(mut self, zoom: f32) -> Self {
        self.zoom = zoom;
        self
    }

    /// Set the current mouse position in document coordinates.
    pub fn mouse_doc_pos(mut self, pos: Option<f32>) -> Self {
        self.mouse_doc_pos = pos;
        self
    }

    /// Show the ruler and return any interaction.
    pub fn show(self, ui: &mut Ui, available_length: f32) -> (Response, Option<RulerInteraction>) {
        let size = match self.orientation {
            RulerOrientation::Horizontal => Vec2::new(available_length, Self::SIZE),
            RulerOrientation::Vertical => Vec2::new(Self::SIZE, available_length),
        };

        let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());

        if ui.is_rect_visible(rect) {
            self.draw_ruler(ui, rect);
        }

        // Handle interaction for creating guides
        let interaction = self.handle_interaction(&response);

        (response, interaction)
    }

    fn draw_ruler(&self, ui: &Ui, rect: egui::Rect) {
        let painter = ui.painter_at(rect);
        let horizontal = self.orientation == RulerOrientation::Horizontal;

        // Background
        painter.rect_filled(rect, 0.0, colors::ruler_bg());

        // White span over the page's extent
        if self.page_size > 0.0 {
            let span = if horizontal {
                egui::Rect::from_x_y_ranges(
                    self.page_origin..=self.page_origin + self.page_size,
                    rect.y_range(),
                )
            } else {
                egui::Rect::from_x_y_ranges(
                    rect.x_range(),
                    self.page_origin..=self.page_origin + self.page_size,
                )
            };
            painter.rect_filled(span.intersect(rect), 0.0, colors::ruler_page_bg());
        }

        // Border along the edge facing the workspace
        let border_stroke = Stroke::new(1.0, colors::ruler_border());
        if horizontal {
            painter.hline(rect.x_range(), rect.bottom() - 0.5, border_stroke);
        } else {
            painter.vline(rect.right() - 0.5, rect.y_range(), border_stroke);
        }

        // Ticks every 1/8 inch, graduated: inch longest, then 1/2, 1/4, 1/8.
        // Finer levels are dropped when they would be closer than 4 px apart.
        let inch_px = Self::POINTS_PER_INCH as f32 * self.zoom;
        if inch_px <= 0.0 {
            return;
        }
        let subdivisions: i64 = if inch_px / 8.0 >= 4.0 {
            8
        } else if inch_px / 4.0 >= 4.0 {
            4
        } else if inch_px / 2.0 >= 4.0 {
            2
        } else {
            1
        };
        // Label every inch, or every 2/5/10... inches when zoomed far out
        let label_step: i64 = [1, 2, 5, 10, 20, 50, 100]
            .into_iter()
            .find(|n| inch_px * *n as f32 >= 24.0)
            .unwrap_or(100);

        let (ruler_start, ruler_end) = if horizontal {
            (rect.left(), rect.right())
        } else {
            (rect.top(), rect.bottom())
        };
        let step_px = inch_px / subdivisions as f32;
        let first = ((ruler_start - self.page_origin) / step_px).floor() as i64 - 1;
        let last = ((ruler_end - self.page_origin) / step_px).ceil() as i64 + 1;

        let tick_stroke = Stroke::new(1.0, colors::ruler_tick());
        let font = egui::FontId::proportional(9.0);
        for i in first..=last {
            // Snap to the pixel grid so 1px ticks stay crisp
            let screen_pos = (self.page_origin + i as f32 * step_px).round() + 0.5;
            let eighths = i.rem_euclid(subdivisions) * (8 / subdivisions);
            let tick_len = match eighths {
                0 => 8.0,
                4 => 6.0,
                2 | 6 => 4.0,
                _ => 2.5,
            };
            let inch = i.div_euclid(subdivisions);
            let label = eighths == 0 && inch % label_step == 0;

            if horizontal {
                painter.vline(
                    screen_pos,
                    egui::Rangef::new(rect.bottom() - tick_len, rect.bottom()),
                    tick_stroke,
                );
                if label {
                    // Centered on the inch tick, above it; negative positions show absolute values
                    painter.text(
                        egui::Pos2::new(screen_pos, rect.top() + 5.5),
                        egui::Align2::CENTER_CENTER,
                        inch.abs().to_string(),
                        font.clone(),
                        colors::ruler_text(),
                    );
                }
            } else {
                painter.hline(
                    egui::Rangef::new(rect.right() - tick_len, rect.right()),
                    screen_pos,
                    tick_stroke,
                );
                if label {
                    // Rotated 90° counter-clockwise (reads bottom to top), centered on the tick
                    let galley = painter.layout_no_wrap(
                        inch.abs().to_string(),
                        font.clone(),
                        colors::ruler_text(),
                    );
                    let size = galley.size();
                    let center_x = rect.left() + 5.5;
                    let pos = egui::Pos2::new(center_x - size.y / 2.0, screen_pos + size.x / 2.0);
                    painter.add(
                        egui::epaint::TextShape::new(pos, galley, colors::ruler_text())
                            .with_angle(-std::f32::consts::FRAC_PI_2),
                    );
                }
            }
        }

        // Mouse position indicator: thin dotted line across the ruler
        if let Some(doc_mouse) = self.mouse_doc_pos {
            let screen_pos = (self.page_origin + doc_mouse * self.zoom).round() + 0.5;
            let stroke = Stroke::new(1.0, colors::ruler_mouse_indicator());
            let points = if horizontal {
                [
                    egui::Pos2::new(screen_pos, rect.top()),
                    egui::Pos2::new(screen_pos, rect.bottom()),
                ]
            } else {
                [
                    egui::Pos2::new(rect.left(), screen_pos),
                    egui::Pos2::new(rect.right(), screen_pos),
                ]
            };
            painter.extend(egui::Shape::dashed_line(&points, stroke, 1.0, 1.0));
        }
    }

    fn handle_interaction(&self, response: &Response) -> Option<RulerInteraction> {
        if !(response.dragged() || response.drag_stopped()) {
            return None;
        }
        let pos = response.interact_pointer_pos()?;
        // Dragging off the top ruler makes a horizontal guide at the cursor's Y,
        // dragging off the left ruler a vertical guide at the cursor's X.
        let guide_doc_pos = match self.orientation {
            RulerOrientation::Horizontal => ((pos.y - self.cross_origin) / self.zoom) as f64,
            RulerOrientation::Vertical => ((pos.x - self.cross_origin) / self.zoom) as f64,
        };
        if response.drag_stopped() {
            Some(RulerInteraction::FinishGuide(guide_doc_pos))
        } else {
            Some(RulerInteraction::CreateGuide(guide_doc_pos))
        }
    }
}
