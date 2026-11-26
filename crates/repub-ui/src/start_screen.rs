//! Start screen and Backstage (File) view.
//!
//! With no document open, the start screen fills the area below the ribbon:
//! a teal rail on the left (Home, New, Open, Recent files) and a white body
//! with blank page tiles. With a document open, the same rail becomes the
//! Backstage view (Info, New, Open, Save, Save As, Print, Export, Close),
//! drawn over the whole window until the back arrow or Esc closes it.

use std::path::{Path, PathBuf};

use egui::{Align2, Color32, CursorIcon, FontId, Pos2, Rect, Rounding, Sense, Shape, Stroke, Vec2};
use repub_model::{PageOrientation, PageSize};

use crate::app::AppAction;
use crate::theme::colors;

const RAIL_WIDTH: f32 = 168.0;
const RAIL_ITEM_HEIGHT: f32 = 38.0;
const RAIL_TEXT_INSET: f32 = 26.0;
const BODY_INSET_X: f32 = 40.0;
const TILE_SIZE: f32 = 124.0;
const TILE_LABEL_HEIGHT: f32 = 38.0;
const TILE_GAP: f32 = 16.0;
const RECENT_ROW_HEIGHT: f32 = 44.0;

/// Page shown in the body of the start screen or Backstage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartPage {
    Home,
    Info,
    New,
    Open,
}

/// Result of an interaction with the start screen or Backstage.
#[derive(Debug, Clone)]
pub enum StartScreenAction {
    /// Create a one-page blank publication.
    CreateBlank {
        page_size: PageSize,
        orientation: PageOrientation,
    },
    /// Open the New Document dialog for other sizes.
    MoreSizes,
    /// Open a file from the recent list.
    OpenRecent(PathBuf),
    /// Run an app command (Open, Save, Save As, Export).
    Command(AppAction),
    /// Close the current publication.
    CloseDocument,
}

/// Information about the open publication, shown on the Backstage Info page.
#[derive(Debug, Clone)]
pub struct DocumentInfo {
    pub name: String,
    pub path: Option<PathBuf>,
    pub pages: usize,
    /// First page size in points.
    pub page_size: (f64, f64),
    pub dirty: bool,
}

struct BlankPreset {
    label: &'static str,
    size: PageSize,
    orientation: PageOrientation,
}

const BLANK_PRESETS: &[BlankPreset] = &[
    BlankPreset {
        label: "Blank 8.5 x 11\"",
        size: PageSize::Letter,
        orientation: PageOrientation::Portrait,
    },
    BlankPreset {
        label: "Blank 11 x 8.5\"",
        size: PageSize::Letter,
        orientation: PageOrientation::Landscape,
    },
    BlankPreset {
        label: "A4 (210 x 297 mm)",
        size: PageSize::A4,
        orientation: PageOrientation::Portrait,
    },
    BlankPreset {
        label: "A5 (148 x 210 mm)",
        size: PageSize::A5,
        orientation: PageOrientation::Portrait,
    },
    BlankPreset {
        label: "Legal 8.5 x 14\"",
        size: PageSize::Legal,
        orientation: PageOrientation::Portrait,
    },
    BlankPreset {
        label: "Tabloid 11 x 17\"",
        size: PageSize::Tabloid,
        orientation: PageOrientation::Portrait,
    },
];

/// Start screen and Backstage view state.
#[derive(Debug)]
pub struct StartScreen {
    backstage_open: bool,
    page: StartPage,
}

impl Default for StartScreen {
    fn default() -> Self {
        Self {
            backstage_open: false,
            page: StartPage::Home,
        }
    }
}

impl StartScreen {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open Backstage on the Info page. Call this from the ribbon's File tab.
    pub fn open_backstage(&mut self) {
        self.backstage_open = true;
        self.page = StartPage::Info;
    }

    /// Open Backstage on the New page (Ctrl/Cmd+N with a document open).
    pub fn open_backstage_new(&mut self) {
        self.backstage_open = true;
        self.page = StartPage::New;
    }

    /// Close Backstage and return to the document.
    pub fn close_backstage(&mut self) {
        self.backstage_open = false;
        self.page = StartPage::Home;
    }

    /// Whether Backstage is open. Only meaningful while a document is open.
    pub fn is_backstage_open(&self) -> bool {
        self.backstage_open
    }

    /// Select a page of the start screen (used when no document is open).
    pub fn show_page(&mut self, page: StartPage) {
        self.page = page;
    }

    /// Show the start screen in `ui`, filling its available area.
    pub fn show_start(
        &mut self,
        ui: &mut egui::Ui,
        recent: &[PathBuf],
    ) -> Option<StartScreenAction> {
        if matches!(self.page, StartPage::Info) {
            self.page = StartPage::Home;
        }
        let rect = ui.available_rect_before_wrap();
        ui.allocate_rect(rect, Sense::hover());
        self.show_in_rect(ui, rect, recent, None)
    }

    /// Show Backstage over the whole window. Returns an action when one is chosen.
    pub fn show_backstage(
        &mut self,
        ctx: &egui::Context,
        recent: &[PathBuf],
        info: DocumentInfo,
    ) -> Option<StartScreenAction> {
        if !self.backstage_open {
            return None;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.close_backstage();
            return None;
        }
        let mut screen = ctx.screen_rect();
        // macOS draws its own title row under the transparent title bar; keep it visible
        if cfg!(target_os = "macos") {
            screen.min.y += crate::theme::metrics::TITLE_BAR_HEIGHT;
        }
        egui::Area::new(egui::Id::new("repub_backstage"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                ui.set_clip_rect(screen);
                // Swallow clicks so nothing underneath reacts
                ui.allocate_rect(screen, Sense::click_and_drag());
                self.show_in_rect(ui, screen, recent, Some(&info))
            })
            .inner
    }

    fn show_in_rect(
        &mut self,
        ui: &mut egui::Ui,
        rect: Rect,
        recent: &[PathBuf],
        info: Option<&DocumentInfo>,
    ) -> Option<StartScreenAction> {
        let rail_rect =
            Rect::from_min_max(rect.min, Pos2::new(rect.left() + RAIL_WIDTH, rect.bottom()));
        let body_rect = Rect::from_min_max(Pos2::new(rail_rect.right(), rect.top()), rect.max);

        ui.painter()
            .rect_filled(rail_rect, 0.0, colors::backstage_rail());
        ui.painter()
            .rect_filled(body_rect, 0.0, colors::backstage_body());

        let mut action = None;

        let mut rail_ui = ui.new_child(egui::UiBuilder::new().max_rect(rail_rect));
        rail_ui.set_clip_rect(rail_rect);
        rail_ui.spacing_mut().item_spacing = Vec2::ZERO;
        if let Some(a) = self.rail(&mut rail_ui, recent, info.is_some()) {
            action = Some(a);
        }

        let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body_rect));
        body_ui.set_clip_rect(body_rect);
        egui::ScrollArea::vertical()
            .id_salt("start_screen_body")
            .auto_shrink([false, false])
            .show(&mut body_ui, |ui| {
                ui.add_space(28.0);
                ui.horizontal(|ui| {
                    ui.add_space(BODY_INSET_X);
                    ui.vertical(|ui| {
                        ui.set_max_width(body_rect.width() - BODY_INSET_X * 2.0);
                        let a = match self.page {
                            StartPage::Home => body_home(ui, recent),
                            StartPage::New => body_new(ui),
                            StartPage::Open => body_open(ui, recent),
                            StartPage::Info => info.and_then(|info| body_info(ui, info)),
                        };
                        if a.is_some() {
                            action = a;
                        }
                    });
                });
                ui.add_space(28.0);
            });

        action
    }

    fn rail(
        &mut self,
        ui: &mut egui::Ui,
        recent: &[PathBuf],
        backstage: bool,
    ) -> Option<StartScreenAction> {
        let mut action = None;

        if backstage {
            ui.add_space(14.0);
            if back_button(ui).clicked() {
                self.close_backstage();
            }
            ui.add_space(14.0);

            if rail_item(ui, "Info", self.page == StartPage::Info).clicked() {
                self.page = StartPage::Info;
            }
            if rail_item(ui, "New", self.page == StartPage::New).clicked() {
                self.page = StartPage::New;
            }
            if rail_item(ui, "Open", self.page == StartPage::Open).clicked() {
                self.page = StartPage::Open;
            }
            if rail_item(ui, "Save", false).clicked() {
                action = Some(StartScreenAction::Command(AppAction::SaveDocument));
            }
            if rail_item(ui, "Save As", false).clicked() {
                action = Some(StartScreenAction::Command(AppAction::SaveDocumentAs));
            }
            if rail_item(ui, "Print", false).clicked() {
                action = Some(StartScreenAction::Command(AppAction::Print));
            }
            if rail_item(ui, "Export", false).clicked() {
                action = Some(StartScreenAction::Command(AppAction::Export));
            }
            if rail_item(ui, "Close", false).clicked() {
                action = Some(StartScreenAction::CloseDocument);
            }
        } else {
            ui.add_space(22.0);
            let (title_rect, _) =
                ui.allocate_exact_size(Vec2::new(RAIL_WIDTH, 40.0), Sense::hover());
            ui.painter().text(
                Pos2::new(title_rect.left() + RAIL_TEXT_INSET, title_rect.center().y),
                Align2::LEFT_CENTER,
                "Repub",
                FontId::proportional(26.0),
                Color32::WHITE,
            );
            ui.add_space(18.0);

            if rail_item(ui, "Home", self.page == StartPage::Home).clicked() {
                self.page = StartPage::Home;
            }
            if rail_item(ui, "New", self.page == StartPage::New).clicked() {
                self.page = StartPage::New;
            }
            if rail_item(ui, "Open", self.page == StartPage::Open).clicked() {
                self.page = StartPage::Open;
            }

            if !recent.is_empty() {
                ui.add_space(12.0);
                rail_divider(ui);
                ui.add_space(10.0);
                let (r, _) = ui.allocate_exact_size(Vec2::new(RAIL_WIDTH, 24.0), Sense::hover());
                ui.painter().text(
                    Pos2::new(r.left() + RAIL_TEXT_INSET - 8.0, r.center().y),
                    Align2::LEFT_CENTER,
                    "Recent",
                    FontId::proportional(14.0),
                    Color32::WHITE,
                );
                ui.add_space(4.0);
                for path in recent.iter().take(8) {
                    if rail_recent_item(ui, path).clicked() {
                        action = Some(StartScreenAction::OpenRecent(path.clone()));
                    }
                }
            }
        }

        action
    }
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string())
}

fn parent_dir(path: &Path) -> String {
    path.parent()
        .map(|p| p.display().to_string())
        .unwrap_or_default()
}

fn rail_item(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(RAIL_WIDTH, RAIL_ITEM_HEIGHT), Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    let fill = if selected {
        Some(colors::backstage_rail_selected())
    } else if response.hovered() {
        Some(colors::backstage_rail_hover())
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().rect_filled(rect, 0.0, fill);
    }
    ui.painter().text(
        Pos2::new(rect.left() + RAIL_TEXT_INSET, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        Color32::WHITE,
    );
    response
}

fn rail_recent_item(ui: &mut egui::Ui, path: &Path) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(RAIL_WIDTH, 40.0), Sense::click());
    let response = response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text(path.display().to_string());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, 0.0, colors::backstage_rail_hover());
    }
    let icon = Rect::from_min_size(
        Pos2::new(rect.left() + 14.0, rect.center().y - 9.0),
        Vec2::new(14.0, 18.0),
    );
    draw_doc_glyph(ui.painter(), icon, Color32::WHITE, colors::backstage_rail());
    let text_x = icon.right() + 8.0;
    let clip = Rect::from_min_max(
        Pos2::new(text_x, rect.top()),
        Pos2::new(rect.right() - 6.0, rect.bottom()),
    );
    let painter = ui.painter().with_clip_rect(clip);
    painter.text(
        Pos2::new(text_x, rect.top() + 12.0),
        Align2::LEFT_CENTER,
        file_name(path),
        FontId::proportional(12.0),
        Color32::WHITE,
    );
    painter.text(
        Pos2::new(text_x, rect.top() + 27.0),
        Align2::LEFT_CENTER,
        parent_dir(path),
        FontId::proportional(10.0),
        Color32::from_rgb(0xC6, 0xE3, 0xDE),
    );
    response
}

fn rail_divider(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(RAIL_WIDTH, 1.0), Sense::hover());
    ui.painter().hline(
        (rect.left() + RAIL_TEXT_INSET)..=(rect.right() - RAIL_TEXT_INSET),
        rect.center().y,
        Stroke::new(1.0, colors::backstage_rail_divider()),
    );
}

fn back_button(ui: &mut egui::Ui) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(RAIL_WIDTH, 44.0), Sense::click());
    let response = response
        .on_hover_cursor(CursorIcon::PointingHand)
        .on_hover_text("Back");
    let center = Pos2::new(rect.left() + 18.0 + 18.0, rect.center().y);
    let painter = ui.painter();
    if response.hovered() {
        painter.circle_filled(center, 18.0, colors::backstage_rail_hover());
    }
    let stroke = Stroke::new(1.5, Color32::WHITE);
    painter.circle_stroke(center, 17.0, stroke);
    painter.line_segment(
        [center + Vec2::new(-8.0, 0.0), center + Vec2::new(8.0, 0.0)],
        stroke,
    );
    painter.line_segment(
        [
            center + Vec2::new(-8.0, 0.0),
            center + Vec2::new(-2.0, -6.0),
        ],
        stroke,
    );
    painter.line_segment(
        [center + Vec2::new(-8.0, 0.0), center + Vec2::new(-2.0, 6.0)],
        stroke,
    );
    response
}

fn page_heading(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .font(FontId::proportional(34.0))
            .color(colors::text_primary()),
    );
    ui.add_space(18.0);
}

fn section_heading(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text)
            .font(FontId::proportional(17.0))
            .color(colors::text_primary()),
    );
    ui.add_space(10.0);
}

fn link(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let galley = ui.painter().layout_no_wrap(
        text.to_owned(),
        FontId::proportional(13.0),
        colors::accent(),
    );
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    ui.painter().galley(rect.min, galley, colors::accent());
    if response.hovered() {
        ui.painter().hline(
            rect.x_range(),
            rect.bottom(),
            Stroke::new(1.0, colors::accent()),
        );
    }
    response
}

fn blank_tiles(ui: &mut egui::Ui) -> Option<StartScreenAction> {
    let mut action = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(TILE_GAP, TILE_GAP);
        for preset in BLANK_PRESETS {
            let base = preset.size.to_points();
            let (w, h) = match preset.orientation {
                PageOrientation::Portrait => (base.width, base.height),
                PageOrientation::Landscape => (base.height, base.width),
            };
            if blank_tile(ui, preset.label, w as f32 / h as f32).clicked() {
                action = Some(StartScreenAction::CreateBlank {
                    page_size: preset.size.clone(),
                    orientation: preset.orientation,
                });
            }
        }
    });
    ui.add_space(12.0);
    if link(ui, "More blank page sizes…").clicked() {
        action = Some(StartScreenAction::MoreSizes);
    }
    if link(ui, "Start from a template…").clicked() {
        action = Some(StartScreenAction::Command(AppAction::NewFromTemplate));
    }
    action
}

/// A page-shaped tile: white square with a dog-eared page drawn inside, label beneath.
fn blank_tile(ui: &mut egui::Ui, label: &str, aspect: f32) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(TILE_SIZE, TILE_SIZE + TILE_LABEL_HEIGHT),
        Sense::click(),
    );
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    let painter = ui.painter();
    let hovered = response.hovered();

    let tile = Rect::from_min_size(rect.min, Vec2::splat(TILE_SIZE));
    let (fill, border) = if hovered {
        (colors::tile_hover(), colors::accent())
    } else {
        (Color32::WHITE, colors::tile_border())
    };
    painter.rect(tile, Rounding::ZERO, fill, Stroke::new(1.0, border));

    let max = TILE_SIZE - 30.0;
    let (pw, ph) = if aspect >= 1.0 {
        (max, max / aspect)
    } else {
        (max * aspect, max)
    };
    let page = Rect::from_center_size(tile.center(), Vec2::new(pw, ph));
    let fold = 14.0_f32.min(pw * 0.25);
    let outline = Color32::from_rgb(0x7A, 0x7A, 0x7A);
    let stroke = Stroke::new(1.0, outline);
    painter.add(Shape::convex_polygon(
        vec![
            page.left_top(),
            Pos2::new(page.right() - fold, page.top()),
            Pos2::new(page.right(), page.top() + fold),
            page.right_bottom(),
            page.left_bottom(),
        ],
        Color32::WHITE,
        stroke,
    ));
    painter.add(Shape::convex_polygon(
        vec![
            Pos2::new(page.right() - fold, page.top()),
            Pos2::new(page.right() - fold, page.top() + fold),
            Pos2::new(page.right(), page.top() + fold),
        ],
        Color32::from_rgb(0xEC, 0xEC, 0xEC),
        stroke,
    ));

    let galley = painter.layout(
        label.to_owned(),
        FontId::proportional(12.0),
        colors::text_primary(),
        TILE_SIZE,
    );
    let pos = Pos2::new(tile.center().x - galley.size().x / 2.0, tile.bottom() + 8.0);
    painter.galley(pos, galley, colors::text_primary());

    response
}

fn draw_doc_glyph(painter: &egui::Painter, rect: Rect, paper: Color32, ink: Color32) {
    let fold = rect.width() * 0.35;
    painter.add(Shape::convex_polygon(
        vec![
            rect.left_top(),
            Pos2::new(rect.right() - fold, rect.top()),
            Pos2::new(rect.right(), rect.top() + fold),
            rect.right_bottom(),
            rect.left_bottom(),
        ],
        paper,
        Stroke::NONE,
    ));
    painter.text(
        Pos2::new(rect.center().x, rect.center().y + 2.0),
        Align2::CENTER_CENTER,
        "P",
        FontId::proportional(rect.height() * 0.55),
        ink,
    );
}

fn recent_list(ui: &mut egui::Ui, recent: &[PathBuf]) -> Option<StartScreenAction> {
    if recent.is_empty() {
        ui.label(
            egui::RichText::new("You haven't opened any publications recently.")
                .font(FontId::proportional(13.0))
                .color(colors::text_secondary()),
        );
        return None;
    }
    let mut action = None;
    let width = ui.available_width().min(760.0);
    // Column header
    let (hdr, _) = ui.allocate_exact_size(Vec2::new(width, 24.0), Sense::hover());
    ui.painter().text(
        Pos2::new(hdr.left() + 44.0, hdr.center().y),
        Align2::LEFT_CENTER,
        "Name",
        FontId::proportional(12.0),
        colors::text_secondary(),
    );
    ui.painter().hline(
        hdr.x_range(),
        hdr.bottom(),
        Stroke::new(1.0, colors::border()),
    );
    for path in recent {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(width, RECENT_ROW_HEIGHT), Sense::click());
        let response = response.on_hover_cursor(CursorIcon::PointingHand);
        if response.hovered() {
            ui.painter().rect_filled(rect, 0.0, colors::tile_hover());
        }
        let icon = Rect::from_min_size(
            Pos2::new(rect.left() + 14.0, rect.center().y - 11.0),
            Vec2::new(18.0, 22.0),
        );
        draw_doc_glyph(ui.painter(), icon, colors::accent(), Color32::WHITE);
        let painter = ui.painter().with_clip_rect(rect);
        painter.text(
            Pos2::new(rect.left() + 44.0, rect.top() + 14.0),
            Align2::LEFT_CENTER,
            file_name(path),
            FontId::proportional(13.0),
            colors::text_primary(),
        );
        painter.text(
            Pos2::new(rect.left() + 44.0, rect.top() + 30.0),
            Align2::LEFT_CENTER,
            parent_dir(path),
            FontId::proportional(11.0),
            colors::text_secondary(),
        );
        if response.clicked() {
            action = Some(StartScreenAction::OpenRecent(path.clone()));
        }
    }
    action
}

fn body_home(ui: &mut egui::Ui, recent: &[PathBuf]) -> Option<StartScreenAction> {
    page_heading(ui, "Welcome to Repub");
    section_heading(ui, "New");
    let mut action = blank_tiles(ui);
    ui.add_space(28.0);
    section_heading(ui, "Recent");
    if let Some(a) = recent_list(ui, recent) {
        action = Some(a);
    }
    action
}

fn body_new(ui: &mut egui::Ui) -> Option<StartScreenAction> {
    page_heading(ui, "New");
    section_heading(ui, "Blank page sizes");
    blank_tiles(ui)
}

fn body_open(ui: &mut egui::Ui, recent: &[PathBuf]) -> Option<StartScreenAction> {
    page_heading(ui, "Open");
    let mut action = None;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(260.0, 48.0), Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);
    if response.hovered() {
        ui.painter().rect_filled(rect, 0.0, colors::tile_hover());
    }
    draw_folder_glyph(
        ui.painter(),
        Rect::from_min_size(
            Pos2::new(rect.left() + 12.0, rect.center().y - 10.0),
            Vec2::new(26.0, 20.0),
        ),
    );
    ui.painter().text(
        Pos2::new(rect.left() + 52.0, rect.center().y),
        Align2::LEFT_CENTER,
        "Browse",
        FontId::proportional(14.0),
        colors::text_primary(),
    );
    if response.clicked() {
        action = Some(StartScreenAction::Command(AppAction::OpenDocument));
    }
    ui.add_space(24.0);
    section_heading(ui, "Recent");
    if let Some(a) = recent_list(ui, recent) {
        action = Some(a);
    }
    action
}

fn draw_folder_glyph(painter: &egui::Painter, rect: Rect) {
    let color = Color32::from_rgb(0xE8, 0xB8, 0x4A);
    let tab = Rect::from_min_size(rect.min, Vec2::new(rect.width() * 0.45, 4.0));
    painter.rect_filled(tab, 1.0, color);
    let body = Rect::from_min_max(Pos2::new(rect.left(), rect.top() + 3.0), rect.max);
    painter.rect_filled(body, 1.0, color);
    painter.rect_stroke(
        body,
        1.0,
        Stroke::new(1.0, Color32::from_rgb(0xC4, 0x92, 0x28)),
    );
}

fn body_info(ui: &mut egui::Ui, info: &DocumentInfo) -> Option<StartScreenAction> {
    page_heading(ui, "Info");
    ui.label(
        egui::RichText::new(&info.name)
            .font(FontId::proportional(20.0))
            .color(colors::text_primary()),
    );
    let location = info
        .path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "Not saved yet".to_owned());
    ui.label(
        egui::RichText::new(location)
            .font(FontId::proportional(12.0))
            .color(colors::text_secondary()),
    );
    ui.add_space(24.0);

    section_heading(ui, "Publication Properties");
    let (w, h) = info.page_size;
    let rows = [
        ("Pages", info.pages.to_string()),
        (
            "Page size",
            format!("{:.2} x {:.2} in.", w / 72.0, h / 72.0),
        ),
        (
            "Status",
            if info.dirty {
                "Unsaved changes".to_owned()
            } else {
                "Saved".to_owned()
            },
        ),
    ];
    egui::Grid::new("backstage_info_props")
        .num_columns(2)
        .spacing(Vec2::new(48.0, 8.0))
        .show(ui, |ui| {
            for (k, v) in rows {
                ui.label(
                    egui::RichText::new(k)
                        .font(FontId::proportional(13.0))
                        .color(colors::text_primary()),
                );
                ui.label(
                    egui::RichText::new(v)
                        .font(FontId::proportional(13.0))
                        .color(colors::text_secondary()),
                );
                ui.end_row();
            }
        });
    ui.add_space(16.0);
    if ui.button("Document Properties…").clicked() {
        return Some(StartScreenAction::Command(
            AppAction::ShowDocumentProperties,
        ));
    }
    None
}
