//! Web application - adapted from desktop GUI.

use std::cell::RefCell;
use std::rc::Rc;

use repub_export_vector::{export_pdf_bytes, export_svg_bytes, PdfOptions, SvgOptions};
use repub_fonts::SystemFontProvider;
use repub_format_native::{load_bytes, save_bytes, SaveOptions};
use repub_geometry::{Point, Rect};
use repub_model::geometry::ShapeGeometry;
use repub_model::shape::{Shape, ShapeKind};
use repub_model::{Document, PageOrientation, PageSize};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

use crate::platform;
use crate::storage::{AutoSave, WebSettings};

/// Tool types available in the editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Select,
    Text,
    Rectangle,
    Ellipse,
    Line,
    Polygon,
    Star,
    Pan,
}

impl Tool {
    fn name(&self) -> &'static str {
        match self {
            Tool::Select => "Select",
            Tool::Text => "Text",
            Tool::Rectangle => "Rectangle",
            Tool::Ellipse => "Ellipse",
            Tool::Line => "Line",
            Tool::Polygon => "Polygon",
            Tool::Star => "Star",
            Tool::Pan => "Pan",
        }
    }

    fn cursor(&self) -> egui::CursorIcon {
        match self {
            Tool::Select => egui::CursorIcon::Default,
            Tool::Text => egui::CursorIcon::Text,
            Tool::Rectangle | Tool::Ellipse | Tool::Polygon | Tool::Star | Tool::Line => {
                egui::CursorIcon::Crosshair
            }
            Tool::Pan => egui::CursorIcon::Grab,
        }
    }
}

/// Drawing state for shape creation.
#[derive(Debug, Default)]
struct DrawState {
    is_drawing: bool,
    start: Option<Point>,
    current: Option<Point>,
    constrain: bool,
}

impl DrawState {
    fn start(&mut self, point: Point) {
        self.is_drawing = true;
        self.start = Some(point);
        self.current = Some(point);
    }

    fn update(&mut self, point: Point) {
        self.current = Some(point);
    }

    fn end(&mut self) -> Option<Rect> {
        let result = self.bounds();
        self.is_drawing = false;
        self.start = None;
        self.current = None;
        result
    }

    fn bounds(&self) -> Option<Rect> {
        let start = self.start?;
        let end = self.current?;

        let mut min_x = start.x.min(end.x);
        let mut min_y = start.y.min(end.y);
        let mut max_x = start.x.max(end.x);
        let mut max_y = start.y.max(end.y);

        if self.constrain {
            let size = (max_x - min_x).max(max_y - min_y);
            if end.x < start.x {
                min_x = start.x - size;
                max_x = start.x;
            } else {
                max_x = start.x + size;
            }
            if end.y < start.y {
                min_y = start.y - size;
                max_y = start.y;
            } else {
                max_y = start.y + size;
            }
        }

        let width = max_x - min_x;
        let height = max_y - min_y;

        if width < 1.0 || height < 1.0 {
            return None;
        }

        Some(Rect::new(min_x, min_y, width, height))
    }

    fn line(&self) -> Option<(Point, Point)> {
        let start = self.start?;
        let end = self.current?;
        Some((start, end))
    }

    fn create_object(&self, tool: Tool) -> Option<Shape> {
        match tool {
            Tool::Rectangle => Some(Shape::rectangle(self.bounds()?)),
            Tool::Ellipse => Some(Shape::ellipse(self.bounds()?)),
            Tool::Polygon => Some(Shape::polygon(self.bounds()?, 6)),
            Tool::Star => Some(Shape::star(self.bounds()?, 5, 0.5)),
            Tool::Line => {
                let (start, end) = self.line()?;
                Some(Shape::line(start, end))
            }
            Tool::Text => Some(Shape::text_frame(self.bounds()?)),
            _ => None,
        }
    }
}

/// Main web application.
pub struct WebApp {
    /// Current document
    document: Option<Document>,
    /// Document name
    document_name: String,
    /// Has unsaved changes
    dirty: bool,
    /// Current page index
    current_page: usize,
    /// Zoom level
    zoom: f32,
    /// Pan offset
    pan_offset: Point,
    /// Current tool
    current_tool: Tool,
    /// Drawing state
    draw_state: DrawState,
    /// Selected object indices
    selection: Vec<usize>,
    /// Settings
    settings: WebSettings,
    /// Auto-save handler
    auto_save: Option<AutoSave>,
    /// File input element
    file_input: Option<HtmlInputElement>,
    /// Pending file data
    pending_file: Rc<RefCell<Option<(Vec<u8>, String)>>>,
    /// Show new document dialog
    show_new_dialog: bool,
    /// Show about dialog
    show_about_dialog: bool,
    /// New document page size
    new_doc_size: String,
    /// New document orientation
    new_doc_orientation: PageOrientation,
    /// Is touch device
    is_touch: bool,
}

impl WebApp {
    /// Create a new web application.
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let settings = WebSettings::load();
        let is_touch = platform::is_touch_device();

        // Set up file input for opening files
        let pending_file = Rc::new(RefCell::new(None));
        let pending_clone = pending_file.clone();

        let file_input = platform::create_file_input(".pcd,.pcdz,.json", move |data, name| {
            *pending_clone.borrow_mut() = Some((data, name));
        })
        .ok();

        Self {
            document: None,
            document_name: "Untitled".to_string(),
            dirty: false,
            current_page: 0,
            zoom: settings.default_zoom,
            pan_offset: Point::ZERO,
            current_tool: Tool::Select,
            draw_state: DrawState::default(),
            selection: Vec::new(),
            settings,
            auto_save: None,
            file_input,
            pending_file,
            show_new_dialog: false,
            show_about_dialog: false,
            new_doc_size: "letter".to_string(),
            new_doc_orientation: PageOrientation::Portrait,
            is_touch,
        }
    }

    fn window_title(&self) -> String {
        let dirty_marker = if self.dirty { "* " } else { "" };
        if self.document.is_some() {
            format!("{}{} - Repub", dirty_marker, self.document_name)
        } else {
            "Repub".to_string()
        }
    }

    fn new_document(&mut self, size: PageSize, orientation: PageOrientation) {
        let doc = Document::new(size, orientation);
        self.document = Some(doc);
        self.document_name = "Untitled".to_string();
        self.dirty = true;
        self.current_page = 0;
        self.selection.clear();

        // Set up auto-save
        if self.settings.auto_save {
            self.auto_save = Some(AutoSave::new(
                &self.document_name,
                self.settings.auto_save_interval,
            ));
        }
    }

    fn save_document(&mut self) {
        if let Some(ref doc) = self.document {
            // The model is serialization-free; serialize via the native-format
            // DTO layer (compact JSON).
            match save_bytes(doc, &SaveOptions::compact()) {
                Ok(bytes) => {
                    let filename = if self.document_name.ends_with(".pcd")
                        || self.document_name.ends_with(".json")
                    {
                        self.document_name.clone()
                    } else {
                        format!("{}.pcd", self.document_name)
                    };

                    if let Err(e) = platform::download_file(&filename, &bytes, "application/json") {
                        log::error!("Failed to download: {:?}", e);
                    } else {
                        self.dirty = false;
                    }
                }
                Err(e) => {
                    log::error!("Failed to serialize: {}", e);
                }
            }
        }
    }

    fn export_pdf(&self) {
        if let Some(ref doc) = self.document {
            let fonts = match SystemFontProvider::new() {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to load fonts: {}", e);
                    return;
                }
            };
            let opts = PdfOptions::default();

            match export_pdf_bytes(doc, &opts, &fonts) {
                Ok(buffer) => {
                    let filename = format!("{}.pdf", self.document_name.trim_end_matches(".pcd"));
                    if let Err(e) = platform::download_file(&filename, &buffer, "application/pdf") {
                        log::error!("Failed to download PDF: {:?}", e);
                    }
                }
                Err(e) => {
                    log::error!("Failed to export PDF: {}", e);
                }
            }
        }
    }

    fn export_svg(&self) {
        if let Some(ref doc) = self.document {
            let fonts = match SystemFontProvider::new() {
                Ok(f) => f,
                Err(e) => {
                    log::error!("Failed to load fonts: {}", e);
                    return;
                }
            };
            let opts = SvgOptions::default();

            match export_svg_bytes(doc, self.current_page, &opts, &fonts) {
                Ok(svg) => {
                    let filename = format!(
                        "{}-page{}.svg",
                        self.document_name.trim_end_matches(".pcd"),
                        self.current_page + 1
                    );
                    if let Err(e) = platform::download_file(&filename, &svg, "image/svg+xml") {
                        log::error!("Failed to download SVG: {:?}", e);
                    }
                }
                Err(e) => {
                    log::error!("Failed to export SVG: {}", e);
                }
            }
        }
    }

    fn process_pending_file(&mut self) {
        let pending = self.pending_file.borrow_mut().take();
        if let Some((data, name)) = pending {
            // The native-format JSON container (uncompressed `.pcd`/`.json`).
            match load_bytes(&data, false) {
                Ok(doc) => {
                    self.document = Some(doc);
                    self.document_name = name;
                    self.dirty = false;
                    self.current_page = 0;
                    self.selection.clear();
                    log::info!("Loaded document: {}", self.document_name);
                }
                Err(e) => {
                    log::error!("Failed to parse document: {}", e);
                    platform::alert(&format!("Failed to open file: {}", e));
                }
            }
        }
    }

    fn show_menu_bar(&mut self, ui: &mut egui::Ui) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New...").clicked() {
                    self.show_new_dialog = true;
                    ui.close_menu();
                }
                if ui.button("Open...").clicked() {
                    if let Some(ref input) = self.file_input {
                        platform::open_file_picker(input);
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui
                    .add_enabled(self.document.is_some(), egui::Button::new("Save"))
                    .clicked()
                {
                    self.save_document();
                    ui.close_menu();
                }
                ui.separator();
                ui.menu_button("Export", |ui| {
                    if ui
                        .add_enabled(self.document.is_some(), egui::Button::new("PDF"))
                        .clicked()
                    {
                        self.export_pdf();
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(self.document.is_some(), egui::Button::new("SVG"))
                        .clicked()
                    {
                        self.export_svg();
                        ui.close_menu();
                    }
                });
            });

            ui.menu_button("Edit", |ui| {
                if ui.button("Select All").clicked() {
                    // TODO: Implement select all
                    ui.close_menu();
                }
            });

            ui.menu_button("View", |ui| {
                if ui.button("Zoom In").clicked() {
                    self.zoom = (self.zoom * 1.25).min(8.0);
                    ui.close_menu();
                }
                if ui.button("Zoom Out").clicked() {
                    self.zoom = (self.zoom / 1.25).max(0.1);
                    ui.close_menu();
                }
                if ui.button("Actual Size").clicked() {
                    self.zoom = 1.0;
                    ui.close_menu();
                }
            });

            ui.menu_button("Insert", |ui| {
                let has_doc = self.document.is_some();
                if ui
                    .add_enabled(has_doc, egui::Button::new("Text Frame"))
                    .clicked()
                {
                    self.current_tool = Tool::Text;
                    ui.close_menu();
                }
                ui.separator();
                if ui
                    .add_enabled(has_doc, egui::Button::new("Rectangle"))
                    .clicked()
                {
                    self.current_tool = Tool::Rectangle;
                    ui.close_menu();
                }
                if ui
                    .add_enabled(has_doc, egui::Button::new("Ellipse"))
                    .clicked()
                {
                    self.current_tool = Tool::Ellipse;
                    ui.close_menu();
                }
                if ui.add_enabled(has_doc, egui::Button::new("Line")).clicked() {
                    self.current_tool = Tool::Line;
                    ui.close_menu();
                }
                ui.separator();
                if ui
                    .add_enabled(has_doc, egui::Button::new("New Page"))
                    .clicked()
                {
                    if let Some(ref mut doc) = self.document {
                        doc.add_page();
                        self.dirty = true;
                    }
                    ui.close_menu();
                }
            });

            ui.menu_button("Help", |ui| {
                if ui.button("About").clicked() {
                    self.show_about_dialog = true;
                    ui.close_menu();
                }
            });
        });
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        let button_size = if self.is_touch {
            egui::vec2(40.0, 40.0)
        } else {
            egui::vec2(28.0, 28.0)
        };

        ui.horizontal(|ui| {
            // Tool buttons
            self.tool_button(ui, Tool::Select, "🖱", button_size);
            self.tool_button(ui, Tool::Text, "T", button_size);
            ui.separator();
            self.tool_button(ui, Tool::Rectangle, "⬜", button_size);
            self.tool_button(ui, Tool::Ellipse, "⬭", button_size);
            self.tool_button(ui, Tool::Line, "╱", button_size);
            self.tool_button(ui, Tool::Polygon, "⬡", button_size);
            self.tool_button(ui, Tool::Star, "★", button_size);
            ui.separator();
            self.tool_button(ui, Tool::Pan, "✋", button_size);

            ui.separator();

            // Zoom controls
            if ui.button("−").clicked() {
                self.zoom = (self.zoom / 1.25).max(0.1);
            }
            ui.label(format!("{:.0}%", self.zoom * 100.0));
            if ui.button("+").clicked() {
                self.zoom = (self.zoom * 1.25).min(8.0);
            }

            ui.separator();

            // Page navigation
            if let Some(ref doc) = self.document {
                let page_count = doc.pages().len();
                if ui
                    .add_enabled(self.current_page > 0, egui::Button::new("◀"))
                    .clicked()
                {
                    self.current_page -= 1;
                }
                ui.label(format!("{}/{}", self.current_page + 1, page_count));
                if ui
                    .add_enabled(self.current_page < page_count - 1, egui::Button::new("▶"))
                    .clicked()
                {
                    self.current_page += 1;
                }
            }
        });
    }

    fn tool_button(&mut self, ui: &mut egui::Ui, tool: Tool, icon: &str, size: egui::Vec2) {
        let is_selected = self.current_tool == tool;
        let button = egui::Button::new(icon).min_size(size);

        let response = if is_selected {
            ui.add(button.fill(egui::Color32::from_rgb(59, 130, 246)))
        } else {
            ui.add(button)
        };

        if response.clicked() {
            self.current_tool = tool;
        }
        response.on_hover_text(tool.name());
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui) {
        let (response, painter) =
            ui.allocate_painter(ui.available_size(), egui::Sense::click_and_drag());

        let rect = response.rect;

        // Background
        painter.rect_filled(
            rect,
            egui::Rounding::ZERO,
            egui::Color32::from_rgb(80, 80, 80),
        );

        if let Some(ref doc) = self.document {
            if let Some(page) = doc.pages().get(self.current_page) {
                // Calculate page position
                let page_width = page.size.width as f32 * self.zoom;
                let page_height = page.size.height as f32 * self.zoom;

                let page_x = rect.center().x - page_width / 2.0 + self.pan_offset.x as f32;
                let page_y = rect.center().y - page_height / 2.0 + self.pan_offset.y as f32;

                let page_rect = egui::Rect::from_min_size(
                    egui::pos2(page_x, page_y),
                    egui::vec2(page_width, page_height),
                );

                // Page shadow
                let shadow_rect = page_rect.translate(egui::vec2(4.0, 4.0));
                painter.rect_filled(
                    shadow_rect,
                    egui::Rounding::same(2.0),
                    egui::Color32::from_black_alpha(60),
                );

                // Page background
                painter.rect_filled(page_rect, egui::Rounding::ZERO, egui::Color32::WHITE);
                painter.rect_stroke(
                    page_rect,
                    egui::Rounding::ZERO,
                    egui::Stroke::new(1.0, egui::Color32::GRAY),
                );

                // Draw objects
                self.draw_objects(&painter, page_rect, page);

                // Draw preview
                if self.draw_state.is_drawing {
                    self.draw_preview(&painter, page_rect);
                }

                // Handle input
                self.handle_canvas_input(&response, page_rect);
            }
        } else {
            // Empty state
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "No document open\n\nUse File → New to create a document\nor File → Open to load one",
                egui::FontId::proportional(16.0),
                egui::Color32::WHITE,
            );
        }

        // Set cursor
        if response.hovered() {
            ui.ctx().set_cursor_icon(self.current_tool.cursor());
        }
    }

    fn draw_objects(
        &self,
        painter: &egui::Painter,
        page_rect: egui::Rect,
        page: &repub_model::Page,
    ) {
        for layer in page.layers() {
            if !layer.visible {
                continue;
            }

            for obj in layer.shapes() {
                self.draw_object(painter, page_rect, obj);
            }
        }
    }

    fn draw_object(&self, painter: &egui::Painter, page_rect: egui::Rect, obj: &Shape) {
        let bounds = obj.bounds();
        let screen_rect = egui::Rect::from_min_size(
            egui::pos2(
                page_rect.left() + (bounds.origin.x as f32) * self.zoom,
                page_rect.top() + (bounds.origin.y as f32) * self.zoom,
            ),
            egui::vec2(
                bounds.width() as f32 * self.zoom,
                bounds.height() as f32 * self.zoom,
            ),
        );

        match &obj.kind {
            ShapeKind::AutoShape(auto) => {
                let fill = match &obj.fill {
                    repub_model::styles::Fill::Solid(c) => egui::Color32::from_rgba_unmultiplied(
                        (c.r * 255.0) as u8,
                        (c.g * 255.0) as u8,
                        (c.b * 255.0) as u8,
                        (c.a * 255.0) as u8,
                    ),
                    repub_model::styles::Fill::None => egui::Color32::TRANSPARENT,
                    _ => egui::Color32::from_rgb(200, 200, 200),
                };

                let stroke = if let Some(s) = &obj.line {
                    egui::Stroke::new(
                        s.width as f32 * self.zoom,
                        egui::Color32::from_rgba_unmultiplied(
                            (s.color.r * 255.0) as u8,
                            (s.color.g * 255.0) as u8,
                            (s.color.b * 255.0) as u8,
                            (s.color.a * 255.0) as u8,
                        ),
                    )
                } else {
                    egui::Stroke::NONE
                };

                match &auto.geometry {
                    ShapeGeometry::Rectangle { corner_radius } => {
                        painter.rect(
                            screen_rect,
                            egui::Rounding::same(*corner_radius as f32 * self.zoom),
                            fill,
                            stroke,
                        );
                    }
                    ShapeGeometry::Ellipse => {
                        painter.rect(
                            screen_rect,
                            egui::Rounding::same(
                                screen_rect.width().min(screen_rect.height()) / 2.0,
                            ),
                            fill,
                            stroke,
                        );
                    }
                    _ => {
                        painter.rect(screen_rect, egui::Rounding::ZERO, fill, stroke);
                    }
                }
            }
            ShapeKind::TextFrame(tf) => {
                painter.rect_filled(
                    screen_rect,
                    egui::Rounding::ZERO,
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200),
                );
                painter.rect_stroke(
                    screen_rect,
                    egui::Rounding::ZERO,
                    egui::Stroke::new(1.0, egui::Color32::LIGHT_GRAY),
                );

                let text = tf.content.plain_text();
                if !text.is_empty() {
                    painter.text(
                        screen_rect.left_top() + egui::vec2(4.0, 4.0),
                        egui::Align2::LEFT_TOP,
                        text,
                        egui::FontId::proportional(12.0 * self.zoom),
                        egui::Color32::BLACK,
                    );
                }
            }
            ShapeKind::Line(line) => {
                // The line's stroke lives on the enclosing shape.
                let (width, color) = match &obj.line {
                    Some(s) => (
                        s.width as f32 * self.zoom,
                        egui::Color32::from_rgba_unmultiplied(
                            (s.color.r * 255.0) as u8,
                            (s.color.g * 255.0) as u8,
                            (s.color.b * 255.0) as u8,
                            (s.color.a * 255.0) as u8,
                        ),
                    ),
                    None => (1.0 * self.zoom, egui::Color32::BLACK),
                };
                let start = egui::pos2(
                    page_rect.left() + (line.start.x as f32) * self.zoom,
                    page_rect.top() + (line.start.y as f32) * self.zoom,
                );
                let end = egui::pos2(
                    page_rect.left() + (line.end.x as f32) * self.zoom,
                    page_rect.top() + (line.end.y as f32) * self.zoom,
                );
                painter.line_segment([start, end], egui::Stroke::new(width, color));
            }
            _ => {
                painter.rect_stroke(
                    screen_rect,
                    egui::Rounding::ZERO,
                    egui::Stroke::new(1.0, egui::Color32::GRAY),
                );
            }
        }
    }

    fn draw_preview(&self, painter: &egui::Painter, page_rect: egui::Rect) {
        if let Some(bounds) = self.draw_state.bounds() {
            let screen_rect = egui::Rect::from_min_size(
                egui::pos2(
                    page_rect.left() + (bounds.origin.x as f32) * self.zoom,
                    page_rect.top() + (bounds.origin.y as f32) * self.zoom,
                ),
                egui::vec2(
                    bounds.width() as f32 * self.zoom,
                    bounds.height() as f32 * self.zoom,
                ),
            );
            painter.rect_stroke(
                screen_rect,
                egui::Rounding::ZERO,
                egui::Stroke::new(1.0, egui::Color32::from_rgb(59, 130, 246)),
            );
        } else if let Some((start, end)) = self.draw_state.line() {
            let screen_start = egui::pos2(
                page_rect.left() + (start.x as f32) * self.zoom,
                page_rect.top() + (start.y as f32) * self.zoom,
            );
            let screen_end = egui::pos2(
                page_rect.left() + (end.x as f32) * self.zoom,
                page_rect.top() + (end.y as f32) * self.zoom,
            );
            painter.line_segment(
                [screen_start, screen_end],
                egui::Stroke::new(1.0, egui::Color32::from_rgb(59, 130, 246)),
            );
        }
    }

    fn handle_canvas_input(&mut self, response: &egui::Response, page_rect: egui::Rect) {
        // Update constrain from shift key
        response.ctx.input(|i| {
            self.draw_state.constrain = i.modifiers.shift;
        });

        if let Some(pos) = response.interact_pointer_pos() {
            let doc_pos = Point::new(
                ((pos.x - page_rect.left()) / self.zoom) as f64,
                ((pos.y - page_rect.top()) / self.zoom) as f64,
            );

            match self.current_tool {
                Tool::Pan => {
                    if response.dragged() {
                        let delta = response.drag_delta();
                        self.pan_offset.x += delta.x as f64;
                        self.pan_offset.y += delta.y as f64;
                    }
                }
                tool @ (Tool::Rectangle
                | Tool::Ellipse
                | Tool::Line
                | Tool::Polygon
                | Tool::Star
                | Tool::Text) => {
                    if response.drag_started() {
                        self.draw_state.start(doc_pos);
                    } else if response.dragged() {
                        self.draw_state.update(doc_pos);
                    } else if response.drag_stopped() {
                        if let Some(obj) = self.draw_state.create_object(tool) {
                            if let Some(ref mut doc) = self.document {
                                if let Some(page) = doc.pages_mut().get_mut(self.current_page) {
                                    page.add_shape(obj);
                                    self.dirty = true;
                                }
                            }
                        }
                        self.draw_state.end();
                        self.current_tool = Tool::Select;
                    }
                }
                Tool::Select => {
                    // Simple selection (click to deselect for now)
                    if response.clicked() {
                        self.selection.clear();
                    }
                }
            }
        }

        // Zoom with scroll
        let scroll = response.ctx.input(|i| i.raw_scroll_delta);
        if scroll.y != 0.0 && response.hovered() {
            if response.ctx.input(|i| i.modifiers.ctrl) {
                if scroll.y > 0.0 {
                    self.zoom = (self.zoom * 1.1).min(8.0);
                } else {
                    self.zoom = (self.zoom / 1.1).max(0.1);
                }
            }
        }
    }

    fn show_new_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_new_dialog {
            return;
        }

        egui::Window::new("New Document")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(280.0);

                ui.horizontal(|ui| {
                    ui.label("Page Size:");
                    egui::ComboBox::from_id_salt("page_size")
                        .selected_text(&self.new_doc_size)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.new_doc_size,
                                "letter".to_string(),
                                "Letter",
                            );
                            ui.selectable_value(&mut self.new_doc_size, "a4".to_string(), "A4");
                            ui.selectable_value(
                                &mut self.new_doc_size,
                                "legal".to_string(),
                                "Legal",
                            );
                            ui.selectable_value(
                                &mut self.new_doc_size,
                                "tabloid".to_string(),
                                "Tabloid",
                            );
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Orientation:");
                    ui.selectable_value(
                        &mut self.new_doc_orientation,
                        PageOrientation::Portrait,
                        "Portrait",
                    );
                    ui.selectable_value(
                        &mut self.new_doc_orientation,
                        PageOrientation::Landscape,
                        "Landscape",
                    );
                });

                ui.add_space(16.0);
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        self.show_new_dialog = false;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Create").clicked() {
                            let size = match self.new_doc_size.as_str() {
                                "letter" => PageSize::Letter,
                                "a4" => PageSize::A4,
                                "legal" => PageSize::Legal,
                                "tabloid" => PageSize::Tabloid,
                                _ => PageSize::Letter,
                            };
                            self.new_document(size, self.new_doc_orientation);
                            self.show_new_dialog = false;
                        }
                    });
                });
            });
    }

    fn show_about_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_about_dialog {
            return;
        }

        egui::Window::new("About Repub")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Repub Web");
                    ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                    ui.add_space(8.0);
                    ui.label("Desktop publishing in your browser");
                    ui.add_space(16.0);
                    ui.hyperlink_to("GitHub", "https://github.com/lukem570/repub");
                    ui.add_space(16.0);
                    if ui.button("Close").clicked() {
                        self.show_about_dialog = false;
                    }
                });
            });
    }
}

impl eframe::App for WebApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check for pending file opens
        self.process_pending_file();

        // Update page title
        platform::set_title(&self.window_title());

        // Menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.show_menu_bar(ui);
        });

        // Toolbar
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            self.show_toolbar(ui);
        });

        // Status bar
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(ref doc) = self.document {
                    ui.label(format!(
                        "Page {} of {}",
                        self.current_page + 1,
                        doc.pages().len()
                    ));
                    ui.separator();
                }
                ui.label(format!("Zoom: {:.0}%", self.zoom * 100.0));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.dirty {
                        ui.label("● Unsaved changes");
                    } else {
                        ui.label("Ready");
                    }
                });
            });
        });

        // Main canvas
        egui::CentralPanel::default().show(ctx, |ui| {
            self.show_canvas(ui);
        });

        // Dialogs
        self.show_new_dialog(ctx);
        self.show_about_dialog(ctx);

        // Handle keyboard shortcuts
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::N) {
                self.show_new_dialog = true;
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::S) {
                self.save_document();
            }
            if i.key_pressed(egui::Key::Escape) {
                self.selection.clear();
                self.draw_state.is_drawing = false;
            }
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Save settings
        if let Ok(json) = serde_json::to_string(&self.settings) {
            storage.set_string("settings", json);
        }
    }
}
