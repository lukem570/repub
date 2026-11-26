//! Document canvas for rendering and interaction.

use egui::{Pos2, Rect, Rounding, Sense, Stroke, TextureHandle, Vec2};
use repub_model::shape::{Shape, ShapeKind};
use repub_geometry::Point;

use crate::app::AppAction;
use crate::background;
use crate::context_menu;
use crate::snapping::{SnapAxis, SnapConfig, SnapEngine, SnapResult};
use crate::state::{AppState, Guide, GuideOrientation};
use crate::text_edit;
use crate::theme::{colors, metrics};
use crate::tools::{DrawState, HandleType, ResizeHandle, SelectState, Tool};

/// State for guide dragging/creation.
#[derive(Debug, Default)]
pub struct GuideState {
    /// Guide being created from ruler (None means not creating).
    pub creating_guide: Option<GuideOrientation>,
    /// Preview position for guide being created (in document coordinates).
    pub preview_position: Option<f64>,
    /// Index of guide being dragged (None means not dragging).
    pub dragging_guide: Option<usize>,
    /// Index of hovered guide (for cursor changes).
    pub hovered_guide: Option<usize>,
}

/// Document canvas.
pub struct Canvas {
    /// Drawing state
    pub draw_state: DrawState,
    /// Selection state
    pub select_state: SelectState,
    /// Snap configuration
    pub snap_config: SnapConfig,
    /// Active snap guides (for visualization)
    pub snap_guides: Vec<SnapResult>,
    /// Cached background texture handle
    background_texture: Option<TextureHandle>,
    /// Guide interaction state
    pub guide_state: GuideState,
    /// Pending context menu request (macOS native menus)
    #[cfg(target_os = "macos")]
    pub pending_context_menu: Option<(crate::native_menu::ContextMenuType, f64, f64)>,
    /// A text selection drag is in progress in the edited text frame.
    text_selecting: bool,
    /// Where the page was drawn last frame (screen points), for tests and tools.
    pub last_page_rect: Option<Rect>,
    /// Bounds of the selected objects when the current move/resize began. Each
    /// frame is computed from these, so movement doesn't compound.
    gesture_origins: Vec<(repub_model::ObjectId, repub_geometry::Rect)>,
    /// Rotation (degrees) of each object in `gesture_origins` when the gesture began.
    gesture_rotations: Vec<f64>,
    /// Groups as they were when the gesture began (a group is rotated by turning its members)
    gesture_groups: Vec<Shape>,
    /// Points placed so far with the Freeform tool, or drawn with the Pencil
    pen_points: Vec<Point>,
    /// Adjust handle being dragged (on the single selected preset shape)
    adjusting: Option<usize>,
    /// Loaded image textures, keyed by file path. `None` marks a failed load.
    image_textures: std::collections::HashMap<String, Option<ImageMips>>,
    /// Linked text boxes of the page being drawn, as laid out along their chains.
    chain_frames: std::collections::HashMap<repub_model::ObjectId, crate::text_chain::ChainFrame>,
    /// Picture fills' images by (source hash, wrapping), with their pixel size.
    #[allow(clippy::type_complexity)]
    fill_textures: std::collections::HashMap<(u64, bool), Option<(TextureHandle, [usize; 2])>>,
}

impl Default for Canvas {
    fn default() -> Self {
        Self::new()
    }
}

impl Canvas {
    /// Create a new canvas.
    pub fn new() -> Self {
        Self {
            draw_state: DrawState::new(),
            select_state: SelectState::new(),
            snap_config: SnapConfig::default(),
            adjusting: None,
            snap_guides: Vec::new(),
            background_texture: None,
            guide_state: GuideState::default(),
            #[cfg(target_os = "macos")]
            pending_context_menu: None,
            text_selecting: false,
            last_page_rect: None,
            gesture_origins: Vec::new(),
            gesture_rotations: Vec::new(),
            gesture_groups: Vec::new(),
            pen_points: Vec::new(),
            image_textures: std::collections::HashMap::new(),
            fill_textures: std::collections::HashMap::new(),
            chain_frames: std::collections::HashMap::new(),
        }
    }

    /// Sync snap config from app state.
    fn sync_snap_config(&mut self, state: &AppState) {
        self.snap_config.grid = state.snap_to_grid;
        self.snap_config.grid_spacing = state.grid_spacing;
        self.snap_config.guides = state.snap_to_guides;
        self.snap_config.objects = state.snap_to_objects;
    }

    /// Create a snap engine for the current page.
    fn create_snap_engine(&self, state: &AppState) -> Option<SnapEngine> {
        let page = state.current_page()?;
        
        let page_bounds = repub_geometry::Rect::new(
            0.0,
            0.0,
            page.size.width,
            page.size.height,
        );
        
        let mut engine = SnapEngine::new(self.snap_config.clone(), page_bounds);
        
        // Add guides from state
        engine.set_h_guides(state.horizontal_guides());
        engine.set_v_guides(state.vertical_guides());
        
        // Add other object bounds for object snapping (exclude selected objects)
        let mut object_bounds = Vec::new();
        for layer in page.layers() {
            if !layer.visible {
                continue;
            }
            for obj in layer.shapes() {
                if !state.is_selected(obj.id()) {
                    object_bounds.push(obj.bounds());
                }
            }
        }
        engine.set_object_bounds(object_bounds);
        
        Some(engine)
    }

    /// Show the canvas and return any triggered action.
    pub fn show(&mut self, ui: &mut egui::Ui, state: &mut AppState) -> Option<AppAction> {
        crate::pattern_fill::set_context(ui.ctx());
        // Sync snap config from state
        self.sync_snap_config(state);
        
        let triggered_action: Option<AppAction> = None;
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            Sense::click_and_drag(),
        );

        let rect = response.rect;

        // Draw workspace background with grainy texture that moves with panning
        self.draw_workspace_background(ui.ctx(), &painter, rect, state);

        // In a two-page spread, pressing on the other page makes it the current page
        // before any tool sees the press, so the press acts on that page
        let layout = PageLayout::compute(rect, state);
        self.activate_pressed_page(ui.ctx(), &response, &layout, state);
        let layout = PageLayout::compute(rect, state);
        self.last_page_rect = layout.rect_of(state.current_page);

        // Pointer position in document points of the current page, for the status bar
        state.pointer_doc = response
            .hover_pos()
            .zip(self.last_page_rect)
            .map(|(p, page_rect)| self.screen_to_doc_point(page_rect, p, state));

        if let (Some(doc), Some(current_rect)) = (state.document.as_ref(), self.last_page_rect) {
            let page_count = doc.pages().len() as u32;
            let pages: Vec<_> = layout
                .pages
                .iter()
                .filter_map(|&(i, r)| state.page_on_view(i).map(|p| (i, r, p)))
                .collect();

            // One shadow for the whole spread, so it never falls across the spine
            if let Some(bounds) = layout.bounds() {
                painter.add(egui::Shape::Rect(egui::epaint::RectShape::filled(
                    bounds.translate(Vec2::new(2.0, 2.0)),
                    Rounding::ZERO,
                    colors::page_shadow(),
                ).with_blur_width(6.0)));
            }
            for &(_, page_rect, page) in &pages {
                update_text_wrap(page);
                self.draw_page_background(&painter, page_rect, page, state);
            }
            let file_name = state.document_path.as_ref().and_then(|p| p.file_stem()).map_or_else(|| "Untitled".to_string(), |s| s.to_string_lossy().into_owned());
            crate::sdf_text::set_file_name(&file_name);
            for &(i, page_rect, page) in &pages {
                crate::sdf_text::set_page_context(i as u32 + 1, page_count.max(1));
                // The master page's objects sit under the page's own (not selectable here)
                let master = page.master_page.and_then(|id| doc.master_page(id));
                self.draw_master_objects(&painter, page_rect.min, state.zoom, master);
                self.draw_objects(ui, &painter, page_rect, page, state);
            }
            // Selection and tool overlays last, so the facing page never covers them
            if let Some(&(i, _, _)) = pages.iter().find(|(i, _, _)| *i == state.current_page) {
                crate::sdf_text::set_page_context(i as u32 + 1, page_count.max(1));
                self.draw_page_overlay(&painter, rect, current_rect, state);
            }

            self.handle_input(&response, rect, current_rect, state);
        }
        // With no document the app shows the start screen instead of the canvas

        // Right-clicking a table cell puts the caret there (as in Publisher), so
        // the menu's Insert/Delete act at that cell
        if response.secondary_clicked() {
            if let Some(doc_pos) = state.pointer_doc {
                let table = self
                    .find_object_at(doc_pos, state)
                    .filter(|id| matches!(state.current_page().and_then(|p| p.get_shape(*id)).map(|s| &s.kind), Some(ShapeKind::Table(_))));
                if let Some(id) = table {
                    let now = response.ctx.input(|i| i.time);
                    text_edit::begin_cell_at_point(state, id, doc_pos.x as f32, doc_pos.y as f32, now);
                }
            }
        }
        // Right-click selects what's under the pointer first (as in Publisher), so the
        // menu acts on that object; right-clicking empty space clears the selection.
        if response.secondary_clicked() && state.text_edit.is_none() {
            if let Some(doc_pos) = state.pointer_doc {
                match self.find_object_at(doc_pos, state) {
                    Some(id) if !state.is_selected(id) => state.select(id, false),
                    Some(_) => {}
                    None => state.clear_selection(),
                }
            }
        }

        // Context menu (right-click)
        // On macOS, we use native context menus via muda
        // On other platforms, we use egui's built-in context menu
        #[cfg(not(target_os = "macos"))]
        response.context_menu(|ui| {
            let action = if state.selection.is_empty() {
                context_menu::canvas_context_menu(ui, state)
            } else {
                context_menu::object_context_menu(ui, state)
            };
            if action.is_some() {
                triggered_action = action;
            }
        });
        
        // On macOS, detect right-click and return a request for native context menu
        #[cfg(target_os = "macos")]
        if response.secondary_clicked() {
            if let Some(pos) = response.interact_pointer_pos() {
                // pos is already in window/view coordinates (egui screen space = window content area)
                let menu_type = if state.selection.is_empty() {
                    crate::native_menu::ContextMenuType::Canvas
                } else {
                    crate::native_menu::ContextMenuType::Object
                };
                // Store the context menu request - pos is in egui coordinates (top-left origin)
                self.pending_context_menu = Some((menu_type, pos.x as f64, pos.y as f64));
            }
        }

        // Set cursor based on context
        let cursor = if response.hovered() {
            // Get current mouse position in document coordinates for locked object check
            let mouse_pos = response.hover_pos();
            let doc_pos = mouse_pos.map(|pos| self.screen_to_doc_point(self.last_page_rect.unwrap_or(rect), pos, state));
            
            // Check modifier keys for copy cursor
            let alt_held = ui.ctx().input(|i| i.modifiers.alt);
            
            if state.link_source.is_some() {
                egui::CursorIcon::Alias
            } else if state.current_tool == Tool::Select {
                // Check for guide hover cursor first
                if self.guide_state.hovered_guide.is_some() || self.guide_state.dragging_guide.is_some() {
                    // Guide cursor: use vertical/horizontal resize based on guide orientation
                    if let Some(guide_idx) = self.guide_state.hovered_guide.or(self.guide_state.dragging_guide) {
                        if let Some(guide) = state.guides().get(guide_idx) {
                            match guide.orientation {
                                GuideOrientation::Horizontal => egui::CursorIcon::ResizeVertical,
                                GuideOrientation::Vertical => egui::CursorIcon::ResizeHorizontal,
                            }
                        } else {
                            egui::CursorIcon::Default
                        }
                    } else {
                        egui::CursorIcon::Default
                    }
                } else if let Some(handle_type) = &self.select_state.hovered_handle {
                    // Check for handle hover cursor
                    match handle_type {
                        HandleType::Resize(handle) => handle.cursor_rotated(self.selection_frame(state).map_or(0.0, |f| f.rotation)),
                        HandleType::Rotation => egui::CursorIcon::Alias, // Best available for rotation
                        HandleType::Adjust(_) => egui::CursorIcon::PointingHand,
                        HandleType::Body => {
                            if self.select_state.is_dragging {
                                // Show Copy cursor when Alt is held while dragging
                                if alt_held {
                                    egui::CursorIcon::Copy
                                } else {
                                    egui::CursorIcon::Grabbing
                                }
                            } else {
                                egui::CursorIcon::Move
                            }
                        }
                    }
                } else {
                    // Check if hovering over a locked object
                    if let Some(pos) = doc_pos {
                        if let Some(obj_id) = self.find_object_at(pos, state) {
                            if self.is_object_locked(obj_id, state) {
                                egui::CursorIcon::NotAllowed
                            } else {
                                egui::CursorIcon::Default
                            }
                        } else {
                            egui::CursorIcon::Default
                        }
                    } else {
                        egui::CursorIcon::Default
                    }
                }
            } else if state.current_tool == Tool::Pan {
                // Show Grabbing when actively panning, Grab when idle
                if response.dragged() {
                    egui::CursorIcon::Grabbing
                } else {
                    egui::CursorIcon::Grab
                }
            } else {
                state.current_tool.cursor()
            }
        } else {
            egui::CursorIcon::Default
        };
        // I-beam over the text frame being edited
        let over_edited_text = state.pointer_doc.is_some_and(|p| text_edit::edit_contains(state, p.x as f32, p.y as f32));
        // Over a hyperlink: say where it goes; with Cmd/Ctrl held it can be followed
        let link = if response.hovered() { state.pointer_doc.and_then(|p| self.link_under(p, state)) } else { None };
        let cursor = match &link {
            Some(_) if ui.input(|i| i.modifiers.command) => egui::CursorIcon::PointingHand,
            _ if over_edited_text => egui::CursorIcon::Text,
            _ => cursor,
        };
        ui.ctx().set_cursor_icon(cursor);
        if let Some(link) = link {
            let key = if cfg!(target_os = "macos") { "Cmd" } else { "Ctrl" };
            egui::show_tooltip_at_pointer(ui.ctx(), ui.layer_id(), egui::Id::new("canvas_link_tip"), |ui| {
                ui.label(link.target.clone());
                ui.label(egui::RichText::new(format!("{key}+click to follow link")).small().color(colors::text_secondary()));
            });
        }
        
        triggered_action
    }

    /// Draw the workspace background with grainy texture.
    /// The texture scrolls with panning to create a subtle parallax effect.
    fn draw_workspace_background(
        &mut self,
        ctx: &egui::Context,
        painter: &egui::Painter,
        rect: Rect,
        state: &AppState,
    ) {
        // Get or create the texture
        let texture = if let Some(ref tex) = self.background_texture {
            tex.clone()
        } else {
            let tex = background::get_background_texture(ctx);
            self.background_texture = Some(tex.clone());
            tex
        };

        // Paint the textured background with pan offset for parallax effect
        let pan = egui::vec2(state.pan_offset.x as f32, state.pan_offset.y as f32);
        background::paint_workspace_with_pan(painter, rect, &texture, pan);
    }

    /// Draw the page's margin guides as a dotted rectangle inset from the page edge.
    fn draw_margin_guides(
        &self,
        painter: &egui::Painter,
        page_rect: Rect,
        page: &repub_model::Page,
        state: &AppState,
    ) {
        let content = page.content_rect();
        if content.size.width <= 0.0 || content.size.height <= 0.0 {
            return;
        }
        let r = Rect::from_min_size(
            page_rect.min + Vec2::new(content.origin.x as f32, content.origin.y as f32) * state.zoom,
            Vec2::new(content.size.width as f32, content.size.height as f32) * state.zoom,
        );
        let corners = [r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom(), r.left_top()];
        painter.extend(egui::Shape::dashed_line(
            &corners,
            Stroke::new(1.0, colors::margin_guide()),
            2.0,
            2.0,
        ));
    }

    /// Make the page under a fresh primary/secondary press the current page.
    ///
    /// Only applies in a two-page spread when the press lands on the facing page
    /// and not on something belonging to the current page (a selection handle or
    /// an object that overhangs the spine).
    fn activate_pressed_page(&mut self, ctx: &egui::Context, response: &egui::Response, layout: &PageLayout, state: &mut AppState) {
        if layout.pages.len() < 2 || !response.hovered() || state.current_tool == Tool::Pan {
            return;
        }
        if self.select_state.is_busy() || self.draw_state.is_drawing || self.guide_state.dragging_guide.is_some() {
            return;
        }
        let press = ctx.input(|i| {
            let pressed = i.pointer.button_pressed(egui::PointerButton::Primary)
                || i.pointer.button_pressed(egui::PointerButton::Secondary);
            if pressed { i.pointer.press_origin() } else { None }
        });
        let Some(pos) = press else { return };
        let Some(target) = layout.page_at(pos) else { return };
        if target == state.current_page {
            return;
        }
        if let Some(current_rect) = layout.rect_of(state.current_page) {
            let doc_pos = self.screen_to_doc_point(current_rect, pos, state);
            if self.hit_test_handles(doc_pos, current_rect, state).is_some() || self.find_object_at(doc_pos, state).is_some() {
                return;
            }
        }
        text_edit::end(state);
        state.edit_points = None;
        self.select_state.hovered_handle = None;
        self.guide_state.hovered_guide = None;
        state.set_page(target);
    }

    /// Draw a page's paper, border, grid and margin guides.
    fn draw_page_background(
        &mut self,
        painter: &egui::Painter,
        page_rect: Rect,
        page: &repub_model::Page,
        state: &AppState,
    ) {
        // Page background (custom fill when set, else default white)
        self.paint_page_background(painter, page_rect, page);
        painter.rect_stroke(page_rect, Rounding::ZERO, Stroke::new(1.0, colors::page_border()));

        // Draw grid (behind objects but on page)
        if state.show_grid {
            self.draw_grid(painter, page_rect, page, state);
        }

        // Margin guides (non-printing), behind objects
        if state.show_guides {
            self.draw_margin_guides(painter, page_rect, page, state);
        }
    }

    /// Draw the current page's interactive overlays: guides, selection, previews.
    fn draw_page_overlay(&self, painter: &egui::Painter, canvas_rect: Rect, page_rect: Rect, state: &AppState) {
        // Draw guides (after objects, before selection handles)
        if state.show_guides {
            self.draw_guides(painter, canvas_rect, page_rect, state);
        }

        // Draw selection handles
        self.draw_selection(painter, page_rect, state);

        // Shape point-editing overlay
        crate::edit_points::draw(painter, page_rect, state);

        // Draw drawing preview
        if state.current_tool.is_creation_tool() && self.draw_state.is_drawing {
            self.draw_preview(painter, page_rect, state);
        }
        if matches!(state.current_tool, Tool::Freeform | Tool::Pencil) {
            self.draw_pen_preview(painter, page_rect, state);
        }

        // Draw marquee selection
        if self.select_state.is_marquee {
            self.draw_marquee(painter, page_rect, state);
        }

        // Draw snap indicators (only during drag/draw operations)
        if !self.snap_guides.is_empty() && (self.select_state.is_dragging || self.draw_state.is_drawing) {
            self.draw_snap_indicators(painter, canvas_rect, page_rect, state);
        }
    }

    /// Draw the grid within page bounds.
    fn draw_grid(
        &self,
        painter: &egui::Painter,
        page_rect: Rect,
        page: &repub_model::Page,
        state: &AppState,
    ) {
        let grid_spacing = state.grid_spacing;
        let major_spacing = 72.0; // 1 inch = 72 points
        
        let page_width = page.size.width;
        let page_height = page.size.height;
        
        // Minor grid lines (at grid_spacing intervals)
        let minor_stroke = Stroke::new(1.0, colors::grid_minor());
        // Major grid lines (every inch)
        let major_stroke = Stroke::new(1.0, colors::grid_major());
        
        // Create a clipped painter
        let clipped_painter = painter.with_clip_rect(page_rect);
        
        // Vertical lines
        let mut x = 0.0;
        while x <= page_width {
            let screen_x = page_rect.left() + (x as f32) * state.zoom;
            let is_major = (x % major_spacing).abs() < 0.01;
            let stroke = if is_major { major_stroke } else { minor_stroke };
            
            clipped_painter.line_segment(
                [Pos2::new(screen_x, page_rect.top()), Pos2::new(screen_x, page_rect.bottom())],
                stroke,
            );
            x += grid_spacing;
        }
        
        // Horizontal lines
        let mut y = 0.0;
        while y <= page_height {
            let screen_y = page_rect.top() + (y as f32) * state.zoom;
            let is_major = (y % major_spacing).abs() < 0.01;
            let stroke = if is_major { major_stroke } else { minor_stroke };
            
            clipped_painter.line_segment(
                [Pos2::new(page_rect.left(), screen_y), Pos2::new(page_rect.right(), screen_y)],
                stroke,
            );
            y += grid_spacing;
        }
    }

    /// Draw guides on the canvas.
    fn draw_guides(
        &self,
        painter: &egui::Painter,
        canvas_rect: Rect,
        page_rect: Rect,
        state: &AppState,
    ) {
        // Draw guides as dashed cyan lines extending across the full canvas
        let guide_color = colors::guide();
        let guide_hover_color = colors::guide_hover();
        
        for (index, guide) in state.guides().iter().enumerate() {
            let is_hovered = self.guide_state.hovered_guide == Some(index);
            let color = if is_hovered { guide_hover_color } else { guide_color };
            
            match guide.orientation {
                GuideOrientation::Horizontal => {
                    let screen_y = page_rect.top() + (guide.position as f32) * state.zoom;
                    self.draw_dashed_line(
                        painter,
                        Pos2::new(canvas_rect.left(), screen_y),
                        Pos2::new(canvas_rect.right(), screen_y),
                        color,
                    );
                }
                GuideOrientation::Vertical => {
                    let screen_x = page_rect.left() + (guide.position as f32) * state.zoom;
                    self.draw_dashed_line(
                        painter,
                        Pos2::new(screen_x, canvas_rect.top()),
                        Pos2::new(screen_x, canvas_rect.bottom()),
                        color,
                    );
                }
            }
        }
        
        // Draw guide creation preview
        if let (Some(orientation), Some(position)) = (
            self.guide_state.creating_guide,
            self.guide_state.preview_position,
        ) {
            match orientation {
                GuideOrientation::Horizontal => {
                    let screen_y = page_rect.top() + (position as f32) * state.zoom;
                    self.draw_dashed_line(
                        painter,
                        Pos2::new(canvas_rect.left(), screen_y),
                        Pos2::new(canvas_rect.right(), screen_y),
                        colors::guide_hover(),
                    );
                }
                GuideOrientation::Vertical => {
                    let screen_x = page_rect.left() + (position as f32) * state.zoom;
                    self.draw_dashed_line(
                        painter,
                        Pos2::new(screen_x, canvas_rect.top()),
                        Pos2::new(screen_x, canvas_rect.bottom()),
                        colors::guide_hover(),
                    );
                }
            }
        }
    }

    /// Draw a dashed line.
    fn draw_dashed_line(
        &self,
        painter: &egui::Painter,
        start: Pos2,
        end: Pos2,
        color: egui::Color32,
    ) {
        let dash_length = 5.0;
        let gap_length = 5.0;
        let stroke = Stroke::new(1.0, color);
        
        let direction = end - start;
        let total_length = direction.length();
        let direction = direction / total_length;
        
        let mut current = 0.0;
        let mut drawing = true;
        
        while current < total_length {
            if drawing {
                let segment_end = (current + dash_length).min(total_length);
                let start_pos = start + direction * current;
                let end_pos = start + direction * segment_end;
                painter.line_segment([start_pos, end_pos], stroke);
                current = segment_end;
            } else {
                current += gap_length;
            }
            drawing = !drawing;
        }
    }

    /// Draw snap indicator lines during drag/resize operations.
    /// Shows thin magenta dashed lines extending across the page at snap positions.
    fn draw_snap_indicators(
        &self,
        painter: &egui::Painter,
        canvas_rect: Rect,
        page_rect: Rect,
        state: &AppState,
    ) {
        let snap_color = colors::snap_indicator();
        
        for snap in &self.snap_guides {
            match snap.axis {
                SnapAxis::Horizontal => {
                    // Horizontal snap affects X position, so draw a vertical line
                    let screen_x = page_rect.left() + (snap.guide_pos as f32) * state.zoom;
                    self.draw_dashed_line(
                        painter,
                        Pos2::new(screen_x, canvas_rect.top()),
                        Pos2::new(screen_x, canvas_rect.bottom()),
                        snap_color,
                    );
                }
                SnapAxis::Vertical => {
                    // Vertical snap affects Y position, so draw a horizontal line
                    let screen_y = page_rect.top() + (snap.guide_pos as f32) * state.zoom;
                    self.draw_dashed_line(
                        painter,
                        Pos2::new(canvas_rect.left(), screen_y),
                        Pos2::new(canvas_rect.right(), screen_y),
                        snap_color,
                    );
                }
            }
        }
    }

    /// Load (and cache) an image file as textures.
    fn image_texture(&mut self, ctx: &egui::Context, path: &str) -> Option<ImageMips> {
        if let Some(cached) = self.image_textures.get(path) {
            return cached.clone();
        }
        let loaded = image::open(path)
            .ok()
            .map(|img| ImageMips::upload(ctx, &format!("image:{path}"), img.to_rgba8()));
        self.image_textures.insert(path.to_string(), loaded.clone());
        loaded
    }

    /// The hyperlink drawn at `doc_pos`: in a text box (as shown, for linked
    /// boxes) or a table cell.
    fn link_under(&self, doc_pos: Point, state: &AppState) -> Option<repub_model::text::Hyperlink> {
        let id = self.find_object_at(doc_pos, state)?;
        let (x, y) = (doc_pos.x as f32, doc_pos.y as f32);
        let shape = state.current_page()?.get_shape(id)?;
        match &shape.kind {
            ShapeKind::TextFrame(_) => {
                // The box itself, or (for a linked box) the part of its story shown here
                match self.chain_frames.get(&id) {
                    Some(chain) => crate::hyperlinks::link_at(&chain.frame, x, y),
                    None => crate::hyperlinks::link_at(shape, x, y),
                }
            }
            ShapeKind::Table(_) => {
                let (row, col) = crate::table_view::cell_at(shape, doc_pos)?;
                let frame = crate::table_view::cell_frame(shape, row, col)?;
                crate::hyperlinks::link_at(&frame, x, y)
            }
            _ => None,
        }
    }

    /// Complete Create Link with a click at `doc_pos`: an empty text box there
    /// becomes the next box of `from`'s story; empty page space gets a new box
    /// the size of `from`. Anything else is explained in an alert.
    fn finish_link(&self, from: repub_model::ObjectId, doc_pos: Point, state: &mut AppState) {
        let target = self.find_object_at(doc_pos, state);
        state.begin_edit();
        let mut new_box = None;
        let linked = match state.current_page_mut() {
            Some(page) => match target {
                Some(to) => crate::text_chain::link(page, from, to),
                None => match page.get_shape(from).map(|s| (s.common.bounds.size, &s.kind)) {
                    Some((size, ShapeKind::TextFrame(src))) => {
                        let (margins, inset) = (src.margins, src.inset);
                        let mut data = repub_model::shape::TextFrameData::new();
                        data.margins = margins;
                        data.inset = inset;
                        let shape = repub_model::shape::Shape::new(
                            repub_geometry::Rect::new(doc_pos.x, doc_pos.y, size.width, size.height),
                            ShapeKind::TextFrame(data),
                        );
                        let id = shape.id();
                        page.add_shape(shape);
                        new_box = Some(id);
                        crate::text_chain::link(page, from, id)
                    }
                    _ => false,
                },
            },
            None => false,
        };
        state.end_edit("Create Text Box Link");
        if linked {
            state.selection = vec![new_box.or(target).unwrap_or(from)];
        } else if target.is_some() {
            state.alert = Some((
                "Can't Link to This Object".to_owned(),
                "Text can only continue in an empty text box that isn't already linked to another one.".to_owned(),
            ));
        }
    }

    /// Load (and cache) a picture fill's image: a `data:` URI or a file path.
    /// `repeat` makes a texture that wraps, for tiled fills.
    fn fill_texture(&mut self, ctx: &egui::Context, source: &str, repeat: bool) -> Option<(TextureHandle, [usize; 2])> {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hasher);
        let key = (hasher.finish(), repeat);
        if let Some(cached) = self.fill_textures.get(&key) {
            return cached.clone();
        }
        let image = if let Some(rest) = source.strip_prefix("data:") {
            use base64::Engine;
            rest.split_once(',')
                .and_then(|(_, data)| base64::engine::general_purpose::STANDARD.decode(data.trim()).ok())
                .and_then(|bytes| image::load_from_memory(&bytes).ok())
        } else {
            image::open(source).ok()
        };
        if image.is_none() {
            log::warn!("Couldn't load a picture fill ({} bytes of source)", source.len());
        }
        let loaded = image.map(|img| {
            // Large pictures are scaled down: fills don't need more detail than this
            let img = if img.width().max(img.height()) > 2048 { img.resize(2048, 2048, image::imageops::FilterType::Triangle) } else { img };
            let rgba = img.to_rgba8();
            let size = [rgba.width() as usize, rgba.height() as usize];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
            let options = if repeat { egui::TextureOptions::LINEAR_REPEAT } else { egui::TextureOptions::LINEAR };
            (ctx.load_texture(format!("fill:{:x}", key.0), color_image, options), size)
        });
        // Failures are cached too, so a bad image isn't decoded again every frame
        self.fill_textures.insert(key, loaded.clone());
        loaded
    }

    /// The egui shapes for a vector shape, including picture fills (which need
    /// textures, so the free [`shape_object_shapes`] draws them as white).
    fn shape_shapes(&mut self, ctx: &egui::Context, shape: &repub_model::shape::Shape, map: DocToScreen, alpha: f32) -> Vec<egui::Shape> {
        use repub_model::styles::{Fill, TileMode};
        if let Fill::Texture(picture) = &shape.fill {
            let repeat = picture.tile != TileMode::Stretch && picture.tile != TileMode::Center;
            if let Some((texture, size)) = self.fill_texture(ctx, &picture.source, repeat) {
                let mut outline = shape.clone();
                outline.line = None;
                outline.fill = Fill::Solid(repub_color::Color::WHITE);
                let fills = shape_object_shapes(&outline, map, 1.0, 0.0);
                let mut out = vec![crate::pattern_fill::texture_fill_shapes(
                    fills, picture, texture.id(), size, shape.common.bounds, shape.common.rotation, map, alpha,
                )];
                let mut strokes = shape.clone();
                strokes.fill = Fill::None;
                out.extend(shape_object_shapes(&strokes, map, alpha, 0.0));
                return out;
            }
        }
        shape_object_shapes(shape, map, alpha, 0.0)
    }

    /// Paint a page's paper: its background fill (any kind), else white.
    fn paint_page_background(&mut self, painter: &egui::Painter, page_rect: Rect, page: &repub_model::Page) {
        painter.rect_filled(page_rect, Rounding::ZERO, colors::page());
        let Some(fill) = page.background.as_ref().filter(|f| !f.is_none()) else { return };
        let map = DocToScreen { origin: page_rect.min, scale: page_rect.width() / page.size.width.max(1.0) as f32 };
        let mut paper = repub_model::shape::Shape::rectangle(repub_geometry::Rect::new(0.0, 0.0, page.size.width, page.size.height));
        paper.fill = fill.clone();
        paper.line = None;
        painter.extend(self.shape_shapes(painter.ctx(), &paper, map, 1.0));
    }

    /// Load (and cache) an embedded image (base64 data) as textures.
    fn embedded_image_texture(
        &mut self,
        ctx: &egui::Context,
        key: &str,
        data: &str,
        _mime_type: &str,
    ) -> Option<ImageMips> {
        // Use key for caching
        if let Some(cached) = self.image_textures.get(key) {
            return cached.clone();
        }

        // Decode base64
        use base64::Engine;
        let loaded = match base64::engine::general_purpose::STANDARD.decode(data) {
            Ok(decoded) => match image::load_from_memory(&decoded) {
                Ok(img) => Some(ImageMips::upload(ctx, &format!("embedded:{key}"), img.to_rgba8())),
                Err(e) => {
                    log::warn!("Failed to load embedded image {}: {}", key, e);
                    None
                }
            },
            Err(e) => {
                log::warn!("Failed to decode base64 image {}: {}", key, e);
                None
            }
        };

        // Failures are cached too, so a bad image isn't decoded again every frame
        self.image_textures.insert(key.to_string(), loaded.clone());
        loaded
    }

    fn draw_objects(
        &mut self,
        _ui: &egui::Ui,
        painter: &egui::Painter,
        page_rect: Rect,
        page: &repub_model::Page,
        state: &AppState,
    ) {
        self.chain_frames = crate::text_chain::layout_chains(page);
        for layer in page.layers() {
            if !layer.visible {
                continue;
            }

            for obj in layer.shapes() {
                self.draw_object(painter, page_rect.min, state.zoom, obj, Some(state));
            }
        }
    }

    /// Draw a page as it prints (background and objects, no non-printing marks)
    /// with its top-left at `page_rect.min`, scaled to `page_rect`'s width.
    /// Used for the page thumbnails, so they match the canvas exactly.
    pub fn draw_page_thumbnail(
        &mut self,
        painter: &egui::Painter,
        page_rect: Rect,
        page: &repub_model::Page,
        master: Option<&repub_model::MasterPage>,
        page_number: u32,
        page_count: u32,
    ) {
        let zoom = page_rect.width() / page.size.width.max(1.0) as f32;
        self.paint_page_background(painter, page_rect, page);
        update_text_wrap(page);
        crate::sdf_text::set_page_context(page_number.max(1), page_count.max(1));
        self.draw_master_objects(painter, page_rect.min, zoom, master);
        self.chain_frames = crate::text_chain::layout_chains(page);
        for layer in page.layers() {
            if !layer.visible {
                continue;
            }
            for obj in layer.shapes() {
                self.draw_object(painter, page_rect.min, zoom, obj, None);
            }
        }
    }

    /// Draw a master page's objects (printable content only; they are edited on
    /// the master, not on the pages that use it).
    fn draw_master_objects(&mut self, painter: &egui::Painter, origin: Pos2, zoom: f32, master: Option<&repub_model::MasterPage>) {
        let Some(master) = master else { return };
        for layer in master.layers.iter().filter(|l| l.visible) {
            for obj in layer.shapes() {
                self.draw_object(painter, origin, zoom, obj, None);
            }
        }
    }

    /// Draw one object with the page's top-left at `origin` and `zoom` screen
    /// pixels per point. `editor` carries the app state when drawing the live
    /// canvas (non-printing frame edges, text selection and caret); thumbnails
    /// pass `None` and get only printable content.
    fn draw_object(
        &mut self,
        painter: &egui::Painter,
        origin: Pos2,
        zoom: f32,
        obj: &Shape,
        editor: Option<&AppState>,
    ) {
        // Hidden in the Selection pane
        if !obj.common.visible {
            return;
        }
        let map = DocToScreen { origin, scale: zoom };
        let bounds = obj.bounds();
        let screen_rect = map.rect(&bounds);
        let is_selected = |id| editor.is_some_and(|s| s.is_selected(id));

        painter.extend(shadow_shapes(obj, map));

        match &obj.kind {
            ShapeKind::AutoShape(_) => {
                painter.extend(self.shape_shapes(painter.ctx(), obj, map, obj.opacity as f32));
            }
            ShapeKind::TextFrame(tf_data) => {
                // The box as drawn: its own shape, or (for a linked box) the part
                // of its chain's story shown here. Cloned so the immutable borrow of
                // `self.chain_frames` doesn't clash with `self.shape_shapes` below.
                let shown = self.chain_frames.get(&obj.common.id).map(|c| c.frame.clone());
                let tf: &Shape = shown.as_ref().unwrap_or(obj);
                // The text payload shown in this box
                let tf_shown = match &tf.kind {
                    ShapeKind::TextFrame(d) => d,
                    _ => tf_data,
                };
                // Text boxes are transparent unless they have a fill (any kind,
                // turned with the box). Also draw stroke/border if present.
                if !obj.fill.is_none() || obj.line.is_some() {
                    let mut back = repub_model::shape::Shape::rectangle(tf.common.bounds);
                    back.common.rotation = tf.common.rotation;
                    back.fill = obj.fill.clone();
                    back.line = obj.line.clone();
                    painter.extend(self.shape_shapes(painter.ctx(), &back, map, obj.opacity as f32));
                }

                // The text being edited in this box: the box's own, or (for a linked
                // box) the part of its chain's story shown here. Caret and
                // selection are mapped into the box; a caret past its text is drawn
                // in a later box instead.
                let chain = self.chain_frames.get(&obj.common.id).map(|c| (c.start, c.head, c.last));
                let edit_info = editor.and_then(|s| s.text_edit.as_ref()).and_then(|e| {
                    let (start, last) = match chain {
                        Some((start, head, last)) if e.frame == head => (start, last),
                        None if e.frame == obj.common.id => (text_edit::TextPos::new(0, 0), true),
                        _ => return None,
                    };
                    let end = text_edit::end_pos(&tf_shown.content);
                    let local = |p| crate::text_chain::from_story(start, p);
                    let caret = local(e.caret).filter(|c| *c < end || (*c == end && last));
                    let (lo, hi) = (e.caret.min(e.anchor), e.caret.max(e.anchor));
                    let selection = match (local(hi), local(lo)) {
                        (Some(hi), lo) if e.caret != e.anchor && lo.map_or(true, |lo| lo <= end) => {
                            Some((lo.unwrap_or_default(), hi.min(end)))
                        }
                        _ => None,
                    };
                    let mut edit = e.clone();
                    edit.caret = caret.unwrap_or_default();
                    edit.anchor = edit.caret;
                    Some((caret.is_some() && e.caret == e.anchor, selection, crate::sdf_text::layout_frame(tf), edit))
                });
                let to_screen = |x: f32, y: f32| {
                    let (dx, dy) = text_edit::from_frame_space(tf, x, y);
                    origin + Vec2::new(dx, dy) * zoom
                };
                if let Some((_, Some((a, b)), layout, _)) = &edit_info {
                    for [x0, y0, x1, y1] in text_edit::selection_rects(layout, *a, *b) {
                        let quad = vec![to_screen(x0, y0), to_screen(x1, y0), to_screen(x1, y1), to_screen(x0, y1)];
                        painter.add(egui::Shape::convex_polygon(quad, colors::text_selection(), Stroke::NONE));
                    }
                }

                // Text: SDF renderer (crisp at any zoom); egui text only as a fallback
                // while the SDF atlas is still building or without the wgpu backend.
                let text = tf_shown.content.plain_text();
                if !text.is_empty() && !crate::sdf_text::draw_text_frame(painter, origin, zoom, tf) {
                    let text_pos = screen_rect.left_top() + Vec2::new(4.0, 4.0) * zoom;
                    painter.with_clip_rect(screen_rect.intersect(painter.clip_rect())).text(
                        text_pos,
                        egui::Align2::LEFT_TOP,
                        text,
                        egui::FontId::proportional(12.0 * zoom),
                        egui::Color32::BLACK,
                    );
                }

                // Blinking caret over the text (solid for a moment after each edit/move)
                if let Some((true, None, layout, edit)) = &edit_info {
                    {
                        let time = painter.ctx().input(|i| i.time);
                        const PERIOD: f64 = 1.06;
                        let phase = (time - edit.blink_epoch).max(0.0) % PERIOD;
                        if phase < PERIOD / 2.0 {
                            if let Some((x, top, bottom)) = text_edit::caret_geometry(layout, edit.caret) {
                                painter.line_segment([to_screen(x, top), to_screen(x, bottom)], Stroke::new(1.5, colors::text_primary()));
                            }
                        }
                        let until_toggle = PERIOD / 2.0 - phase % (PERIOD / 2.0);
                        painter.ctx().request_repaint_after(std::time::Duration::from_secs_f64(until_toggle));
                    }
                }

                // Overflow mark on a selected box whose text doesn't all fit (and
                // doesn't continue in another box)
                let selected_or_edited = is_selected(tf.common.id) || edit_info.is_some();
                if selected_or_edited && tf_shown.linked_frame.is_none() && crate::text_chain::overflow_pos(tf).is_some() {
                    let mark = overflow_mark_rect(tf, origin, zoom);
                    painter.rect_filled(mark, Rounding::same(2.0), egui::Color32::WHITE);
                    painter.rect_stroke(mark, Rounding::same(2.0), Stroke::new(1.0, colors::selection_frame()));
                    painter.text(mark.center(), egui::Align2::CENTER_CENTER, "...", egui::FontId::proportional(11.0), colors::text_primary());
                }
            }
            ShapeKind::Picture(img) => {
                // Try to get texture from various sources
                let texture = match &img.source {
                    repub_model::shape::ImageSource::File(path) => {
                        self.image_texture(painter.ctx(), path)
                    }
                    repub_model::shape::ImageSource::Embedded { data, mime_type } => {
                        // Use object ID as cache key for embedded images
                        let key = format!("embedded_{:?}", obj.common.id);
                        self.embedded_image_texture(painter.ctx(), &key, data, mime_type)
                    }
                    _ => None,
                };
                // The frame's own fill shows behind the picture and its outline around it
                let mut frame = repub_model::shape::Shape::rectangle(obj.bounds());
                frame.common.rotation = obj.common.rotation;
                if !obj.fill.is_none() {
                    frame.fill = obj.fill.clone();
                    frame.line = None;
                    painter.extend(self.shape_shapes(painter.ctx(), &frame, map, obj.opacity as f32));
                }
                if let Some(mips) = texture {
                    let [tex_w, tex_h] = mips.levels[0].size();
                    let pieces = picture_placement(img.fit, img.crop.as_ref(), [tex_w as f32, tex_h as f32], screen_rect, 0.75 * zoom);
                    let tint = egui::Color32::from_white_alpha((obj.opacity.clamp(0.0, 1.0) * 255.0) as u8);
                    // Rotation turns the picture about the frame's centre (tiles stay upright)
                    let rotation = if img.fit == repub_model::shape::ImageFit::Tile { 0.0 } else { obj.common.rotation as f32 };
                    let (sin, cos) = rotation.to_radians().sin_cos();
                    let centre = screen_rect.center();
                    // Flips mirror the picture in its frame, then it turns
                    let mirror = Vec2::new(if obj.common.flip_h { -1.0 } else { 1.0 }, if obj.common.flip_v { -1.0 } else { 1.0 });
                    let turn = |p: Pos2| {
                        let d = (p - centre) * mirror;
                        centre + Vec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos)
                    };
                    let clipped = painter.with_clip_rect(if rotation == 0.0 { screen_rect.intersect(painter.clip_rect()) } else { painter.clip_rect() });
                    for (dest, uv) in pieces {
                        // Texture level for the on-screen size of the whole source image
                        let needed = Vec2::new(dest.width() / uv.width().max(1e-6), dest.height() / uv.height().max(1e-6));
                        let texture = mips.for_screen_size(needed * painter.ctx().pixels_per_point());
                        let mut mesh = egui::Mesh::with_texture(texture.id());
                        let corners = [
                            (dest.left_top(), uv.left_top()),
                            (dest.right_top(), uv.right_top()),
                            (dest.right_bottom(), uv.right_bottom()),
                            (dest.left_bottom(), uv.left_bottom()),
                        ];
                        for (pos, uv) in corners {
                            mesh.vertices.push(egui::epaint::Vertex { pos: turn(pos), uv, color: tint });
                        }
                        mesh.add_triangle(0, 1, 2);
                        mesh.add_triangle(0, 2, 3);
                        clipped.add(egui::Shape::mesh(mesh));
                    }
                    // Brightness and reduced contrast, approximated by a translucent wash over the frame
                    let wash = |color: egui::Color32| {
                        let quad = [screen_rect.left_top(), screen_rect.right_top(), screen_rect.right_bottom(), screen_rect.left_bottom()].map(turn).to_vec();
                        painter.add(egui::Shape::convex_polygon(quad, color, Stroke::NONE));
                    };
                    let (brightness, contrast) = (img.brightness.clamp(-1.0, 1.0) as f32, img.contrast.clamp(-1.0, 1.0) as f32);
                    if brightness != 0.0 {
                        let alpha = (brightness.abs() * 0.8 * 255.0) as u8;
                        wash(if brightness > 0.0 { egui::Color32::from_white_alpha(alpha) } else { egui::Color32::from_black_alpha(alpha) });
                    }
                    if contrast < 0.0 {
                        wash(egui::Color32::from_rgba_unmultiplied(128, 128, 128, (-contrast * 0.8 * 255.0) as u8));
                    }
                } else {
                    // Placeholder: grey box with an X
                    painter.rect_filled(screen_rect, Rounding::ZERO, egui::Color32::from_rgb(220, 220, 220));
                    painter.rect_stroke(screen_rect, Rounding::ZERO, Stroke::new(1.0, egui::Color32::GRAY));
                    painter.line_segment(
                        [screen_rect.left_top(), screen_rect.right_bottom()],
                        Stroke::new(1.0, egui::Color32::GRAY),
                    );
                    painter.line_segment(
                        [screen_rect.right_top(), screen_rect.left_bottom()],
                        Stroke::new(1.0, egui::Color32::GRAY),
                    );
                }
                if obj.line.is_some() {
                    frame.fill = repub_model::styles::Fill::None;
                    frame.line = obj.line.clone();
                    painter.extend(self.shape_shapes(painter.ctx(), &frame, map, obj.opacity as f32));
                }
                // Non-printing frame edge when selected handled by selection overlay.
                if editor.is_some() && !is_selected(obj.common.id) && obj.line.is_none() {
                    painter.rect_stroke(screen_rect, Rounding::ZERO, Stroke::new(1.0, egui::Color32::from_gray(200)));
                }
            }
            ShapeKind::Line(line) => {
                // A rotated line turns its ends about its box's centre
                let c = obj.common.bounds.center();
                let (sin, cos) = obj.common.rotation.to_radians().sin_cos();
                let turn = |p: repub_geometry::Point| {
                    let (dx, dy) = (p.x - c.x, p.y - c.y);
                    map.point(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos)
                };
                let mut pts: Vec<Pos2> = crate::hit_test::line_polyline(line).into_iter().map(turn).collect();
                let (start, end) = (pts[0], pts[pts.len() - 1]);

                // The line's stroke lives on the shape wrapper (`None` for no line)
                let Some(line_stroke) = obj.line.clone() else { return };
                let color = core_color_to_egui(&line_stroke.color, 1.0);
                let stroke = Stroke::new(line_stroke.width as f32 * zoom, color);
                // Direction the line leaves each end, pointing out of the line
                let out_start = pts.get(1).map_or(Vec2::ZERO, |p| (start - *p).normalized());
                let out_end = pts.len().checked_sub(2).map_or(Vec2::ZERO, |i| (end - pts[i]).normalized());
                if out_start.length_sq() > 0.0 && out_end.length_sq() > 0.0 {
                    // Endings, and stop the line short of filled tips
                    let (start_shapes, start_inset) = line_ending_shapes(&line.start_ending, start, out_start, stroke.width, color);
                    let (end_shapes, end_inset) = line_ending_shapes(&line.end_ending, end, out_end, stroke.width, color);
                    let length: f32 = pts.windows(2).map(|w| (w[1] - w[0]).length()).sum();
                    if start_inset + end_inset < length {
                        let last = pts.len() - 1;
                        pts[0] = start - out_start * start_inset;
                        pts[last] = end - out_end * end_inset;
                    }
                    painter.extend(stroke_path(&pts, false, stroke, line_stroke.dash_pattern.as_deref(), zoom));
                    painter.extend(start_shapes);
                    painter.extend(end_shapes);
                } else {
                    painter.add(egui::Shape::line(pts, stroke));
                }
            }
            ShapeKind::Table(table) => {
                use crate::table_view::{visible_cells, TableGrid};
                let grid = TableGrid::of(obj);
                let cells = visible_cells(obj, &grid);
                let stroke_of = |s: &Option<repub_model::styles::Stroke>| {
                    s.as_ref()
                        .filter(|s| s.width > 0.0 && s.color.a > 0.0)
                        .map(|s| Stroke::new((s.width as f32 * zoom).max(0.5), core_color_to_egui(&s.color, 1.0)))
                };

                // Shapes are collected, then turned with the table's rotation
                let mut shapes: Vec<egui::Shape> = Vec::new();
                let rotation = obj.common.rotation as f32;
                let (sin, cos) = rotation.to_radians().sin_cos();
                let centre = screen_rect.center();

                // Cell fills, then text, then borders on top
                for c in &cells {
                    let cell_rect = map.rect(&c.rect);
                    match &c.cell.fill {
                        repub_model::styles::Fill::Solid(color) => {
                            shapes.push(egui::Shape::rect_filled(cell_rect, Rounding::ZERO, core_color_to_egui(color, 1.0)));
                        }
                        repub_model::styles::Fill::LinearGradient(_) | repub_model::styles::Fill::RadialGradient(_) => {
                            // Create a shape from the cell rect, apply gradient
                            let outline = vec![egui::Shape::rect_filled(cell_rect, Rounding::ZERO, egui::Color32::WHITE)];
                            shapes.push(crate::gradient::fill_shapes(outline, &c.cell.fill, c.rect, 0.0, map, 1.0));
                        }
                        repub_model::styles::Fill::None => {}
                        other => {
                            // Pattern or texture - use primary color as fallback
                            shapes.push(egui::Shape::rect_filled(cell_rect, Rounding::ZERO, core_color_to_egui(&other.primary_color(), 1.0)));
                        }
                    }
                }
                painter.extend(rotate_shapes(std::mem::take(&mut shapes), centre, sin, cos));
                for c in &cells {
                    if c.cell.content.plain_text().is_empty() {
                        continue;
                    }
                    // Same layout and MTSDF renderer as text boxes; a rotated table
                    // turns each cell's text about the table's centre
                    let tf = crate::table_view::placed_cell_frame(obj, c.cell, c.rect);
                    if !crate::sdf_text::draw_text_frame(painter, origin, zoom, &tf) {
                        // Fallback: use egui text with proper alignment and size
                        let cell_rect = map.rect(&c.rect);
                        let pad = c.cell.padding as f32 * zoom;
                        let padded = Rect::from_min_max(
                            cell_rect.min + Vec2::splat(pad),
                            cell_rect.max - Vec2::splat(pad),
                        );
                        // Get alignment from first paragraph
                        let align = c.cell.content.paragraphs.first()
                            .map(|p| match p.style.alignment {
                                repub_model::styles::TextAlignment::Center => egui::Align2::CENTER_TOP,
                                repub_model::styles::TextAlignment::Right => egui::Align2::RIGHT_TOP,
                                _ => egui::Align2::LEFT_TOP,
                            })
                            .unwrap_or(egui::Align2::LEFT_TOP);
                        // Get font size from first run, but cap to fit cell height
                        let raw_font_size = c.cell.content.paragraphs.first()
                            .and_then(|p| p.elements.first())
                            .and_then(|e| match e {
                                repub_model::text::InlineElement::Text(r) => Some(r.style.font_size as f32),
                                _ => None,
                            })
                            .unwrap_or(12.0);
                        // Scale font to fit within the cell height (with some padding)
                        let max_font_size = (padded.height() / zoom * 0.85).max(6.0);
                        let font_size = raw_font_size.min(max_font_size);
                        
                        // Vertically center the text in the cell
                        let v_offset = ((padded.height() - font_size * zoom) / 2.0).max(0.0);
                        let pos = match align {
                            egui::Align2::RIGHT_TOP => egui::pos2(padded.max.x, padded.min.y + v_offset),
                            egui::Align2::CENTER_TOP => egui::pos2(padded.center().x, padded.min.y + v_offset),
                            _ => egui::pos2(padded.min.x, padded.min.y + v_offset),
                        };
                        painter.with_clip_rect(cell_rect.intersect(painter.clip_rect())).text(
                            pos,
                            align,
                            c.cell.content.plain_text(),
                            egui::FontId::proportional((font_size * zoom).max(1.0)),
                            egui::Color32::BLACK,
                        );
                    }
                }

                // The cell being edited: its selection and blinking caret
                let editing_here = editor.and_then(|s| s.text_edit.as_ref()).is_some_and(|e| e.frame == obj.common.id && e.cell.is_some());
                if let Some((tf, layout, edit)) = editor.filter(|_| editing_here).and_then(text_edit::draw_info) {
                    draw_edit_marks(painter, &tf, &layout, edit, origin, zoom);
                }

                // Non-printing outline so a borderless table can still be seen and found
                let outer = map.rect(&obj.common.bounds);
                if editor.is_some() && !is_selected(obj.common.id) {
                    shapes.extend(egui::Shape::dashed_line(
                        &[outer.left_top(), outer.right_top(), outer.right_bottom(), outer.left_bottom(), outer.left_top()],
                        Stroke::new(1.0, colors::text_boundary()),
                        3.0,
                        2.0,
                    ));
                }

                // Inner borders: each cell's right and bottom edge inside the table
                let (right, bottom) = (grid.xs[grid.cols()], grid.ys[grid.rows()]);
                let inner_v = stroke_of(&table.borders.inner_vertical);
                let inner_h = stroke_of(&table.borders.inner_horizontal);
                for c in &cells {
                    let r = map.rect(&c.rect);
                    if let Some(stroke) = inner_v.filter(|_| c.rect.origin.x + c.rect.size.width < right - 1e-6) {
                        shapes.push(egui::Shape::line_segment([r.right_top(), r.right_bottom()], stroke));
                    }
                    if let Some(stroke) = inner_h.filter(|_| c.rect.origin.y + c.rect.size.height < bottom - 1e-6) {
                        shapes.push(egui::Shape::line_segment([r.left_bottom(), r.right_bottom()], stroke));
                    }
                }
                
                // Non-printing inner cell boundaries when table has no visible inner borders
                // (so users can see the row/column structure in the editor)
                if editor.is_some() && inner_v.is_none() && inner_h.is_none() {
                    for c in &cells {
                        let r = map.rect(&c.rect);
                        // Draw dashed line at bottom of each cell (except last row)
                        if c.rect.origin.y + c.rect.size.height < bottom - 1e-6 {
                            shapes.extend(egui::Shape::dashed_line(
                                &[r.left_bottom(), r.right_bottom()],
                                Stroke::new(1.0, colors::text_boundary()),
                                3.0,
                                2.0,
                            ));
                        }
                        // Draw dashed line at right of each cell (except last column)
                        if c.rect.origin.x + c.rect.size.width < right - 1e-6 {
                            shapes.extend(egui::Shape::dashed_line(
                                &[r.right_top(), r.right_bottom()],
                                Stroke::new(1.0, colors::text_boundary()),
                                3.0,
                                2.0,
                            ));
                        }
                    }
                }
                // Outer borders
                for (border, ends) in [
                    (&table.borders.top, [outer.left_top(), outer.right_top()]),
                    (&table.borders.right, [outer.right_top(), outer.right_bottom()]),
                    (&table.borders.bottom, [outer.left_bottom(), outer.right_bottom()]),
                    (&table.borders.left, [outer.left_top(), outer.left_bottom()]),
                ] {
                    if let Some(stroke) = stroke_of(border) {
                        shapes.push(egui::Shape::line_segment(ends, stroke));
                    }
                }
                painter.extend(rotate_shapes(shapes, centre, sin, cos));
            }
            // Text on a path: text following a custom path
            ShapeKind::TextEffect(wordart) if matches!(wordart.transform, repub_model::shape::TextTransform::Path(_)) => {
                let tf = fitted_text_frame(obj);
                let text = wordart.text.clone();
                if !crate::sdf_text::draw_text_frame(painter, origin, zoom, &tf) {
                    painter.with_clip_rect(screen_rect.intersect(painter.clip_rect())).text(
                        screen_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        text,
                        egui::FontId::proportional((screen_rect.height() * 0.6).max(1.0)),
                        egui::Color32::BLACK,
                    );
                }
            }
            // WordArt: decorative text with fill, outline, and transforms
            ShapeKind::TextEffect(wordart) => {
                let text = wordart.text.trim();
                if !text.is_empty() {
                    use crate::wordart_warp::WarpPreset;
                    use repub_model::shape::TextTransform;
                    
                    // Determine warp preset from transform
                    let warp_preset = match &wordart.transform {
                        TextTransform::Preset(p) => {
                            use repub_model::shape::WordArtPreset;
                            match p {
                                WordArtPreset::Plain => WarpPreset::Plain,
                                WordArtPreset::Wave => WarpPreset::Wave1,
                                WordArtPreset::ArchUp => WarpPreset::ArchUp,
                                WordArtPreset::ArchDown => WarpPreset::ArchDown,
                                WordArtPreset::Circle => WarpPreset::Circle,
                                WordArtPreset::Inflate => WarpPreset::Inflate,
                                WordArtPreset::Deflate => WarpPreset::Deflate,
                                WordArtPreset::SlantUp => WarpPreset::SlantUp,
                                WordArtPreset::SlantDown => WarpPreset::SlantDown,
                                WordArtPreset::FadeRight => WarpPreset::FadeRight,
                                WordArtPreset::FadeLeft => WarpPreset::FadeLeft,
                                WordArtPreset::ChevronUp => WarpPreset::Chevron,
                                WordArtPreset::ChevronDown => WarpPreset::ChevronInverted,
                                WordArtPreset::TriangleUp => WarpPreset::Triangle,
                                WordArtPreset::TriangleDown => WarpPreset::TriangleInverted,
                                WordArtPreset::CanUp => WarpPreset::CanUp,
                                WordArtPreset::CanDown => WarpPreset::CanDown,
                                WordArtPreset::CascadeUp => WarpPreset::CascadeUp,
                                WordArtPreset::CascadeDown => WarpPreset::CascadeDown,
                                WordArtPreset::Stop => WarpPreset::Stop,
                                WordArtPreset::RingInside => WarpPreset::RingInside,
                                WordArtPreset::RingOutside => WarpPreset::RingOutside,
                                WordArtPreset::Button => WarpPreset::Button,
                                _ => WarpPreset::Plain,
                            }
                        }
                        _ => WarpPreset::Plain,
                    };
                    
                    let rotation = obj.common.rotation as f32;
                    let fill_color = core_color_to_egui(&obj.fill.primary_color(), 1.0);
                    let outline = obj.line.as_ref().map(|s| {
                        (core_color_to_egui(&s.color, 1.0), (s.width as f32 * zoom).max(0.5))
                    });
                    
                    // Use path-based rendering for all presets (proper glyph outlines and strokes)
                    // Use gradient-aware rendering if fill is a gradient
                    let fill = if crate::gradient::is_gradient(&obj.fill) {
                        crate::wordart_warp::WarpFill::Gradient(&obj.fill)
                    } else {
                        crate::wordart_warp::WarpFill::Solid(fill_color)
                    };
                    // Get font info from WordArt style
                    let font_family = if wordart.style.font.family.is_empty() {
                        None
                    } else {
                        Some(wordart.style.font.family.as_str())
                    };
                    let bold = matches!(wordart.style.font.weight, repub_model::styles::FontWeight::Bold | repub_model::styles::FontWeight::ExtraBold | repub_model::styles::FontWeight::Black);
                    let italic = matches!(wordart.style.font.style, repub_model::styles::FontStyle::Italic | repub_model::styles::FontStyle::Oblique);
                    
                    // Build 3D extrusion params if present
                    let extrusion_params = wordart.extrusion.as_ref().map(|ext| {
                        // Calculate extrusion direction based on 3D rotation
                        let angle_x = wordart.rotation_3d.x as f32;
                        let angle_y = wordart.rotation_3d.y as f32;
                        
                        // Direction is derived from rotation in the render function
                        let dir_x = angle_y.to_radians().sin() * 0.7;
                        let dir_y = -angle_x.to_radians().sin() * 0.5 + 0.5;
                        
                        let extrusion_color = ext.color.map(|c| core_color_to_egui(&c, 1.0));
                        
                        // Map bevel type from core to render type
                        let bevel = ext.bevel.as_ref().map(|b| {
                            use repub_model::shape::BevelType;
                            match b {
                                BevelType::Flat => crate::wordart_path::Bevel3D::None,
                                BevelType::Circle => crate::wordart_path::Bevel3D::Circle,
                                BevelType::RelaxedInset => crate::wordart_path::Bevel3D::RelaxedInset,
                                BevelType::Cross => crate::wordart_path::Bevel3D::Cross,
                                BevelType::Convex => crate::wordart_path::Bevel3D::Convex,
                                BevelType::CoolSlant => crate::wordart_path::Bevel3D::CoolSlant,
                                BevelType::Angle => crate::wordart_path::Bevel3D::Angle,
                                BevelType::SoftRound => crate::wordart_path::Bevel3D::SoftRound,
                                BevelType::ArtDeco => crate::wordart_path::Bevel3D::Convex,  // Map to similar
                                BevelType::Slope => crate::wordart_path::Bevel3D::Angle,
                                BevelType::Divot => crate::wordart_path::Bevel3D::RelaxedInset,
                                BevelType::Riblet => crate::wordart_path::Bevel3D::Cross,
                                BevelType::HardEdge => crate::wordart_path::Bevel3D::HardEdge,
                            }
                        }).unwrap_or(crate::wordart_path::Bevel3D::None);
                        
                        crate::wordart_path::Extrusion3DParams {
                            depth: (ext.depth as f32 * zoom).max(0.0),
                            direction: (dir_x, dir_y),
                            color: extrusion_color,
                            rotation_x: angle_x,
                            rotation_y: angle_y,
                            perspective: wordart.rotation_3d.perspective as f32,
                            bevel,
                            bevel_size: (ext.bevel_size as f32 * zoom).max(0.0),
                            lighting: crate::wordart_path::Lighting3D::default(),
                            material: crate::wordart_path::Material3D::default(),
                            contour_width: (ext.contour_width as f32 * zoom).max(0.0),
                            contour_color: None,
                        }
                    });
                    
                    // Use path-based renderer for all WordArt (proper outlines and curves)
                    crate::wordart_path::render_wordart_path_3d(
                        painter,
                        text,
                        screen_rect,
                        warp_preset,
                        &fill,
                        outline,
                        font_family,
                        bold,
                        italic,
                        extrusion_params.as_ref(),
                    );
                    
                    // Legacy code for reference - no longer used
                    if false {
                        // Plain preset: use whole-string rendering with gradient support
                        let measure_size = 72.0_f32;
                        let measure_galley = painter.layout_no_wrap(
                            text.to_string(),
                            egui::FontId::proportional(measure_size),
                            egui::Color32::WHITE,
                        );
                        let text_w = measure_galley.rect.width().max(1.0);
                        let text_h = measure_galley.rect.height().max(1.0);
                        
                        let padding = 4.0 * zoom;
                        let avail_w = (screen_rect.width() - padding * 2.0).max(1.0);
                        let avail_h = (screen_rect.height() - padding * 2.0).max(1.0);
                        let scale = (avail_w / text_w).min(avail_h / text_h).min(10.0);
                        let font_size = (measure_size * scale).max(6.0);
                        
                        let center = screen_rect.center();
                        let font_id = egui::FontId::proportional(font_size);
                        
                        // Draw outline first (behind fill)
                        if let Some((outline_color, outline_width)) = outline {
                            let outline_galley = painter.layout_no_wrap(text.to_string(), font_id.clone(), outline_color);
                            
                            let offsets = [
                                (-outline_width, 0.0), (outline_width, 0.0),
                                (0.0, -outline_width), (0.0, outline_width),
                                (-outline_width * 0.707, -outline_width * 0.707),
                                (outline_width * 0.707, -outline_width * 0.707),
                                (-outline_width * 0.707, outline_width * 0.707),
                                (outline_width * 0.707, outline_width * 0.707),
                            ];
                            
                            for (dx, dy) in offsets {
                                let offset = egui::vec2(
                                    -outline_galley.rect.width() / 2.0 + dx,
                                    -outline_galley.rect.height() / 2.0 + dy,
                                );
                                painter.add(egui::Shape::Text(egui::epaint::TextShape {
                                    pos: center + offset,
                                    galley: outline_galley.clone(),
                                    underline: Stroke::NONE,
                                    fallback_color: outline_color,
                                    override_text_color: Some(outline_color),
                                    opacity_factor: 1.0,
                                    angle: rotation.to_radians(),
                                }));
                            }
                        }
                        
                        // Draw fill with gradient support
                        if crate::gradient::is_gradient(&obj.fill) {
                            let num_slices = 20;
                            let slice_height = font_size / num_slices as f32;
                            
                            for i in 0..num_slices {
                                let v = i as f64 / (num_slices - 1) as f64;
                                let color = crate::gradient::color_at(&obj.fill, 0.5, v);
                                let slice_color = core_color_to_egui(&color, 1.0);
                                
                                let slice_galley = painter.layout_no_wrap(text.to_string(), font_id.clone(), slice_color);
                                let galley_height = slice_galley.rect.height();
                                let galley_width = slice_galley.rect.width();
                                
                                let slice_top = center.y - galley_height / 2.0 + i as f32 * slice_height;
                                let slice_rect = egui::Rect::from_min_size(
                                    egui::pos2(center.x - galley_width / 2.0 - 10.0, slice_top),
                                    egui::vec2(galley_width + 20.0, slice_height + 1.0),
                                );
                                
                                let clip_rect = if rotation.abs() > 0.01 {
                                    screen_rect
                                } else {
                                    slice_rect.intersect(screen_rect)
                                };
                                
                                let offset = egui::vec2(-galley_width / 2.0, -galley_height / 2.0);
                                painter.with_clip_rect(clip_rect).add(egui::Shape::Text(egui::epaint::TextShape {
                                    pos: center + offset,
                                    galley: slice_galley,
                                    underline: Stroke::NONE,
                                    fallback_color: slice_color,
                                    override_text_color: Some(slice_color),
                                    opacity_factor: 1.0,
                                    angle: rotation.to_radians(),
                                }));
                            }
                        } else {
                            let fill_galley = painter.layout_no_wrap(text.to_string(), font_id, fill_color);
                            
                            let offset = egui::vec2(-fill_galley.rect.width() / 2.0, -fill_galley.rect.height() / 2.0);
                            painter.add(egui::Shape::Text(egui::epaint::TextShape {
                                pos: center + offset,
                                galley: fill_galley,
                                underline: Stroke::NONE,
                                fallback_color: fill_color,
                                override_text_color: Some(fill_color),
                                opacity_factor: 1.0,
                                angle: rotation.to_radians(),
                            }));
                        }
                    }
                }
            }
            ShapeKind::Group(group) => {
                for child in &group.children {
                    self.draw_object(painter, origin, zoom, child, editor);
                }
            }
        }
    }
    fn draw_selection(&self, painter: &egui::Painter, page_rect: Rect, state: &AppState) {
        let Some(frame) = self.selection_frame(state) else { return };
        let to_screen = |p: Point| self.doc_to_screen_point(page_rect, &p, state);
        let outline = Stroke::new(1.0, colors::selection_frame());
        let corners = |f: &SelectionFrame| {
            let r = f.rect;
            [r.origin, Point::new(r.origin.x + r.size.width, r.origin.y), Point::new(r.origin.x + r.size.width, r.origin.y + r.size.height), Point::new(r.origin.x, r.origin.y + r.size.height)]
                .map(|p| to_screen(f.to_page(p)))
                .to_vec()
        };

        // Each selected object is outlined along its own outline (turned with it); the
        // box the handles sit on is drawn faintly behind a shape that isn't box-shaped
        let objects = state.selected_objects();
        let outline_of = |obj: &Shape, stroke: Stroke| {
            let trace = |points: &[Point], closed: bool| {
                let pts: Vec<Pos2> = points.iter().map(|p| to_screen(*p)).collect();
                if closed { egui::Shape::closed_line(pts, stroke) } else { egui::Shape::line(pts, stroke) }
            };
            match selection_outline(obj, state.zoom) {
                Some(paths) => paths.iter().map(|(points, closed)| trace(points, *closed)).collect::<Vec<_>>(),
                None => {
                    let f = SelectionFrame { rect: obj.bounds(), rotation: obj.common.rotation };
                    vec![egui::Shape::closed_line(corners(&f), stroke)]
                }
            }
        };
        let faint = colors::selection_frame().gamma_multiply(0.35);
        let single = objects.len() == 1;
        for obj in &objects {
            let stroke = if single { outline } else { Stroke::new(1.0, colors::selection_frame().gamma_multiply(0.6)) };
            if selection_outline(obj, state.zoom).is_some() {
                let f = SelectionFrame { rect: obj.bounds(), rotation: obj.common.rotation };
                painter.add(egui::Shape::closed_line(corners(&f), Stroke::new(1.0, faint)));
            }
            painter.extend(outline_of(obj, stroke));
        }
        // Several objects: the box around them all
        if !single {
            painter.add(egui::Shape::closed_line(corners(&frame), outline));
        }

        // Rotation handle: circular-arrow glyph above the top centre, joined by a thin line
        let handle_radius = metrics::HANDLE_SIZE / 2.0 - 0.5;
        let glyph_radius = metrics::ROTATION_HANDLE_SIZE / 2.0 - 0.5;
        let top_center = to_screen(frame.to_page(ResizeHandle::TopCenter.position(&frame.rect)));
        let (sin, cos) = (frame.rotation as f32).to_radians().sin_cos();
        let up = Vec2::new(sin, -cos);
        let rotation_pos = top_center + up * metrics::ROTATION_HANDLE_DISTANCE;
        painter.line_segment([top_center + up * handle_radius, rotation_pos - up * (glyph_radius + 2.0)], outline);
        draw_rotation_glyph(painter, rotation_pos, glyph_radius);

        // Selection handles (Publisher 2016+: small white circles, gray outline)
        for handle in ResizeHandle::all() {
            painter.circle(
                to_screen(frame.to_page(handle.position(&frame.rect))),
                handle_radius,
                colors::handle_fill(),
                Stroke::new(metrics::HANDLE_BORDER, colors::handle_outline()),
            );
        }

        // Adjust handles of a preset shape: small yellow diamonds
        if let [shape] = objects[..] {
            for h in adjust_handles(shape) {
                let c = to_screen(h);
                let r = metrics::HANDLE_SIZE / 2.0 + 0.5;
                let diamond = vec![c + Vec2::new(0.0, -r), c + Vec2::new(r, 0.0), c + Vec2::new(0.0, r), c + Vec2::new(-r, 0.0)];
                painter.add(egui::Shape::convex_polygon(diamond, egui::Color32::from_rgb(0xFF, 0xD7, 0x00), Stroke::new(1.0, egui::Color32::from_rgb(0x80, 0x60, 0x00))));
            }
        }

        // Draw feedback overlay during drag/resize/rotate operations
        self.draw_interaction_feedback(painter, page_rect, state, Some(frame.rect));
    }

    /// Draw position/size/angle feedback during drag, resize, or rotate operations.
    fn draw_interaction_feedback(
        &self,
        painter: &egui::Painter,
        page_rect: Rect,
        state: &AppState,
        selection_bounds: Option<repub_geometry::Rect>,
    ) {
        let bounds = match selection_bounds {
            Some(b) => b,
            None => return,
        };

        // Determine feedback text and position based on operation
        let (feedback_text, tooltip_pos) = if self.select_state.is_dragging && !self.select_state.is_resizing {
            // Position feedback while moving
            let text = format!("X: {}  Y: {}", state.units.format(bounds.origin.x), state.units.format(bounds.origin.y));
            // Position tooltip near the top-left of the object
            let screen_pos = self.doc_to_screen_point(page_rect, &bounds.origin, state);
            let tooltip_pos = Pos2::new(screen_pos.x + 15.0, screen_pos.y - 25.0);
            (text, tooltip_pos)
        } else if self.select_state.is_resizing {
            // Size feedback while resizing
            let text = format!("W: {}  H: {}", state.units.format(bounds.width()), state.units.format(bounds.height()));
            // Position tooltip near the active handle (turned with a rotated object)
            let rotation = self.selection_frame(state).map_or(0.0, |f| f.rotation);
            let handle_pos = if let Some(handle) = self.select_state.resize_handle {
                let doc_pos = SelectionFrame { rect: bounds, rotation }.to_page(handle.position(&bounds));
                self.doc_to_screen_point(page_rect, &doc_pos, state)
            } else {
                let center = Point::new(
                    bounds.origin.x + bounds.width() / 2.0,
                    bounds.origin.y + bounds.height() / 2.0,
                );
                self.doc_to_screen_point(page_rect, &center, state)
            };
            let tooltip_pos = Pos2::new(handle_pos.x + 15.0, handle_pos.y - 25.0);
            (text, tooltip_pos)
        } else if self.select_state.is_rotating {
            // Draw rotation center indicator
            self.draw_rotation_center_indicator(painter, page_rect, state, &bounds);
            
            // Angle feedback while rotating
            let angle_deg = if let Some(delta) = self.select_state.rotation_delta() {
                delta.to_degrees()
            } else {
                0.0
            };
            let text = format!("{:.1}°", angle_deg);
            // Position near the rotation handle, which turns with the object
            let frame = SelectionFrame { rect: bounds, rotation: self.selection_frame(state).map_or(0.0, |f| f.rotation) };
            let rotation_distance = metrics::ROTATION_HANDLE_DISTANCE as f64 / state.zoom as f64;
            let rotation_pos = frame.to_page(Point::new(bounds.origin.x + bounds.width() / 2.0, bounds.origin.y - rotation_distance));
            let screen_pos = self.doc_to_screen_point(page_rect, &rotation_pos, state);
            let tooltip_pos = Pos2::new(screen_pos.x + 15.0, screen_pos.y - 10.0);
            (text, tooltip_pos)
        } else {
            return;
        };

        // Draw the tooltip background and text
        self.draw_feedback_tooltip(painter, tooltip_pos, &feedback_text);
    }

    /// Draw a crosshair indicator at the rotation center point.
    fn draw_rotation_center_indicator(
        &self,
        painter: &egui::Painter,
        page_rect: Rect,
        state: &AppState,
        bounds: &repub_geometry::Rect,
    ) {
        // Calculate the center of the selection
        let center = Point::new(
            bounds.origin.x + bounds.width() / 2.0,
            bounds.origin.y + bounds.height() / 2.0,
        );
        let screen_center = self.doc_to_screen_point(page_rect, &center, state);
        
        // Draw crosshair lines
        let crosshair_size = 8.0;
        let stroke = Stroke::new(1.5, colors::rotation_handle());
        
        // Horizontal line
        painter.line_segment(
            [
                Pos2::new(screen_center.x - crosshair_size, screen_center.y),
                Pos2::new(screen_center.x + crosshair_size, screen_center.y),
            ],
            stroke,
        );
        
        // Vertical line
        painter.line_segment(
            [
                Pos2::new(screen_center.x, screen_center.y - crosshair_size),
                Pos2::new(screen_center.x, screen_center.y + crosshair_size),
            ],
            stroke,
        );
        
        // Small circle at center
        painter.circle_stroke(screen_center, 3.0, stroke);
    }

    /// Draw a tooltip with a semi-transparent background.
    fn draw_feedback_tooltip(&self, painter: &egui::Painter, pos: Pos2, text: &str) {
        let font = egui::FontId::proportional(11.0);
        let galley = painter.layout_no_wrap(text.to_string(), font.clone(), colors::text_primary());
        
        let padding = Vec2::new(6.0, 3.0);
        let bg_rect = Rect::from_min_size(
            pos,
            galley.size() + padding * 2.0,
        );
        
        // A tooltip in the theme's surface colour, so its text reads clearly
        painter.rect_filled(bg_rect.translate(Vec2::new(0.0, 1.0)), Rounding::same(3.0), egui::Color32::from_black_alpha(25));
        painter.rect_filled(bg_rect, Rounding::same(3.0), colors::bg_window());
        painter.rect_stroke(bg_rect, Rounding::same(3.0), Stroke::new(1.0, colors::border_strong()));
        
        // Text
        let text_pos = pos + padding;
        painter.galley(text_pos, galley, colors::text_primary());
    }

    fn draw_preview(&self, painter: &egui::Painter, page_rect: Rect, state: &AppState) {
        if let Some(bounds) = self.draw_state.bounds() {
            let screen_rect = self.doc_to_screen_rect(page_rect, &bounds, state);
            
            let stroke = Stroke::new(1.5, colors::accent());
            // A shape that isn't box-shaped previews along its own outline
            let outline = self
                .draw_state
                .new_object(state.current_tool)
                .and_then(|shape| selection_outline(&shape, state.zoom));
            match outline {
                Some(paths) => {
                    for (points, closed) in paths {
                        let pts: Vec<Pos2> = points.iter().map(|p| self.doc_to_screen_point(page_rect, p, state)).collect();
                        painter.add(if closed { egui::Shape::closed_line(pts, stroke) } else { egui::Shape::line(pts, stroke) });
                    }
                }
                None => {
                    painter.rect_stroke(screen_rect, Rounding::ZERO, stroke);
                }
            }
        } else if let Some((start, end)) = self.draw_state.line() {
            let screen_start = self.doc_to_screen_point(page_rect, &start, state);
            let screen_end = self.doc_to_screen_point(page_rect, &end, state);
            
            painter.line_segment(
                [screen_start, screen_end],
                Stroke::new(1.5, colors::accent()),
            );
        }
    }

    fn draw_marquee(&self, painter: &egui::Painter, page_rect: Rect, state: &AppState) {
        if let Some(rect) = self.select_state.marquee_rect() {
            let screen_rect = self.doc_to_screen_rect(page_rect, &rect, state);
            
            // Draw semi-transparent fill
            painter.rect_filled(
                screen_rect,
                Rounding::ZERO,
                colors::marquee_fill(),
            );
            
            // Draw animated marching ants border
            let ctx = painter.ctx();
            let time = ctx.input(|i| i.time);
            
            // Animation: dash offset cycles through dash pattern
            // Dash pattern: 4px dash, 4px gap = 8px period
            let dash_length = 4.0;
            let gap_length = 4.0;
            let total_period = dash_length + gap_length;
            
            // Animate at ~60px per second for smooth marching effect
            let animation_speed = 60.0;
            let dash_offset = (time * animation_speed) % total_period as f64;
            
            // Draw dashed rectangle using line segments
            let stroke = Stroke::new(1.0, colors::marquee_stroke());
            self.draw_dashed_rect(painter, screen_rect, stroke, dash_length, gap_length, dash_offset as f32);
            
            // Request continuous repaint for animation
            ctx.request_repaint();
        }
    }
    
    /// Draw a dashed rectangle with animated offset for marching ants effect
    fn draw_dashed_rect(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        stroke: Stroke,
        dash_length: f32,
        gap_length: f32,
        offset: f32,
    ) {
        // Draw all four edges with consistent dash pattern flowing clockwise
        let edges = [
            // Top edge: left to right
            (rect.left_top(), rect.right_top()),
            // Right edge: top to bottom
            (rect.right_top(), rect.right_bottom()),
            // Bottom edge: right to left
            (rect.right_bottom(), rect.left_bottom()),
            // Left edge: bottom to top
            (rect.left_bottom(), rect.left_top()),
        ];
        
        let total_period = dash_length + gap_length;
        let mut cumulative_offset = offset;
        
        for (start, end) in edges {
            let edge_length = start.distance(end);
            let direction = (end - start) / edge_length;
            
            // Adjust for cumulative offset from previous edges
            let edge_offset = cumulative_offset % total_period;
            
            // Calculate starting position based on where we are in the dash pattern
            let mut pos = if edge_offset < dash_length {
                // We're in a dash, draw the remaining dash portion
                let remaining_dash = dash_length - edge_offset;
                let dash_end = remaining_dash.min(edge_length);
                let p1 = start;
                let p2 = start + direction * dash_end;
                painter.line_segment([p1, p2], stroke);
                dash_end + gap_length
            } else {
                // We're in a gap, skip the remaining gap portion
                total_period - edge_offset
            };
            
            // Draw remaining dashes
            while pos < edge_length {
                let dash_start = pos;
                let dash_end = (pos + dash_length).min(edge_length);
                
                let p1 = start + direction * dash_start;
                let p2 = start + direction * dash_end;
                painter.line_segment([p1, p2], stroke);
                
                pos += total_period;
            }
            
            // Update cumulative offset for next edge
            cumulative_offset += edge_length;
        }
    }

    /// Keyboard input while editing text. When not editing, Return on a single
    /// selected text frame starts editing at the end of its text.
    fn handle_text_keys(&mut self, ctx: &egui::Context, state: &mut AppState) {
        use egui::{Event, Key};
        let now = ctx.input(|i| i.time);
        let events = ctx.input(|i| i.events.clone());

        if state.link_source.is_some() && events.iter().any(|e| matches!(e, Event::Key { key: Key::Escape, pressed: true, .. })) {
            state.link_source = None;
        }
        if state.text_edit.is_none() {
            let enter = events
                .iter()
                .any(|e| matches!(e, Event::Key { key: Key::Enter, pressed: true, modifiers, .. } if !modifiers.command));
            if enter && state.selection.len() == 1 && text_edit::is_text_frame(state, state.selection[0]) {
                text_edit::begin(state, state.selection[0], None, now);
                state.current_tool = Tool::Select;
            } else if enter && state.selection.len() == 1 && matches!(state.selected_objects().first().map(|s| &s.kind), Some(ShapeKind::Table(_))) {
                // Return on a selected table edits its first cell
                text_edit::begin_cell(state, state.selection[0], 0, 0, None, now);
                state.current_tool = Tool::Select;
            }
            return;
        }

        let mac = cfg!(target_os = "macos");
        for event in events {
            if state.text_edit.is_none() {
                break;
            }
            match event {
                Event::Text(s) => {
                    let s: String = s.chars().filter(|c| !c.is_control()).collect();
                    text_edit::type_text(state, &s, now);
                }
                Event::Paste(s) => text_edit::type_text(state, &s, now),
                Event::Copy | Event::Cut => {
                    let selected = text_edit::selected_text(state);
                    if !selected.is_empty() {
                        ctx.copy_text(selected.clone());
                        state.text_clipboard = selected;
                        if matches!(event, Event::Cut) {
                            text_edit::delete_forward(state, false, now);
                        }
                    }
                }
                Event::Key { key, pressed: true, modifiers, .. } => {
                    use text_edit::Motion;
                    // macOS: Option moves by word, Cmd by line; elsewhere Ctrl moves by word
                    let word = if mac { modifiers.alt } else { modifiers.ctrl };
                    let line = mac && modifiers.command;
                    let shift = modifiers.shift;
                    match key {
                        Key::Escape => text_edit::end(state),
                        Key::Enter if shift => text_edit::line_break(state, now),
                        Key::Enter => text_edit::newline(state, now),
                        // In a table, Tab moves between cells (Ctrl+Tab types a tab)
                        Key::Tab if state.text_edit.as_ref().is_some_and(|e| e.cell.is_some()) && !modifiers.ctrl => {
                            text_edit::next_cell(state, !shift, now)
                        }
                        Key::Tab => text_edit::type_text(state, "\t", now),
                        Key::Backspace => text_edit::backspace(state, word, now),
                        Key::Delete => text_edit::delete_forward(state, word, now),
                        Key::ArrowLeft => text_edit::move_caret(state, if line { Motion::LineStart } else if word { Motion::WordLeft } else { Motion::Left }, shift, now),
                        Key::ArrowRight => text_edit::move_caret(state, if line { Motion::LineEnd } else if word { Motion::WordRight } else { Motion::Right }, shift, now),
                        Key::ArrowUp => text_edit::move_caret(state, if line { Motion::DocStart } else { Motion::Up }, shift, now),
                        Key::ArrowDown => text_edit::move_caret(state, if line { Motion::DocEnd } else { Motion::Down }, shift, now),
                        Key::Home => text_edit::move_caret(state, if modifiers.command { Motion::DocStart } else { Motion::LineStart }, shift, now),
                        Key::End => text_edit::move_caret(state, if modifiers.command { Motion::DocEnd } else { Motion::LineEnd }, shift, now),
                        Key::A if modifiers.command => text_edit::select_all(state, now),
                        Key::Z if modifiers.command => text_edit::undo(state, shift, now),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    /// Mouse input while editing text. Returns true when the event was used by the
    /// text editor; a press outside the edited frame ends editing and returns false
    /// so the same click selects or deselects as usual.
    fn handle_text_mouse(&mut self, response: &egui::Response, doc_pos: Point, state: &mut AppState) -> bool {
        let Some(frame_id) = state.text_edit.as_ref().map(|e| e.frame) else {
            self.text_selecting = false;
            return false;
        };
        let (x, y) = (doc_pos.x as f32, doc_pos.y as f32);
        let inside = text_edit::edit_contains(state, x, y);
        let (now, shift, pressed, down) = response.ctx.input(|i| (i.time, i.modifiers.shift, i.pointer.primary_pressed(), i.pointer.primary_down()));

        // Editing a table: a press in another of its cells moves editing there
        let editing_cell = state.text_edit.as_ref().is_some_and(|e| e.cell.is_some());
        if pressed && !inside && editing_cell && text_edit::begin_cell_at_point(state, frame_id, x, y, now) {
            self.text_selecting = true;
            return true;
        }

        if response.double_clicked() && inside {
            text_edit::select_word_at(state, x, y, now);
            self.text_selecting = false;
            return true;
        }
        if pressed {
            if inside {
                text_edit::click_at(state, x, y, shift, now);
                self.text_selecting = true;
                return true;
            }
            text_edit::end(state);
            self.text_selecting = false;
            return false;
        }
        if self.text_selecting {
            if response.dragged() {
                text_edit::drag_to(state, x, y, now);
            }
            if !down {
                self.text_selecting = false;
            }
            return true;
        }
        // Swallow the click/drag events that follow a press inside the frame
        inside
    }

    fn handle_input(&mut self, response: &egui::Response, canvas_rect: Rect, page_rect: Rect, state: &mut AppState) {
        // Handle keyboard shortcuts
        let ctx = response.ctx.clone();
        
        // Handle middle mouse button drag for panning (regardless of tool)
        let middle_button_down = ctx.input(|i| i.pointer.middle_down());
        if middle_button_down && response.dragged() {
            let delta = response.drag_delta();
            state.pan_offset.x += delta.x as f64;
            state.pan_offset.y += delta.y as f64;
        }
        
        // Text editing keyboard input (and Return to start editing a selected text frame)
        self.handle_text_keys(&ctx, state);
        self.handle_pen_keys(&ctx, state);

        // Handle mouse input
        if let Some(pos) = response.interact_pointer_pos() {
            let doc_pos = self.screen_to_doc_point(page_rect, pos, state);

            // Cmd/Ctrl+click on a hyperlink follows it (as in Publisher)
            if response.clicked() && ctx.input(|i| i.modifiers.command) {
                if let Some(link) = self.link_under(doc_pos, state) {
                    state.follow_link = Some(link);
                    return;
                }
            }

            // While editing text, the edited frame takes the mouse; a press outside ends editing
            let text_consumed = !middle_button_down && self.handle_text_mouse(response, doc_pos, state);

            // Skip tool handling if middle button is for panning
            if !middle_button_down && !text_consumed {
                match state.current_tool {
                    Tool::Select => self.handle_select_input(response, doc_pos, page_rect, state),
                    Tool::Pan => self.handle_pan_input(response, state),
                    Tool::Zoom => self.handle_zoom_input(response, canvas_rect, state),
                    Tool::Freeform | Tool::Pencil => self.handle_pen_input(response, doc_pos, state),
                    tool if tool.is_creation_tool() => self.handle_draw_input(response, doc_pos, state),
                    _ => {}
                }
            }
        }

        // Handle zoom and scroll
        if response.hovered() {
            // Zoom keeps the point under the pointer fixed
            let anchor = response.hover_pos().unwrap_or(canvas_rect.center());

            // Pinch-to-zoom on trackpad, and Cmd/Ctrl+wheel: egui reports both as a
            // smooth zoom factor (it converts command-modified scrolling itself)
            let zoom_delta = ctx.input(|i| i.zoom_delta());
            if zoom_delta != 1.0 {
                zoom_about(state, canvas_rect, anchor, state.zoom * zoom_delta);
            }
            
            // Plain scrolling pans (the command-modified scroll above zooms instead)
            let (scroll_delta, command) = ctx.input(|i| (i.raw_scroll_delta, i.modifiers.command));
            if !command {
                state.pan_offset.y += scroll_delta.y as f64;
                // Horizontal scroll (two-finger swipe on trackpad)
                state.pan_offset.x += scroll_delta.x as f64;
            }
        }

        // Update modifiers for drawing
        ctx.input(|i| {
            self.draw_state.constrain = i.modifiers.shift;
            self.draw_state.from_center = i.modifiers.alt;
        });
    }

    fn handle_select_input(&mut self, response: &egui::Response, doc_pos: Point, page_rect: Rect, state: &mut AppState) {
        let shift_held = response.ctx.input(|i| i.modifiers.shift);
        // Use command modifier (Cmd on macOS, Ctrl on other platforms)
        let cmd_held = response.ctx.input(|i| i.modifiers.command);

        // Shape point editing consumes input first.
        if state.edit_points.is_some() {
            let pointer = response.ctx.input(|i| i.pointer.interact_pos()).unwrap_or(Pos2::ZERO);
            if crate::edit_points::handle_input(response, pointer, page_rect, state) {
                return;
            }
        }
        
        // Check for handle hover (for cursor)
        if !self.select_state.is_busy() && self.guide_state.dragging_guide.is_none() {
            self.select_state.hovered_handle = self.hit_test_handles(doc_pos, page_rect, state);
            
            // Check for guide hover
            if state.show_guides && !state.guides_locked {
                let tolerance = metrics::GUIDE_HIT_TOLERANCE as f64 / state.zoom as f64;
                self.guide_state.hovered_guide = state.find_guide_at(doc_pos, tolerance).map(|(idx, _)| idx);
            } else {
                self.guide_state.hovered_guide = None;
            }
        }

        // Handle double-click to delete guide or edit object
        if response.double_clicked() {
            // First check if double-clicking a guide to delete it
            if state.show_guides && !state.guides_locked {
                let tolerance = metrics::GUIDE_HIT_TOLERANCE as f64 / state.zoom as f64;
                if let Some((guide_idx, _)) = state.find_guide_at(doc_pos, tolerance) {
                    if !state.guides()[guide_idx].locked {
                        state.remove_guide(guide_idx);
                        self.guide_state.hovered_guide = None;
                        return;
                    }
                }
            }
            
            // Then double-click a text frame to edit its text, caret at the pointer.
            // Double-clicking empty space deselects, like a single click.
            let Some(id) = self.find_object_at(doc_pos, state) else {
                if !(shift_held || cmd_held) {
                    state.clear_selection();
                }
                return;
            };
            {
                state.select(id, false);
                if matches!(state.current_page().and_then(|p| p.get_shape(id)).map(|s| &s.kind), Some(ShapeKind::Table(_))) {
                    // Double-click a table cell to edit its text
                    let now = response.ctx.input(|i| i.time);
                    text_edit::begin_cell_at_point(state, id, doc_pos.x as f32, doc_pos.y as f32, now);
                    state.current_tool = Tool::Select;
                } else if matches!(state.current_page().and_then(|p| p.get_shape(id)).map(|s| &s.kind), Some(ShapeKind::TextEffect(_))) {
                    state.edit_wordart_requested = true;
                } else if text_edit::is_text_frame(state, id) {
                    let now = response.ctx.input(|i| i.time);
                    text_edit::begin_at_point(state, id, doc_pos.x as f32, doc_pos.y as f32, now);
                    // Double-click lands on a word: select it, as text editors do
                    // (a linked box edits its chain's first box: caret only)
                    if state.text_edit.as_ref().is_some_and(|e| e.frame == id) {
                        text_edit::select_word_at(state, doc_pos.x as f32, doc_pos.y as f32, now);
                    }
                }
            }
            return;
        }
        
        // Plain click (pointer released without passing the drag threshold).
        // egui reports these as `clicked()`, never as `drag_started()`.
        if response.clicked() {
            // Create Link: the next click picks the box the text continues in
            if let Some(from) = state.link_source.take() {
                self.finish_link(from, doc_pos, state);
                return;
            }
            // A click on the overflow mark of the selected text box starts linking
            if let (Some(pos), [id]) = (response.interact_pointer_pos(), &state.selection[..]) {
                let id = *id;
                if let Some(shape) = state.current_page().and_then(|p| p.get_shape(id)) {
                  if let ShapeKind::TextFrame(data) = &shape.kind {
                    // The box as drawn (its own, or its chain's shown part)
                    let shown = self.chain_frames.get(&id).map(|c| &c.frame);
                    let tf: &Shape = shown.unwrap_or(shape);
                    if data.linked_frame.is_none()
                        && overflow_mark_rect(tf, page_rect.min, state.zoom).contains(pos)
                        && crate::text_chain::overflow_pos(tf).is_some()
                    {
                        text_edit::end(state);
                        state.link_source = Some(id);
                        return;
                    }
                  }
                }
            }
            // Format Painter: the next click applies the copied formatting.
            if state.is_format_painter_active() {
                if let Some(id) = self.find_object_at(doc_pos, state) {
                    state.record_undo("Format Painter");
                    state.apply_copied_format_to(id);
                    state.deactivate_format_painter();
                }
                return;
            }
            match self.hit_test_handles(doc_pos, page_rect, state) {
                // Clicking a resize/rotation handle without dragging keeps the selection
                Some(HandleType::Resize(_)) | Some(HandleType::Rotation) | Some(HandleType::Adjust(_)) => {}
                _ => match self.find_object_at(doc_pos, state) {
                    Some(id) if shift_held || cmd_held => state.toggle_selection(id),
                    Some(id) => state.select(id, false),
                    // Click on empty canvas or off the page: deselect
                    None if !(shift_held || cmd_held) => state.clear_selection(),
                    None => {}
                },
            }
            return;
        }
        
        // A drag starts where the button was pressed, not where the pointer is once
        // it crosses the drag threshold, so hit-test at the press origin.
        let doc_pos = if response.drag_started() {
            response
                .ctx
                .input(|i| i.pointer.press_origin())
                .map(|p| self.screen_to_doc_point(page_rect, p, state))
                .unwrap_or(doc_pos)
        } else {
            doc_pos
        };
        
        if response.drag_started() {
            // First check if dragging a guide
            if state.show_guides && !state.guides_locked {
                let tolerance = metrics::GUIDE_HIT_TOLERANCE as f64 / state.zoom as f64;
                if let Some((guide_idx, _)) = state.find_guide_at(doc_pos, tolerance) {
                    if !state.guides()[guide_idx].locked {
                        self.guide_state.dragging_guide = Some(guide_idx);
                        return;
                    }
                }
            }
            
            // If not dragging a guide, proceed with normal selection handling.
            // Snapshot for undo; recorded at drag end only if something changed.
            state.begin_edit();
            // First check if we're clicking on a selection handle
            if let Some(handle_type) = self.hit_test_handles(doc_pos, page_rect, state) {
                match handle_type {
                    HandleType::Resize(handle) => {
                        // Start resizing
                        if let Some(frame) = self.selection_frame(state) {
                            self.select_state.start_resize(handle, frame.rect);
                            self.select_state.resize_rotation = frame.rotation;
                            self.select_state.start_drag(doc_pos);
                        }
                    }
                    HandleType::Rotation => {
                        // Start rotating
                        if let Some(bounds) = self.get_selection_bounds(state) {
                            let center = Point::new(
                                bounds.origin.x + bounds.width() / 2.0,
                                bounds.origin.y + bounds.height() / 2.0,
                            );
                            let angle = (doc_pos.y - center.y).atan2(doc_pos.x - center.x);
                            self.select_state.start_rotation(center, angle, 0.0);
                        }
                    }
                    HandleType::Body => {
                        // Start dragging (handled below)
                        self.select_state.start_drag(doc_pos);
                    }
                    HandleType::Adjust(index) => self.adjusting = Some(index),
                }
            } else {
                // Check if clicking on an object
                let clicked_obj = self.find_object_at(doc_pos, state);
                
                if let Some(id) = clicked_obj {
                    if shift_held || cmd_held {
                        // Toggle selection
                        state.toggle_selection(id);
                    } else if !state.is_selected(id) {
                        // Select only this object
                        state.select(id, false);
                    }
                    self.select_state.start_drag(doc_pos);
                } else {
                    // Click on empty space
                    state.clear_selection();
                    self.select_state.start_marquee(doc_pos);
                }
            }
            self.capture_gesture_origins(state);
        } else if response.dragged() {
            // Handle guide dragging first
            if let Some(guide_idx) = self.guide_state.dragging_guide {
                if guide_idx < state.guides().len() {
                    let guide = &state.guides()[guide_idx];
                    let new_pos = match guide.orientation {
                        GuideOrientation::Horizontal => doc_pos.y,
                        GuideOrientation::Vertical => doc_pos.x,
                    };
                    state.update_guide(guide_idx, new_pos);
                }
            } else if let Some(index) = self.adjusting {
                // Reshape live: the handle's adjust values follow the pointer
                if let [id] = state.selection[..] {
                    if let Some(shape) = state.current_page_mut().and_then(|p| p.get_shape_mut(id)) {
                        let local = shape_frame_point(shape, doc_pos);
                        let bounds = shape.common.bounds;
                        if let ShapeKind::AutoShape(auto) = &mut shape.kind {
                            use repub_escher::PresetExt;
                            let changed = auto.preset.as_mut().is_some_and(|g| g.drag_handle(index, bounds, local));
                            // Rebuild the preset's outline for the new adjust values
                            if changed {
                                if let Some(path) = auto.preset.as_ref().and_then(|g| g.path(bounds)) {
                                    auto.geometry = repub_model::geometry::ShapeGeometry::Path(path);
                                }
                            }
                        }
                    }
                }
            } else if self.select_state.is_marquee {
                self.select_state.update_marquee(doc_pos);
            } else if self.select_state.is_rotating {
                // Update rotation
                if let Some(center) = self.select_state.rotation_center {
                    let angle = (doc_pos.y - center.y).atan2(doc_pos.x - center.x);
                    self.select_state.update_rotation(angle);
                    // Apply live; Shift snaps the resulting angle to 15°
                    if let Some(delta) = self.select_state.rotation_delta() {
                        self.apply_rotation(delta, shift_held, state);
                    }
                }
            } else if self.select_state.is_resizing {
                self.select_state.update_drag(doc_pos);
                // Apply resize in real-time
                let alt_held = response.ctx.input(|i| i.modifiers.alt);
                self.apply_resize(doc_pos, shift_held, alt_held, state);
            } else if self.select_state.is_dragging {
                self.select_state.update_drag(doc_pos);
                // Apply move in real-time
                self.apply_drag(state);
            }
        } else if response.drag_stopped() {
            // Clear snap indicators when drag ends
            self.snap_guides.clear();
            let undo_label = if self.adjusting.take().is_some() {
                "Adjust Shape"
            } else if self.select_state.is_rotating {
                "Rotate"
            } else if self.select_state.is_resizing {
                "Resize"
            } else {
                "Move"
            };
            
            // Handle guide drag stop
            if self.guide_state.dragging_guide.is_some() {
                self.guide_state.dragging_guide = None;
            } else if self.select_state.is_marquee {
                // Direction decides the mode; read the start before end_marquee() clears it
                let left_to_right = self.select_state.marquee_start
                    .map(|s| s.x < doc_pos.x)
                    .unwrap_or(true);
                if let Some(marquee) = self.select_state.end_marquee() {
                    self.select_objects_in_rect(&marquee, left_to_right, state);
                }
            } else if self.select_state.is_rotating {
                if let Some(_delta) = self.select_state.end_rotation() {
                    state.mark_dirty();
                }
            } else if self.select_state.is_resizing {
                self.select_state.end_resize();
                self.select_state.end_drag();
                state.mark_dirty();
            } else if self.select_state.is_dragging {
                if let Some(delta) = self.select_state.end_drag() {
                    if delta.x.abs() > 1.0 || delta.y.abs() > 1.0 {
                        state.mark_dirty();
                    }
                }
            }
            // One undo step for the whole drag (nothing recorded if nothing changed)
            state.end_edit(undo_label);
        }
    }

    /// Hit test selection handles.
    fn hit_test_handles(&self, doc_pos: Point, _page_rect: Rect, state: &AppState) -> Option<HandleType> {
        if state.selection.is_empty() {
            return None;
        }

        // A preset shape's adjust handles sit on top of everything
        if let [shape] = state.selected_objects()[..] {
            let reach = metrics::HANDLE_SIZE as f64 / state.zoom as f64;
            for (i, h) in adjust_handles(shape).into_iter().enumerate() {
                if (h.x - doc_pos.x).hypot(h.y - doc_pos.y) <= reach {
                    return Some(HandleType::Adjust(i));
                }
            }
        }

        // Test in the selection frame's own (unturned) axes
        let frame = self.selection_frame(state)?;
        let bounds = frame.rect;
        let doc_pos = frame.to_local(doc_pos);
        let handle_size = metrics::HANDLE_SIZE as f64 / state.zoom as f64;
        let rotation_distance = metrics::ROTATION_HANDLE_DISTANCE as f64 / state.zoom as f64;
        let rotation_size = metrics::ROTATION_HANDLE_SIZE as f64 / state.zoom as f64;

        // Check rotation handle first (it's on top)
        let top_center = Point::new(
            bounds.origin.x + bounds.width() / 2.0,
            bounds.origin.y,
        );
        let rotation_pos = Point::new(top_center.x, top_center.y - rotation_distance);
        let dist_to_rotation = ((doc_pos.x - rotation_pos.x).powi(2) + (doc_pos.y - rotation_pos.y).powi(2)).sqrt();
        if dist_to_rotation <= rotation_size {
            return Some(HandleType::Rotation);
        }

        // Check resize handles
        for handle in ResizeHandle::all() {
            let handle_pos = handle.position(&bounds);
            let handle_rect = repub_geometry::Rect::new(
                handle_pos.x - handle_size / 2.0,
                handle_pos.y - handle_size / 2.0,
                handle_size,
                handle_size,
            );
            if handle_rect.contains(doc_pos) {
                return Some(HandleType::Resize(handle));
            }
        }

        // Check if inside selection bounds (for moving)
        if bounds.contains(doc_pos) {
            return Some(HandleType::Body);
        }

        None
    }

    /// The box the selection handles sit on: a single object's own box, turned
    /// with it; for several objects, the upright box around them as drawn.
    fn selection_frame(&self, state: &AppState) -> Option<SelectionFrame> {
        let objects = state.selected_objects();
        if let [obj] = objects[..] {
            return Some(SelectionFrame { rect: obj.bounds(), rotation: obj.common.rotation });
        }
        objects
            .iter()
            .map(|o| SelectionFrame { rect: o.bounds(), rotation: o.common.rotation }.page_bounds())
            .reduce(|a, b| a.union(&b))
            .map(|rect| SelectionFrame { rect, rotation: 0.0 })
    }

    /// Get combined bounds of all selected objects (the selection frame's box).
    fn get_selection_bounds(&self, state: &AppState) -> Option<repub_geometry::Rect> {
        self.selection_frame(state).map(|f| f.rect)
    }

    /// Apply drag movement to selected objects.
    fn apply_drag(&mut self, state: &mut AppState) {
        let delta = match self.select_state.drag_delta() {
            Some(d) => d,
            None => return,
        };
        
        // Snap and move from where the objects were when the drag began
        let mut combined_bounds = self.gesture_origins.iter().map(|(_, b)| *b).reduce(|a, b| a.union(&b));
        
        // Apply snap if we have bounds
        let snapped_delta = if let (Some(engine), Some(bounds)) = (self.create_snap_engine(state), combined_bounds.as_mut()) {
            // Calculate where the bounds would move to
            let moved_bounds = repub_geometry::Rect::new(
                bounds.origin.x + delta.x,
                bounds.origin.y + delta.y,
                bounds.width(),
                bounds.height(),
            );
            
            // Snap the moved bounds
            let (snapped_pos, snaps) = engine.snap_move(&moved_bounds);
            
            // Store snap guides for visualization
            self.snap_guides.clear();
            self.snap_guides.extend(snaps);
            
            // Calculate the actual snapped delta
            Point::new(
                snapped_pos.x - bounds.origin.x,
                snapped_pos.y - bounds.origin.y,
            )
        } else {
            delta
        };
        
        if let Some(page) = state.current_page_mut() {
            for (id, origin) in &self.gesture_origins {
                if let Some(obj) = page.get_shape_mut(*id) {
                    obj.set_position(Point::new(origin.origin.x + snapped_delta.x, origin.origin.y + snapped_delta.y));
                }
            }
        }
    }

    /// Remember the selected objects' bounds at the start of a move/resize.
    /// Locked objects (or objects on locked layers) aren't moved, resized or rotated.
    fn capture_gesture_origins(&mut self, state: &AppState) {
        let objects: Vec<&Shape> = state
            .selection
            .iter()
            .filter(|id| !self.is_object_locked(**id, state))
            .filter_map(|id| state.current_page()?.get_shape(*id))
            .collect();
        self.gesture_origins = objects.iter().map(|o| (o.id(), o.bounds())).collect();
        self.gesture_rotations = objects.iter().map(|o| o.common.rotation).collect();
        self.gesture_groups = objects.iter().filter(|o| matches!(o.kind, ShapeKind::Group(_))).map(|o| (*o).clone()).collect();
    }

    /// Rotate the selection by `delta` radians from where the gesture began:
    /// one object about its centre, several together about their common centre.
    /// `snap` rounds the result to 15°.
    fn apply_rotation(&self, delta: f64, snap: bool, state: &mut AppState) {
        let Some(first) = self.gesture_rotations.first() else { return };
        let mut degrees = delta.to_degrees();
        if snap {
            degrees = ((first + degrees) / 15.0).round() * 15.0 - first;
        }
        let Some(all) = self.gesture_origins.iter().map(|(_, b)| *b).reduce(|a, b| a.union(&b)) else { return };
        let pivot = all.center();
        let (sin, cos) = degrees.to_radians().sin_cos();
        let Some(page) = state.current_page_mut() else { return };
        for ((id, bounds), start) in self.gesture_origins.iter().zip(&self.gesture_rotations) {
            let Some(obj) = page.get_shape_mut(*id) else { continue };
            if let Some(original) = self.gesture_groups.iter().find(|g| g.id() == *id) {
                *obj = original.clone();
                rotate_about(obj, pivot, degrees);
                continue;
            }
            // Turn each centre about the pivot (a single object's centre is the pivot)
            let c = bounds.center();
            let (dx, dy) = (c.x - pivot.x, c.y - pivot.y);
            let centre = Point::new(pivot.x + dx * cos - dy * sin, pivot.y + dx * sin + dy * cos);
            obj.set_position(Point::new(centre.x - bounds.size.width / 2.0, centre.y - bounds.size.height / 2.0));
            obj.common.rotation = (start + degrees).rem_euclid(360.0);
        }
    }

    /// Apply resize to selected objects.
    fn apply_resize(&self, doc_pos: Point, constrain: bool, from_center: bool, state: &mut AppState) {
        let handle = match self.select_state.resize_handle {
            Some(h) => h,
            None => return,
        };
        
        let original = match self.select_state.original_bounds {
            Some(b) => b,
            None => return,
        };
        
        let drag_start = match self.select_state.drag_start {
            Some(p) => p,
            None => return,
        };

        // The pointer's movement in the box's own (unturned) axes
        let rotation = self.select_state.resize_rotation;
        let (sin, cos) = (-rotation).to_radians().sin_cos();
        let (mx, my) = (doc_pos.x - drag_start.x, doc_pos.y - drag_start.y);
        let (dx, dy) = (mx * cos - my * sin, mx * sin + my * cos);

        // Calculate new bounds based on handle
        let (mut new_x, mut new_y, mut new_w, mut new_h) = (
            original.origin.x,
            original.origin.y,
            original.width(),
            original.height(),
        );

        match handle {
            ResizeHandle::TopLeft => {
                new_x += dx;
                new_y += dy;
                new_w -= dx;
                new_h -= dy;
            }
            ResizeHandle::TopCenter => {
                new_y += dy;
                new_h -= dy;
            }
            ResizeHandle::TopRight => {
                new_y += dy;
                new_w += dx;
                new_h -= dy;
            }
            ResizeHandle::MiddleLeft => {
                new_x += dx;
                new_w -= dx;
            }
            ResizeHandle::MiddleRight => {
                new_w += dx;
            }
            ResizeHandle::BottomLeft => {
                new_x += dx;
                new_w -= dx;
                new_h += dy;
            }
            ResizeHandle::BottomCenter => {
                new_h += dy;
            }
            ResizeHandle::BottomRight => {
                new_w += dx;
                new_h += dy;
            }
        }

        // Constrain proportions if shift held
        if constrain && original.width() > 1e-6 && original.height() > 1e-6 {
            let orig_ratio = original.width() / original.height();
            if new_w / new_h > orig_ratio {
                new_h = new_w / orig_ratio;
            } else {
                new_w = new_h * orig_ratio;
            }
        }

        // Resize from center if alt held
        if from_center {
            let center_x = original.origin.x + original.width() / 2.0;
            let center_y = original.origin.y + original.height() / 2.0;
            new_x = center_x - new_w / 2.0;
            new_y = center_y - new_h / 2.0;
        }

        // Minimum size; a flat box (a horizontal or vertical line) stays flat on that axis
        let (flat_w, flat_h) = (original.width() <= 1e-6, original.height() <= 1e-6);
        if flat_w {
            new_w = 0.0;
            new_x = original.origin.x;
        } else {
            new_w = new_w.max(10.0);
        }
        if flat_h {
            new_h = 0.0;
            new_y = original.origin.y;
        } else {
            new_h = new_h.max(10.0);
        }

        // A turned box: the new box was worked out in its own axes; put its centre
        // back in page space so the opposite edge stays where it is
        if rotation != 0.0 {
            let frame = SelectionFrame { rect: original, rotation };
            let centre = frame.to_page(Point::new(new_x + new_w / 2.0, new_y + new_h / 2.0));
            new_x = centre.x - new_w / 2.0;
            new_y = centre.y - new_h / 2.0;
        }

        // Apply to selected objects (scale relative to original bounds)
        let scale_x = if flat_w { 1.0 } else { new_w / original.width() };
        let scale_y = if flat_h { 1.0 } else { new_h / original.height() };

        // Scale each object from its bounds at the start of the resize
        if let Some(page) = state.current_page_mut() {
            for (id, obj_bounds) in &self.gesture_origins {
                let Some(obj) = page.get_shape_mut(*id) else { continue };
                // Position relative to the selection origin, scaled
                let rel_x = obj_bounds.origin.x - original.origin.x;
                let rel_y = obj_bounds.origin.y - original.origin.y;
                obj.set_position(Point::new(new_x + rel_x * scale_x, new_y + rel_y * scale_y));
                obj.set_size(obj_bounds.width() * scale_x, obj_bounds.height() * scale_y);
            }
        }
    }

    fn handle_pan_input(&mut self, response: &egui::Response, state: &mut AppState) {
        if response.dragged() {
            let delta = response.drag_delta();
            state.pan_offset.x += delta.x as f64;
            state.pan_offset.y += delta.y as f64;
        }
    }

    fn handle_zoom_input(&mut self, response: &egui::Response, canvas_rect: Rect, state: &mut AppState) {
        let shift_held = response.ctx.input(|i| i.modifiers.shift);
        
        // Double-click to fit to window
        if response.double_clicked() {
            let viewport_width = canvas_rect.width();
            let viewport_height = canvas_rect.height();
            state.zoom_to_fit(viewport_width, viewport_height);
            state.pan_offset = Point::new(0.0, 0.0);
            return;
        }
        
        // Single click to zoom in, Shift+click to zoom out, about the clicked point
        if response.clicked() {
            let anchor = response.interact_pointer_pos().unwrap_or(canvas_rect.center());
            let factor = if shift_held { 1.0 / 1.25 } else { 1.25 };
            zoom_about(state, canvas_rect, anchor, state.zoom * factor);
        }
    }

    /// Enter finishes a Freeform outline (open), Escape drops it.
    fn handle_pen_keys(&mut self, ctx: &egui::Context, state: &mut AppState) {
        if !matches!(state.current_tool, Tool::Freeform | Tool::Pencil) {
            // Choosing another tool abandons an outline in progress
            self.pen_points.clear();
        }
        if state.current_tool != Tool::Freeform || self.pen_points.is_empty() {
            return;
        }
        let (enter, escape) = ctx.input(|i| (i.key_pressed(egui::Key::Enter), i.key_pressed(egui::Key::Escape)));
        if enter {
            self.finish_pen(false, state);
        } else if escape {
            self.pen_points.clear();
            state.current_tool = Tool::Select;
        }
    }

    /// Freeform: click to place points (the first point, or a double-click, finishes).
    /// Pencil: drag to draw a smooth curve.
    fn handle_pen_input(&mut self, response: &egui::Response, doc_pos: Point, state: &mut AppState) {
        let pos = match self.create_snap_engine(state) {
            Some(engine) => engine.snap_point(doc_pos).0,
            None => doc_pos,
        };
        if state.current_tool == Tool::Pencil {
            if response.drag_started() {
                self.pen_points.clear();
            }
            if response.dragged() {
                let min_step = 1.0 / f64::from(state.zoom.max(0.01));
                if self.pen_points.last().map_or(true, |q| (q.x - doc_pos.x).hypot(q.y - doc_pos.y) >= min_step) {
                    self.pen_points.push(doc_pos);
                }
            }
            if response.drag_stopped() {
                let points = std::mem::take(&mut self.pen_points);
                if let Some(path) = crate::tools::smooth_path(&points, 2.0 / f64::from(state.zoom.max(0.01))) {
                    self.add_drawn_path(path, "Draw Curve", state);
                }
                state.current_tool = Tool::Select;
            }
            return;
        }
        // Freeform
        if response.double_clicked() {
            self.finish_pen(false, state);
        } else if response.clicked() {
            let close_distance = 8.0 / f64::from(state.zoom.max(0.01));
            let near_start = self.pen_points.len() >= 3 && self.pen_points.first().is_some_and(|f| (f.x - pos.x).hypot(f.y - pos.y) <= close_distance);
            if near_start {
                self.finish_pen(true, state);
            } else {
                self.pen_points.push(pos);
            }
        }
    }

    fn finish_pen(&mut self, closed: bool, state: &mut AppState) {
        let points = std::mem::take(&mut self.pen_points);
        if let Some(path) = crate::tools::path_from_points(&points, closed) {
            self.add_drawn_path(path, "Draw Freeform", state);
        }
        state.current_tool = Tool::Select;
    }

    fn add_drawn_path(&mut self, path: repub_model::geometry::Path, label: &str, state: &mut AppState) {
        let Some(shape) = crate::tools::shape_from_path(path) else { return };
        state.record_undo(label);
        let id = shape.id();
        if let Some(page) = state.current_page_mut() {
            page.add_shape(shape);
        }
        state.select(id, false);
        state.mark_dirty();
    }

    /// The outline being drawn with Freeform or Pencil, with the rubber band to the pointer.
    fn draw_pen_preview(&self, painter: &egui::Painter, page_rect: Rect, state: &AppState) {
        if self.pen_points.is_empty() {
            return;
        }
        let stroke = Stroke::new(1.5, colors::accent());
        let mut pts: Vec<Pos2> = self.pen_points.iter().map(|p| self.doc_to_screen_point(page_rect, p, state)).collect();
        if state.current_tool == Tool::Freeform {
            if let Some(hover) = painter.ctx().input(|i| i.pointer.hover_pos()) {
                pts.push(hover);
            }
        }
        painter.add(egui::Shape::line(pts.clone(), stroke));
        for p in pts.iter().take(self.pen_points.len()) {
            painter.circle(*p, 3.0, egui::Color32::WHITE, stroke);
        }
        painter.ctx().request_repaint();
    }

    fn handle_draw_input(&mut self, response: &egui::Response, doc_pos: Point, state: &mut AppState) {
        // Apply snapping to draw positions
        let snapped_pos = if let Some(engine) = self.create_snap_engine(state) {
            let (snapped, h_snap, v_snap) = engine.snap_point(doc_pos);
            // Store snap guides for visualization
            self.snap_guides.clear();
            if let Some(s) = h_snap {
                self.snap_guides.push(s);
            }
            if let Some(s) = v_snap {
                self.snap_guides.push(s);
            }
            snapped
        } else {
            doc_pos
        };

        let now = response.ctx.input(|i| i.time);

        // Text tool: clicking an existing text frame edits it; clicking anywhere
        // else makes a default-size text box there, ready to type into
        if state.current_tool == Tool::Text && response.clicked() {
            state.current_tool = Tool::Select;
            if let Some(id) = self.find_object_at(doc_pos, state).filter(|&id| text_edit::is_text_frame(state, id)) {
                text_edit::begin_at_point(state, id, doc_pos.x as f32, doc_pos.y as f32, now);
            } else {
                self.create_default_text_box(state, snapped_pos, now);
            }
            return;
        }

        if response.drag_started() {
            // Start where the button went down, not where the pointer crossed the
            // drag threshold (otherwise every shape loses a few pixels at its start)
            let page_rect = self.last_page_rect.unwrap_or(Rect::NOTHING);
            let origin = response
                .ctx
                .input(|i| i.pointer.press_origin())
                .map(|p| self.screen_to_doc_point(page_rect, p, state))
                .unwrap_or(doc_pos);
            let origin = match self.create_snap_engine(state) {
                Some(engine) => engine.snap_point(origin).0,
                None => origin,
            };
            self.draw_state.start(origin);
        } else if response.dragged() {
            self.draw_state.update(snapped_pos);
        } else if response.drag_stopped() {
            let tool = state.current_tool;
            if let Some(obj) = self.draw_state.create_object(tool) {
                let label = format!("Draw {}", tool.name());
                state.record_undo(&label);
                let id = obj.id();
                if let Some(page) = state.current_page_mut() {
                    page.add_shape(obj);
                }
                state.select(id, false);
                state.mark_dirty();
                // A new text box is ready to type into, as in Publisher
                if tool == Tool::Text {
                    text_edit::begin(state, id, Some(text_edit::TextPos::default()), now);
                }
            } else if tool == Tool::Text {
                // The drag was too small to size a box (a tap that egui reported
                // as a tiny drag): make a default-size text box at the start,
                // ready to type into — same as a plain click with the Text tool.
                let origin = self.draw_state.start_point.unwrap_or(snapped_pos);
                self.create_default_text_box(state, origin, now);
            }
            self.draw_state.end();
            self.snap_guides.clear();
            
            // Switch back to select tool after creating
            state.current_tool = Tool::Select;
        }
    }

    /// Create a default-size text box at `origin` (document points), select it,
    /// and begin editing — the behaviour of a click (or a sizeless drag) with
    /// the Text tool.
    fn create_default_text_box(&mut self, state: &mut AppState, origin: Point, now: f64) {
        const DEFAULT_W: f64 = 144.0;
        const DEFAULT_H: f64 = 36.0;
        let frame = repub_model::shape::Shape::text_frame(repub_geometry::Rect::new(
            origin.x, origin.y, DEFAULT_W, DEFAULT_H,
        ));
        let id = frame.common.id;
        state.record_undo("Draw Text Box");
        if let Some(page) = state.current_page_mut() {
            page.add_shape(frame);
        }
        state.select(id, false);
        state.mark_dirty();
        text_edit::begin(state, id, Some(text_edit::TextPos::default()), now);
    }

    /// The object a click at `pos` selects: visible content first, with a few
    /// pixels of slack (see [`crate::hit_test`]).
    fn find_object_at(&self, pos: Point, state: &AppState) -> Option<repub_model::ObjectId> {
        let page = state.current_page()?;
        let tolerance = (crate::hit_test::HIT_TOLERANCE_PX / state.zoom) as f64;
        let id = crate::hit_test::object_at(page, pos, tolerance)?;
        // A click on a group that is already selected (or has a selected member)
        // picks the member under the pointer, as in Publisher
        let mut current = page.get_shape(id)?;
        while let ShapeKind::Group(group) = &current.kind {
            let selected_here = state.selection.iter().any(|s| current.find(*s).is_some());
            if !selected_here {
                break;
            }
            let Some(member) = crate::hit_test::member_at(group, pos, tolerance) else { break };
            if state.selection.contains(&member.id()) {
                return Some(member.id());
            }
            current = member;
        }
        Some(current.id())
    }

    /// Check if an object (or its layer) is locked.
    fn is_object_locked(&self, obj_id: repub_model::ObjectId, state: &AppState) -> bool {
        let Some(page) = state.current_page() else { return false };

        for layer in page.layers() {
            // Check if the layer itself is locked
            if layer.locked {
                for obj in layer.shapes() {
                    if obj.id() == obj_id {
                        return true;
                    }
                }
            }
            
            // Check if the object is locked
            for obj in layer.shapes() {
                if obj.id() == obj_id {
                    return obj.common.locked;
                }
            }
        }

        false
    }

    /// Select objects in a marquee rectangle.
    /// If `fully_inside` is true, only selects objects completely inside.
    /// If false, selects objects that touch the rectangle.
    fn select_objects_in_rect(&self, rect: &repub_geometry::Rect, fully_inside: bool, state: &mut AppState) {
        let Some(page) = state.current_page() else { return };
        let ids: Vec<_> = page
            .layers()
            .iter()
            .filter(|l| l.visible)
            .flat_map(|l| l.shapes())
            .filter(|o| o.common.visible)
            .filter(|obj| {
                let bounds = obj.bounds();
                if fully_inside { rect.contains_rect(&bounds) } else { rect.intersects(&bounds) }
            })
            .map(|obj| obj.id())
            .collect();
        state.selection = ids;
    }

    fn doc_to_screen_point(&self, page_rect: Rect, point: &Point, state: &AppState) -> Pos2 {
        Pos2::new(
            page_rect.left() + (point.x as f32) * state.zoom,
            page_rect.top() + (point.y as f32) * state.zoom,
        )
    }

    fn doc_to_screen_rect(&self, page_rect: Rect, bounds: &repub_geometry::Rect, state: &AppState) -> Rect {
        Rect::from_min_size(
            Pos2::new(
                page_rect.left() + (bounds.origin.x as f32) * state.zoom,
                page_rect.top() + (bounds.origin.y as f32) * state.zoom,
            ),
            Vec2::new(
                bounds.width() as f32 * state.zoom,
                bounds.height() as f32 * state.zoom,
            ),
        )
    }

    fn screen_to_doc_point(&self, page_rect: Rect, pos: Pos2, state: &AppState) -> Point {
        Point::new(
            ((pos.x - page_rect.left()) / state.zoom) as f64,
            ((pos.y - page_rect.top()) / state.zoom) as f64,
        )
    }
}

/// Where the pages on view sit on screen: one page, or a two-page spread with
/// the pages touching at the spine. Shared by the canvas, rulers and scroll bars.
#[derive(Debug, Clone, Default)]
pub struct PageLayout {
    /// Page index and screen rect, left to right.
    pub pages: Vec<(usize, Rect)>,
}

impl PageLayout {
    /// Lay out `state.visible_pages()` centered in `canvas_rect`, offset by the pan.
    pub fn compute(canvas_rect: Rect, state: &AppState) -> Self {
        let sizes: Vec<(usize, Vec2)> = state
            .visible_pages()
            .into_iter()
            .filter_map(|i| {
                let p = state.page_on_view(i)?;
                Some((i, Vec2::new(p.size.width as f32, p.size.height as f32) * state.zoom))
            })
            .collect();
        let total_width: f32 = sizes.iter().map(|(_, s)| s.x).sum();
        let max_height = sizes.iter().map(|(_, s)| s.y).fold(0.0, f32::max);
        let mut x = canvas_rect.center().x - total_width / 2.0 + state.pan_offset.x as f32;
        let top = canvas_rect.center().y - max_height / 2.0 + state.pan_offset.y as f32;
        let pages = sizes
            .into_iter()
            .map(|(i, size)| {
                let r = Rect::from_min_size(Pos2::new(x, top), size);
                x += size.x;
                (i, r)
            })
            .collect();
        Self { pages }
    }

    /// Screen rect of page `index`, if it is on view.
    pub fn rect_of(&self, index: usize) -> Option<Rect> {
        self.pages.iter().find(|(i, _)| *i == index).map(|(_, r)| *r)
    }

    /// Union of all page rects on view.
    pub fn bounds(&self) -> Option<Rect> {
        self.pages.iter().map(|(_, r)| *r).reduce(|a, b| a.union(b))
    }

    /// Index of the page under `pos`, if any.
    pub fn page_at(&self, pos: Pos2) -> Option<usize> {
        self.pages.iter().find(|(_, r)| r.contains(pos)).map(|(i, _)| *i)
    }
}

/// Set the zoom (clamped to 10%–800%), adjusting the pan so the document point
/// under `anchor` (screen) stays under it.
///
/// The pages on view are centered at `canvas_rect.center() + pan`, so a screen
/// point `p` sits `(p - center - pan) / zoom` points from the spread's center.
/// Keeping that fixed gives `pan' = (p - center) * (1 - z'/z) + pan * z'/z`.
pub fn zoom_about(state: &mut AppState, canvas_rect: Rect, anchor: Pos2, new_zoom: f32) {
    let new_zoom = new_zoom.clamp(0.1, 8.0);
    let ratio = (new_zoom / state.zoom) as f64;
    let offset = anchor - canvas_rect.center();
    state.pan_offset.x = offset.x as f64 * (1.0 - ratio) + state.pan_offset.x * ratio;
    state.pan_offset.y = offset.y as f64 * (1.0 - ratio) + state.pan_offset.y * ratio;
    state.zoom = new_zoom;
}

/// An image uploaded at full size plus successively halved copies (a mip chain).
///
/// egui-wgpu creates textures without mipmaps, so an image drawn much smaller
/// than its pixel size skips most source pixels and aliases (jagged edges,
/// shimmering detail). Drawing from the level nearest the on-screen size keeps
/// the GPU's linear filter within 2x minification, where it averages properly.
#[derive(Clone)]
struct ImageMips {
    /// Level 0 is full size; each next level is half the previous.
    pub(crate) levels: Vec<TextureHandle>,
}

impl ImageMips {
    /// Smallest level worth keeping (longest side, in pixels).
    const MIN_LEVEL_SIZE: u32 = 8;

    fn upload(ctx: &egui::Context, name: &str, image: image::RgbaImage) -> Self {
        let mut levels = Vec::new();
        let mut current = image;
        loop {
            let size = [current.width() as usize, current.height() as usize];
            let color_image = egui::ColorImage::from_rgba_unmultiplied(size, current.as_raw());
            levels.push(ctx.load_texture(format!("{name}#{}", levels.len()), color_image, egui::TextureOptions::LINEAR));
            if current.width().max(current.height()) / 2 < Self::MIN_LEVEL_SIZE {
                break;
            }
            current = halve_image(&current);
        }
        Self { levels }
    }

    /// The smallest level that still has at least `screen_px` pixels on both
    /// axes (physical pixels), or full size when drawn larger than that.
    fn for_screen_size(&self, screen_px: Vec2) -> &TextureHandle {
        self.levels
            .iter()
            .rev()
            .find(|t| {
                let [w, h] = t.size();
                w as f32 >= screen_px.x && h as f32 >= screen_px.y
            })
            .unwrap_or(&self.levels[0])
    }
}

/// Downscale by half with a 2x2 box filter. Colors are weighted by alpha, so
/// transparent pixels don't darken the edges of what's around them. An odd
/// last row/column is folded into its neighbour.
fn halve_image(src: &image::RgbaImage) -> image::RgbaImage {
    let (sw, sh) = src.dimensions();
    let (dw, dh) = ((sw / 2).max(1), (sh / 2).max(1));
    image::RgbaImage::from_fn(dw, dh, |x, y| {
        let mut sum = [0u32; 4];
        for sy in [2 * y, (2 * y + 1).min(sh - 1)] {
            for sx in [2 * x, (2 * x + 1).min(sw - 1)] {
                let [r, g, b, a] = src.get_pixel(sx, sy).0;
                let a = a as u32;
                sum[0] += r as u32 * a;
                sum[1] += g as u32 * a;
                sum[2] += b as u32 * a;
                sum[3] += a;
            }
        }
        if sum[3] == 0 {
            return image::Rgba([0, 0, 0, 0]);
        }
        let channel = |c: u32| ((c + sum[3] / 2) / sum[3]) as u8;
        image::Rgba([channel(sum[0]), channel(sum[1]), channel(sum[2]), ((sum[3] + 2) / 4) as u8])
    })
}

/// Mapping from document points to screen pixels: `screen = origin + doc * scale`.
/// Shared by the canvas and the page thumbnails so both draw identical geometry.
#[derive(Debug, Clone, Copy)]
pub struct DocToScreen {
    pub origin: Pos2,
    pub scale: f32,
}

impl DocToScreen {
    pub fn point(&self, x: f64, y: f64) -> Pos2 {
        Pos2::new(
            self.origin.x + x as f32 * self.scale,
            self.origin.y + y as f32 * self.scale,
        )
    }

    pub fn rect(&self, r: &repub_geometry::Rect) -> Rect {
        Rect::from_min_size(
            self.point(r.origin.x, r.origin.y),
            Vec2::new(r.width() as f32 * self.scale, r.height() as f32 * self.scale),
        )
    }
}

/// Thickness of the workspace scroll bars, in points.
pub const SCROLLBAR_SIZE: f32 = 14.0;

/// A Publisher-style workspace scroll bar driving one axis of `pan`.
///
/// The scrollable extent is the page plus one viewport of margin on each side.
/// `page_len` and `view_len` are in screen pixels; `pan` is the matching
/// component of `AppState::pan_offset` (page centered when 0).
pub fn workspace_scrollbar(ui: &mut egui::Ui, rect: Rect, vertical: bool, page_len: f32, view_len: f32, pan: &mut f64) {
    let id = ui.id().with(("workspace_scrollbar", vertical));
    let response = ui.interact(rect, id, Sense::click_and_drag());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::ZERO, colors::scrollbar_track());

    let along = |v: Vec2| if vertical { v.y } else { v.x };
    let track_len = if vertical { rect.height() } else { rect.width() };
    let content = (page_len + 2.0 * view_len).max(1.0);
    let max_scroll = (page_len + view_len).max(1.0);
    let thumb_len = (track_len * view_len / content).clamp(20.0_f32.min(track_len), track_len);
    let free = (track_len - thumb_len).max(1.0);

    // Scroll position: distance of the viewport's start from the start of the content
    let scroll = ((view_len + page_len) / 2.0 - *pan as f32).clamp(0.0, max_scroll);
    let thumb_start = scroll / max_scroll * free;
    let inset = 3.0;
    let thumb_rect = if vertical {
        Rect::from_min_max(
            Pos2::new(rect.left() + inset, rect.top() + thumb_start + 1.0),
            Pos2::new(rect.right() - inset, rect.top() + thumb_start + thumb_len - 1.0),
        )
    } else {
        Rect::from_min_max(
            Pos2::new(rect.left() + thumb_start + 1.0, rect.top() + inset),
            Pos2::new(rect.left() + thumb_start + thumb_len - 1.0, rect.bottom() - inset),
        )
    };

    let mut new_scroll = scroll;
    if response.dragged() {
        new_scroll += along(response.drag_delta()) * max_scroll / free;
    } else if response.clicked() {
        if let Some(p) = response.interact_pointer_pos() {
            // Click on the track pages by most of a viewport toward the click
            let before = if vertical { p.y < thumb_rect.top() } else { p.x < thumb_rect.left() };
            let after = if vertical { p.y > thumb_rect.bottom() } else { p.x > thumb_rect.right() };
            if before {
                new_scroll -= view_len * 0.9;
            } else if after {
                new_scroll += view_len * 0.9;
            }
        }
    }
    if new_scroll != scroll {
        *pan = ((view_len + page_len) / 2.0 - new_scroll.clamp(0.0, max_scroll)) as f64;
    }

    let hovered = response.hover_pos().is_some_and(|p| thumb_rect.expand(inset).contains(p));
    let color = if response.dragged() {
        colors::scrollbar_thumb_active()
    } else if hovered {
        colors::scrollbar_thumb_hover()
    } else {
        colors::scrollbar_thumb()
    };
    let radius = (SCROLLBAR_SIZE - 2.0 * inset) / 2.0;
    painter.rect_filled(thumb_rect, Rounding::same(radius), color);
}

/// Small previous/next page button drawn under the vertical scroll bar.
/// Returns true when clicked.
pub fn page_step_button(ui: &mut egui::Ui, rect: Rect, up: bool, enabled: bool) -> bool {
    let id = ui.id().with(("page_step_button", up));
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let response = ui.interact(rect, id, sense);
    let painter = ui.painter_at(rect);
    let bg = if enabled && response.is_pointer_button_down_on() {
        colors::scrollbar_thumb()
    } else if enabled && response.hovered() {
        colors::scrollbar_thumb_hover()
    } else {
        colors::scrollbar_track()
    };
    painter.rect_filled(rect, Rounding::ZERO, bg);

    // Double chevron (⏶ / ⏷) like Publisher's page buttons
    let disabled_arrow = if colors::is_dark() { egui::Color32::from_gray(0x50) } else { egui::Color32::from_gray(0xB8) };
    let color = if enabled { colors::scrollbar_arrow() } else { disabled_arrow };
    let c = rect.center();
    let (w, h) = (3.5, 2.5);
    let dir = if up { -1.0 } else { 1.0 };
    for offset in [-2.0_f32, 2.0] {
        let tip = Pos2::new(c.x, c.y + (offset + h / 2.0) * dir);
        let left = Pos2::new(c.x - w, c.y + (offset - h / 2.0) * dir);
        let right = Pos2::new(c.x + w, c.y + (offset - h / 2.0) * dir);
        painter.add(egui::Shape::line(vec![left, tip, right], Stroke::new(1.2, color)));
    }
    response.on_hover_text(if up { "Previous Page" } else { "Next Page" }).clicked()
}

/// Draw Publisher's rotation handle: a clockwise circular arrow centered on `center`.
fn draw_rotation_glyph(painter: &egui::Painter, center: Pos2, radius: f32) {
    use std::f32::consts::{FRAC_PI_2, TAU};
    let stroke = Stroke::new(1.25, colors::rotation_glyph());
    // Soft white backing keeps the glyph legible over artwork
    painter.circle_filled(center, radius + 2.0, egui::Color32::from_white_alpha(170));

    let start = -FRAC_PI_2 + 0.55;
    let end = -FRAC_PI_2 + TAU - 0.45;
    let n = 24;
    let arc: Vec<Pos2> = (0..=n)
        .map(|i| {
            let a = start + (end - start) * i as f32 / n as f32;
            center + Vec2::new(a.cos(), a.sin()) * radius
        })
        .collect();
    painter.add(egui::Shape::line(arc, stroke));

    // Arrowhead at the end of the arc, pointing clockwise
    let tip_dir = Vec2::new(-end.sin(), end.cos());
    let radial = Vec2::new(end.cos(), end.sin());
    let tip = center + radial * radius + tip_dir * 2.5;
    let base = center + radial * radius - tip_dir * 0.5;
    let half = 2.2;
    painter.add(egui::Shape::convex_polygon(
        vec![tip, base + radial * half, base - radial * half],
        colors::rotation_glyph(),
        Stroke::NONE,
    ));
}

/// Convert a core color to egui, multiplying its alpha by `alpha`.
pub fn core_color_to_egui(c: &repub_color::Color, alpha: f32) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(
        (c.r * 255.0) as u8,
        (c.g * 255.0) as u8,
        (c.b * 255.0) as u8,
        (c.a * alpha as f64 * 255.0) as u8,
    )
}

/// The geometry of a shape: an auto shape's own geometry, or a plain rectangle
/// for every other kind (so the outline helpers can treat any shape uniformly).
fn shape_geometry(shape: &repub_model::shape::Shape) -> &repub_model::geometry::ShapeGeometry {
    use repub_model::geometry::ShapeGeometry;
    // A shared default so this can return a reference for non-auto shapes.
    static RECT: ShapeGeometry = ShapeGeometry::Rectangle { corner_radius: 0.0 };
    match &shape.kind {
        ShapeKind::AutoShape(auto) => &auto.geometry,
        _ => &RECT,
    }
}

/// Outline of a shape in document coordinates, with the object's rotation applied.
/// `scale` (screen pixels per point) only controls how finely curves are flattened.
pub fn shape_outline(shape: &repub_model::shape::Shape, scale: f32) -> Vec<Point> {
    use repub_model::geometry::{PathCommand, ShapeGeometry};
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    let bounds = shape.common.bounds;
    let (x0, y0, w, h) = (bounds.origin.x, bounds.origin.y, bounds.width(), bounds.height());
    let center = bounds.center();
    let curve_segments = |radius_pts: f64| -> usize {
        ((radius_pts * scale as f64 * 0.75) as usize).clamp(6, 48)
    };

    let geometry = shape_geometry(shape);
    let mut points: Vec<Point> = match geometry {
        ShapeGeometry::Rectangle { corner_radius } => {
            let r = corner_radius.min(w / 2.0).min(h / 2.0).max(0.0);
            if r <= 0.0 {
                vec![
                    Point::new(x0, y0),
                    Point::new(x0 + w, y0),
                    Point::new(x0 + w, y0 + h),
                    Point::new(x0, y0 + h),
                ]
            } else {
                let n = curve_segments(r) / 2 + 2;
                let corners = [
                    (x0 + w - r, y0 + r, -FRAC_PI_2),
                    (x0 + w - r, y0 + h - r, 0.0),
                    (x0 + r, y0 + h - r, FRAC_PI_2),
                    (x0 + r, y0 + r, PI),
                ];
                let mut pts = Vec::with_capacity(4 * (n + 1));
                for (cx, cy, start) in corners {
                    for i in 0..=n {
                        let a = start + FRAC_PI_2 * i as f64 / n as f64;
                        pts.push(Point::new(cx + r * a.cos(), cy + r * a.sin()));
                    }
                }
                pts
            }
        }
        ShapeGeometry::Ellipse => {
            let n = curve_segments((w + h) / 2.0) * 2;
            (0..n)
                .map(|i| {
                    let a = TAU * i as f64 / n as f64;
                    Point::new(center.x + w / 2.0 * a.cos(), center.y + h / 2.0 * a.sin())
                })
                .collect()
        }
        ShapeGeometry::Polygon { sides } if *sides >= 3 => {
            let radius = w.min(h) / 2.0;
            let step = TAU / *sides as f64;
            (0..*sides)
                .map(|i| {
                    let a = -FRAC_PI_2 + step * i as f64;
                    Point::new(center.x + radius * a.cos(), center.y + radius * a.sin())
                })
                .collect()
        }
        ShapeGeometry::Star { points, inner_ratio } if *points >= 3 => {
            let outer = w.min(h) / 2.0;
            let inner = outer * inner_ratio;
            let step = PI / *points as f64;
            (0..points * 2)
                .map(|i| {
                    let radius = if i % 2 == 0 { outer } else { inner };
                    let a = -FRAC_PI_2 + step * i as f64;
                    Point::new(center.x + radius * a.cos(), center.y + radius * a.sin())
                })
                .collect()
        }
        ShapeGeometry::Path(path) => {
            let mut pts = Vec::new();
            let mut current = Point::new(x0, y0);
            let n = curve_segments(w.max(h) / 4.0);
            for cmd in &path.commands {
                match cmd {
                    PathCommand::MoveTo(p) | PathCommand::LineTo(p) => {
                        pts.push(*p);
                        current = *p;
                    }
                    PathCommand::QuadTo { control, end } => {
                        for i in 1..=n {
                            let t = i as f64 / n as f64;
                            let u = 1.0 - t;
                            pts.push(Point::new(
                                u * u * current.x + 2.0 * u * t * control.x + t * t * end.x,
                                u * u * current.y + 2.0 * u * t * control.y + t * t * end.y,
                            ));
                        }
                        current = *end;
                    }
                    PathCommand::CubicTo { control1, control2, end } => {
                        for i in 1..=n {
                            let t = i as f64 / n as f64;
                            let u = 1.0 - t;
                            let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                            pts.push(Point::new(
                                a * current.x + b * control1.x + c * control2.x + d * end.x,
                                a * current.y + b * control1.y + c * control2.y + d * end.y,
                            ));
                        }
                        current = *end;
                    }
                    PathCommand::Close => {}
                }
            }
            pts
        }
        // Degenerate polygons/stars fall back to the bounding box
        _ => vec![
            Point::new(x0, y0),
            Point::new(x0 + w, y0),
            Point::new(x0 + w, y0 + h),
            Point::new(x0, y0 + h),
        ],
    };

    // Flips mirror the outline in its own frame, before rotation
    if shape.common.flip_h || shape.common.flip_v {
        for p in &mut points {
            if shape.common.flip_h {
                p.x = 2.0 * center.x - p.x;
            }
            if shape.common.flip_v {
                p.y = 2.0 * center.y - p.y;
            }
        }
    }
    let rotation = shape.common.rotation;
    if rotation != 0.0 {
        let (sin, cos) = rotation.to_radians().sin_cos();
        for p in &mut points {
            let (dx, dy) = (p.x - center.x, p.y - center.y);
            *p = Point::new(center.x + dx * cos - dy * sin, center.y + dx * sin + dy * cos);
        }
    }
    points
}

/// A shape's outline as separate subpaths, flattened, flipped and rotated
/// like [`shape_outline`]: points, whether the subpath is closed, and its
/// drawing flags. Paths give one entry per subpath; other shapes one closed
/// outline.
pub fn shape_subpaths(shape: &repub_model::shape::Shape, scale: f32) -> Vec<(Vec<Point>, bool, repub_model::geometry::SubpathStyle)> {
    use repub_model::geometry::{PathCommand, ShapeGeometry};
    let ShapeGeometry::Path(path) = shape_geometry(shape) else {
        return vec![(shape_outline(shape, scale), true, Default::default())];
    };
    let b = shape.common.bounds;
    let center = b.center();
    let n = ((b.size.width.max(b.size.height) / 4.0 * scale as f64 * 0.75) as usize).clamp(6, 48);
    let (sin, cos) = shape.common.rotation.to_radians().sin_cos();
    let place = |mut p: Point| {
        if shape.common.flip_h {
            p.x = 2.0 * center.x - p.x;
        }
        if shape.common.flip_v {
            p.y = 2.0 * center.y - p.y;
        }
        if shape.common.rotation != 0.0 {
            let (dx, dy) = (p.x - center.x, p.y - center.y);
            p = Point::new(center.x + dx * cos - dy * sin, center.y + dx * sin + dy * cos);
        }
        p
    };
    let mut out = Vec::new();
    for (index, sub) in path.split_subpaths().into_iter().enumerate() {
        let mut pts = Vec::new();
        let mut current = Point::new(b.origin.x, b.origin.y);
        let mut closed = false;
        for cmd in sub {
            match cmd {
                PathCommand::MoveTo(p) | PathCommand::LineTo(p) => {
                    pts.push(*p);
                    current = *p;
                }
                PathCommand::QuadTo { control, end } => {
                    for i in 1..=n {
                        let t = i as f64 / n as f64;
                        let u = 1.0 - t;
                        pts.push(Point::new(
                            u * u * current.x + 2.0 * u * t * control.x + t * t * end.x,
                            u * u * current.y + 2.0 * u * t * control.y + t * t * end.y,
                        ));
                    }
                    current = *end;
                }
                PathCommand::CubicTo { control1, control2, end } => {
                    for i in 1..=n {
                        let t = i as f64 / n as f64;
                        let u = 1.0 - t;
                        let (a, bb, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
                        pts.push(Point::new(
                            a * current.x + bb * control1.x + c * control2.x + d * end.x,
                            a * current.y + bb * control1.y + c * control2.y + d * end.y,
                        ));
                    }
                    current = *end;
                }
                PathCommand::Close => closed = true,
            }
        }
        if pts.len() >= 2 {
            out.push((pts.into_iter().map(place).collect(), closed, path.subpath_style(index)));
        }
    }
    out
}

/// Fill `subpaths` (screen points) with the nonzero or even-odd rule as one mesh.
fn fill_subpaths(subpaths: &[Vec<Pos2>], even_odd: bool, color: egui::Color32) -> Option<egui::Shape> {
    use lyon::math::point;
    use lyon::path::Path as LyonPath;
    use lyon::tessellation::{BuffersBuilder, FillOptions, FillRule, FillTessellator, FillVertex, VertexBuffers};
    let mut builder = LyonPath::builder();
    for sub in subpaths.iter().filter(|s| s.len() >= 3) {
        builder.begin(point(sub[0].x, sub[0].y));
        for p in &sub[1..] {
            builder.line_to(point(p.x, p.y));
        }
        builder.end(true);
    }
    let path = builder.build();
    let mut buffers: VertexBuffers<Pos2, u32> = VertexBuffers::new();
    let options = FillOptions::default().with_fill_rule(if even_odd { FillRule::EvenOdd } else { FillRule::NonZero }).with_tolerance(0.1);
    FillTessellator::new()
        .tessellate_path(&path, &options, &mut BuffersBuilder::new(&mut buffers, |v: FillVertex| Pos2::new(v.position().x, v.position().y)))
        .ok()?;
    if buffers.indices.is_empty() {
        return None;
    }
    let mut mesh = egui::Mesh::default();
    for p in buffers.vertices {
        mesh.colored_vertex(p, color);
    }
    mesh.indices = buffers.indices;
    Some(egui::Shape::mesh(mesh))
}

/// Build the egui shapes that draw a vector shape object, used by both the canvas
/// and the page thumbnails. `alpha` multiplies all colors; strokes are drawn at least
/// `min_stroke` pixels wide.
pub fn shape_object_shapes(
    shape: &repub_model::shape::Shape,
    map: DocToScreen,
    alpha: f32,
    min_stroke: f32,
) -> Vec<egui::Shape> {
    use repub_model::geometry::ShapeGeometry;
    use repub_model::styles::Fill;

    // Gradients and patterns are drawn by recolouring or texturing the flat
    // fill's mesh (see `gradient`, `pattern_fill`)
    let gradient = crate::gradient::is_gradient(&shape.fill) || crate::pattern_fill::is_pattern(&shape.fill);
    let fill = match &shape.fill {
        Fill::None => egui::Color32::TRANSPARENT,
        Fill::Solid(c) => core_color_to_egui(c, alpha),
        _ if gradient => egui::Color32::WHITE,
        other => core_color_to_egui(&other.primary_color(), alpha),
    };
    let stroke = match &shape.line {
        Some(s) if s.width > 0.0 => Stroke::new(
            (s.width as f32 * map.scale).max(min_stroke),
            core_color_to_egui(&s.color, alpha),
        ),
        _ => Stroke::NONE,
    };

    let dash = shape.line.as_ref().and_then(|s| s.dash_pattern.as_deref());
    let geometry = shape_geometry(shape);
    let mut out = Vec::new();
    let mut fills = Vec::new();

    // Unrotated rectangles keep egui's exact (anti-aliased) rounded rect
    if let ShapeGeometry::Rectangle { corner_radius } = geometry {
        if shape.common.rotation == 0.0 {
            let rect = map.rect(&shape.common.bounds);
            fills.push(egui::Shape::rect_filled(rect, Rounding::same(*corner_radius as f32 * map.scale), fill));
            if stroke.width > 0.0 {
                match dash {
                    // Dashed outlines follow the outline points instead
                    Some(_) => {
                        let points: Vec<Pos2> = shape_outline(shape, map.scale).iter().map(|p| map.point(p.x, p.y)).collect();
                        out.extend(stroke_path(&points, true, stroke, dash, map.scale));
                    }
                    None => out.push(egui::Shape::rect_stroke(rect, Rounding::same(*corner_radius as f32 * map.scale), stroke)),
                }
            }
            return finish_fills(shape, fills, out, gradient, map, alpha);
        }
    }

    // Paths: subpaths filled together (holes by winding), each outlined or
    // filled only as its flags say
    if let ShapeGeometry::Path(path) = geometry {
        let subpaths: Vec<(Vec<Pos2>, bool, repub_model::geometry::SubpathStyle)> = shape_subpaths(shape, map.scale)
            .into_iter()
            .map(|(pts, closed, style)| (pts.iter().map(|p| map.point(p.x, p.y)).collect(), closed, style))
            .collect();
        if fill.a() > 0 {
            let filled: Vec<Vec<Pos2>> = subpaths.iter().filter(|(_, _, s)| !s.no_fill).map(|(p, _, _)| p.clone()).collect();
            if let Some(mesh) = fill_subpaths(&filled, path.even_odd, fill) {
                fills.push(mesh);
                // Hairline in the fill color anti-aliases the mesh edges
                if stroke.width <= 0.0 {
                    for sub in filled.iter().filter(|s| s.len() >= 3) {
                        fills.push(egui::Shape::closed_line(sub.clone(), Stroke::new(0.75, fill)));
                    }
                }
            }
        }
        if stroke.width > 0.0 {
            for (pts, closed, style) in &subpaths {
                if !style.no_stroke {
                    out.extend(stroke_path(pts, *closed, stroke, dash, map.scale));
                }
            }
        }
        return finish_fills(shape, fills, out, gradient, map, alpha);
    }

    let points: Vec<Pos2> = shape_outline(shape, map.scale)
        .iter()
        .map(|p| map.point(p.x, p.y))
        .collect();
    if points.len() < 3 {
        return out;
    }

    let convex = !matches!(geometry, ShapeGeometry::Star { .. } | ShapeGeometry::Path(_));
    if fill.a() > 0 {
        if convex {
            fills.push(egui::Shape::convex_polygon(points.clone(), fill, Stroke::NONE));
        } else {
            // Stars (and most simple paths) are star-shaped around their center,
            // so a triangle fan from the center fills them correctly.
            let center = if let ShapeGeometry::Path(_) = geometry {
                let sum = points.iter().fold(Vec2::ZERO, |acc, p| acc + p.to_vec2());
                (sum / points.len() as f32).to_pos2()
            } else {
                let c = shape.common.bounds.center();
                map.point(c.x, c.y)
            };
            let mut mesh = egui::Mesh::default();
            mesh.colored_vertex(center, fill);
            for p in &points {
                mesh.colored_vertex(*p, fill);
            }
            let n = points.len() as u32;
            for i in 0..n {
                mesh.add_triangle(0, 1 + i, 1 + (i + 1) % n);
            }
            fills.push(egui::Shape::mesh(mesh));
            // Hairline in the fill color anti-aliases the mesh edges
            if stroke.width <= 0.0 {
                fills.push(egui::Shape::closed_line(points.clone(), Stroke::new(0.75, fill)));
            }
        }
    }
    if stroke.width > 0.0 {
        out.extend(stroke_path(&points, true, stroke, dash, map.scale));
    }
    finish_fills(shape, fills, out, gradient, map, alpha)
}

/// Stroke a polyline (closed when `closed`), dashed when `dash` holds a
/// pattern of alternating dash and gap lengths in points.
pub fn stroke_path(points: &[Pos2], closed: bool, stroke: Stroke, dash: Option<&[f64]>, scale: f32) -> Vec<egui::Shape> {
    if points.len() < 2 || stroke.width <= 0.0 {
        return Vec::new();
    }
    let pattern: Vec<f32> = dash.unwrap_or(&[]).iter().map(|&d| (d as f32 * scale).max(1.0)).collect();
    if pattern.len() < 2 {
        return vec![if closed { egui::Shape::closed_line(points.to_vec(), stroke) } else { egui::Shape::line(points.to_vec(), stroke) }];
    }
    // An odd-length pattern repeats with dashes and gaps swapped, as in SVG/PDF
    let pattern = if pattern.len() % 2 == 1 { [pattern.clone(), pattern].concat() } else { pattern };
    let dashes: Vec<f32> = pattern.iter().step_by(2).copied().collect();
    let gaps: Vec<f32> = pattern.iter().skip(1).step_by(2).copied().collect();
    let mut path = points.to_vec();
    if closed {
        path.push(points[0]);
    }
    egui::Shape::dashed_line_with_offset(&path, stroke, &dashes, &gaps, 0.0)
}

/// Shapes for a line's arrowhead (or other ending) at `tip`, pointing along
/// `dir` (unit vector from the line towards the tip). `width` is the line
/// width on screen. Returns the shapes and how far the line should stop short
/// of the tip so it doesn't poke through a filled ending.
fn line_ending_shapes(
    ending: &repub_model::shape::LineEnding,
    tip: Pos2,
    dir: Vec2,
    width: f32,
    color: egui::Color32,
) -> (Vec<egui::Shape>, f32) {
    use repub_model::shape::LineEndingStyle as S;
    let length = (width * ending.size.max(1.0) as f32 * 1.5).max(7.0);
    let half = length * 0.5;
    let perp = Vec2::new(-dir.y, dir.x);
    let base = tip - dir * length;
    let fill = if ending.filled { color } else { egui::Color32::TRANSPARENT };
    let outline = Stroke::new(width.max(1.0), color);
    match ending.style {
        S::None => (Vec::new(), 0.0),
        S::Arrow => (vec![egui::Shape::convex_polygon(vec![tip, base + perp * half, base - perp * half], color, Stroke::NONE)], length * 0.8),
        S::StealthArrow => {
            let notch = tip - dir * length * 0.65;
            let left = egui::Shape::convex_polygon(vec![tip, base + perp * half * 0.8, notch], color, Stroke::NONE);
            let right = egui::Shape::convex_polygon(vec![tip, notch, base - perp * half * 0.8], color, Stroke::NONE);
            (vec![left, right], length * 0.6)
        }
        S::OpenArrow => (vec![egui::Shape::line(vec![base + perp * half, tip, base - perp * half], outline)], 0.0),
        S::Diamond => {
            let (c, h) = (tip, half * 0.7);
            let pts = vec![c + dir * h, c + perp * h, c - dir * h, c - perp * h];
            (vec![egui::Shape::convex_polygon(pts, fill, if ending.filled { Stroke::NONE } else { outline })], if ending.filled { 0.0 } else { h })
        }
        S::Circle => {
            let r = half * 0.6;
            (vec![egui::Shape::circle_filled(tip, r, fill), egui::Shape::circle_stroke(tip, r, if ending.filled { Stroke::NONE } else { outline })], if ending.filled { 0.0 } else { r })
        }
        S::Square => {
            let h = half * 0.6;
            let pts = vec![tip + dir * h + perp * h, tip - dir * h + perp * h, tip - dir * h - perp * h, tip + dir * h - perp * h];
            (vec![egui::Shape::convex_polygon(pts, fill, if ending.filled { Stroke::NONE } else { outline })], if ending.filled { 0.0 } else { h })
        }
    }
}

/// Where a picture goes in its frame: screen rects and the texture
/// coordinates (0–1) drawn into each. `frame` is the unrotated frame on screen,
/// `px` the image size in pixels, `screen_per_px` screen points per image pixel
/// at natural size (96 dpi: 0.75 pt per pixel, times zoom). The crop rectangle
/// (normalised) picks the part of the image used; `fit` places that part.
pub fn picture_placement(
    fit: repub_model::shape::ImageFit,
    crop: Option<&repub_geometry::Rect>,
    px: [f32; 2],
    frame: Rect,
    screen_per_px: f32,
) -> Vec<(Rect, Rect)> {
    use repub_model::shape::ImageFit;
    let crop = crop
        .map(|c| Rect::from_min_size(Pos2::new(c.origin.x as f32, c.origin.y as f32), Vec2::new(c.size.width as f32, c.size.height as f32)))
        .filter(|c| c.width() > 0.0 && c.height() > 0.0)
        .unwrap_or(Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)));
    // Size in pixels of the part of the image in use
    let src = Vec2::new(px[0] * crop.width(), px[1] * crop.height());
    if src.x <= 0.0 || src.y <= 0.0 || frame.width() <= 0.0 || frame.height() <= 0.0 {
        return Vec::new();
    }
    let src_aspect = src.x / src.y;
    let frame_aspect = frame.width() / frame.height();
    match fit {
        ImageFit::Stretch => vec![(frame, crop)],
        // Cover the frame, trimming the source evenly on the long side
        ImageFit::Fill => {
            let uv = if src_aspect > frame_aspect {
                let keep = frame_aspect / src_aspect;
                Rect::from_center_size(crop.center(), Vec2::new(crop.width() * keep, crop.height()))
            } else {
                let keep = src_aspect / frame_aspect;
                Rect::from_center_size(crop.center(), Vec2::new(crop.width(), crop.height() * keep))
            };
            vec![(frame, uv)]
        }
        // Whole source inside the frame, centred
        ImageFit::Fit => {
            let size = if src_aspect > frame_aspect {
                Vec2::new(frame.width(), frame.width() / src_aspect)
            } else {
                Vec2::new(frame.height() * src_aspect, frame.height())
            };
            vec![(Rect::from_center_size(frame.center(), size), crop)]
        }
        // Natural size, centred (clipped to the frame by the caller)
        ImageFit::Center => vec![(Rect::from_center_size(frame.center(), src * screen_per_px), crop)],
        // Natural-size tiles from the frame's top-left (clipped by the caller)
        ImageFit::Tile => {
            let tile = src * screen_per_px;
            if tile.x < 1.0 || tile.y < 1.0 {
                return vec![(frame, crop)];
            }
            let (cols, rows) = ((frame.width() / tile.x).ceil() as usize, (frame.height() / tile.y).ceil() as usize);
            if cols * rows > 4096 {
                return vec![(frame, crop)];
            }
            (0..rows)
                .flat_map(|r| (0..cols).map(move |c| (r, c)))
                .map(|(r, c)| (Rect::from_min_size(frame.min + Vec2::new(c as f32 * tile.x, r as f32 * tile.y), tile), crop))
                .collect()
        }
    }
}

/// Tell the text layout, for every text box on `page`, which objects in front
/// of it it wraps around (objects with a text wrap style, padded by their wrap
/// margins). Rotated text boxes don't wrap.
pub fn update_text_wrap(page: &repub_model::Page) {
    use repub_model::styles::TextWrapStyle;
    let objects: Vec<&Shape> = page.layers().iter().filter(|l| l.visible).flat_map(|l| l.shapes()).filter(|o| o.common.visible).collect();
    for (i, obj) in objects.iter().enumerate() {
        let ShapeKind::TextFrame(_) = &obj.kind else { continue };
        let frame = obj.common.bounds;
        let mut exclusions = Vec::new();
        if obj.common.rotation == 0.0 {
            for other in &objects[i + 1..] {
                let wrap = other.common.text_wrap;
                if wrap.style == TextWrapStyle::None {
                    continue;
                }
                let (x0, y0, x1, y1) = rotated_extent(other);
                let (x0, y0, x1, y1) = (x0 - wrap.margin_left, y0 - wrap.margin_top, x1 + wrap.margin_right, y1 + wrap.margin_bottom);
                let overlaps = x0 < frame.origin.x + frame.size.width && x1 > frame.origin.x && y0 < frame.origin.y + frame.size.height && y1 > frame.origin.y;
                if overlaps {
                    exclusions.push(crate::sdf_text::Exclusion {
                        x0: x0 as f32,
                        y0: y0 as f32,
                        x1: x1 as f32,
                        y1: y1 as f32,
                        full_width: wrap.style == TextWrapStyle::TopAndBottom,
                        side: wrap.side,
                    });
                }
            }
        }
        crate::sdf_text::set_exclusions(obj.common.id, exclusions);
    }
}

/// Axis-aligned extent of an object after its rotation: `(x0, y0, x1, y1)`.
fn rotated_extent(obj: &Shape) -> (f64, f64, f64, f64) {
    let b = obj.bounds();
    let c = b.center();
    let (sin, cos) = obj.common.rotation.to_radians().sin_cos();
    let (hw, hh) = (b.size.width / 2.0, b.size.height / 2.0);
    let (ex, ey) = ((hw * cos).abs() + (hh * sin).abs(), (hw * sin).abs() + (hh * cos).abs());
    (c.x - ex, c.y - ey, c.x + ex, c.y + ey)
}

/// An object's glow and drop shadow (shapes, text boxes, pictures), drawn
/// behind it: silhouettes of the object, the glow spread around it and the
/// shadow offset.
pub fn shadow_shapes(obj: &Shape, map: DocToScreen) -> Vec<egui::Shape> {
    // Shadows and glows are drawn for auto shapes, text boxes and pictures.
    if !matches!(obj.kind, ShapeKind::AutoShape(_) | ShapeKind::TextFrame(_) | ShapeKind::Picture(_)) {
        return Vec::new();
    }
    let effects = &obj.effects;
    let mut out = Vec::new();
    if let Some(glow) = effects.outer_glow.as_ref().filter(|g| g.color.a > 0.0) {
        // The glow's radius blurs it; its spread widens it beyond the object
        out.extend(silhouette_shapes(obj, map, glow.color, 0.0, 0.0, glow.spread + glow.radius / 2.0, glow.radius));
    }
    if let Some(shadow) = effects.drop_shadow.as_ref().filter(|s| s.color.a > 0.0) {
        out.extend(silhouette_shapes(obj, map, shadow.color, shadow.offset_x, shadow.offset_y, shadow.spread, shadow.blur_radius));
    }
    out
}

/// A silhouette of `obj` (its outline for shapes, its rotated box otherwise)
/// in `color`, moved by `(dx, dy)`, grown by `spread` and blurred by `blur`
/// (points). Unrotated rectangles get egui's true blur; other outlines
/// approximate it with stacked silhouettes from small to large.
fn silhouette_shapes(obj: &Shape, map: DocToScreen, color: repub_color::Color, dx: f64, dy: f64, spread: f64, blur: f64) -> Vec<egui::Shape> {
    use repub_model::geometry::ShapeGeometry;
    use repub_model::shape::Shape;
    use repub_model::styles::Fill;
    let mut silhouette = match &obj.kind {
        ShapeKind::AutoShape(_) => obj.clone(),
        _ => {
            let mut box_shape = Shape::rectangle(obj.bounds());
            box_shape.common.rotation = obj.common.rotation;
            box_shape
        }
    };
    silhouette.line = None;
    silhouette.effects.drop_shadow = None;
    silhouette.effects.outer_glow = None;
    let b = silhouette.common.bounds;
    let place = |grow: f64| -> Shape {
        let mut s = silhouette.clone();
        let spread = spread + grow;
        s.set_bounds(repub_geometry::Rect::new(
            b.origin.x + dx - spread,
            b.origin.y + dy - spread,
            (b.size.width + 2.0 * spread).max(0.1),
            (b.size.height + 2.0 * spread).max(0.1),
        ));
        s
    };
    let blur = blur.max(0.0);

    if let ShapeGeometry::Rectangle { corner_radius } = *shape_geometry(&silhouette) {
        if silhouette.common.rotation == 0.0 {
            let s = place(0.0);
            let rect = map.rect(&s.common.bounds);
            let fill = core_color_to_egui(&color, 1.0);
            let radius = (corner_radius + spread.max(0.0)) as f32 * map.scale;
            return vec![egui::Shape::Rect(
                egui::epaint::RectShape::filled(rect, Rounding::same(radius), fill).with_blur_width(blur as f32 * map.scale),
            )];
        }
    }
    let layers = if blur * map.scale as f64 >= 1.0 { 5 } else { 1 };
    let mut out = Vec::new();
    for k in 0..layers {
        let grow = if layers == 1 { 0.0 } else { blur * (k as f64 / (layers - 1) as f64 - 0.5) };
        let mut s = place(grow);
        let mut c = color;
        c.a /= layers as f64;
        s.fill = Fill::Solid(c);
        out.extend(shape_object_shapes(&s, map, 1.0, 0.0));
    }
    out
}

/// A text box showing a WordArt or text-on-path object's text: one centred
/// line in its style and fill colour, Best Fit to the frame, no margins.
pub fn fitted_text_frame(obj: &Shape) -> repub_model::shape::Shape {
    use repub_model::shape::{Margins, Shape, ShapeKind, TextFitMode, TextFrameData, VerticalAlignment};
    use repub_model::text::{Paragraph, StyledText, TextRun};
    let (text, mut style) = match &obj.kind {
        ShapeKind::TextEffect(effect) => {
            let mut style = effect.style.clone();
            // The text effect's fill colours its text (as the old WordArt did)
            style.color = obj.fill.primary_color();
            style.letter_spacing = effect.letter_spacing;
            // Use a safe fallback font - custom fonts may not be registered with egui
            style.font.family = "Arial".to_string();
            (effect.text.clone(), style)
        }
        _ => (String::new(), Default::default()),
    };
    style.font_size = style.font_size.max(1.0);
    let mut data = TextFrameData::new();
    let mut para = Paragraph::new();
    para.style.alignment = repub_model::styles::TextAlignment::Center;
    para.style.default_char_style = style.clone();
    para.elements.push(repub_model::text::InlineElement::Text(TextRun::styled(&text, style)));
    data.content = StyledText { paragraphs: vec![para] };
    data.margins = Margins::uniform(0.0);
    data.inset = 0.0;
    data.vertical_align = VerticalAlignment::Center;
    data.text_fit = TextFitMode::BestFit;
    let mut tf = Shape::new(obj.bounds(), ShapeKind::TextFrame(data));
    tf.common.rotation = obj.common.rotation;
    tf.common.id = obj.id(); // fit is cached per object
    tf
}

/// Turn `shape` by `degrees` (clockwise) about `pivot`. A group turns by turning each member, so its
/// members keep their own angles and the group's box is recomputed around them.
pub fn rotate_about(shape: &mut Shape, pivot: Point, degrees: f64) {
    if let ShapeKind::Group(group) = &mut shape.kind {
        for child in &mut group.children {
            rotate_about(child, pivot, degrees);
        }
        group.recalculate_bounds();
        shape.common.bounds = group.bounds;
        return;
    }
    let (sin, cos) = degrees.to_radians().sin_cos();
    let (b, c) = (shape.bounds(), shape.bounds().center());
    let (dx, dy) = (c.x - pivot.x, c.y - pivot.y);
    let centre = Point::new(pivot.x + dx * cos - dy * sin, pivot.y + dx * sin + dy * cos);
    shape.set_position(Point::new(centre.x - b.size.width / 2.0, centre.y - b.size.height / 2.0));
    shape.common.rotation = (shape.common.rotation + degrees).rem_euclid(360.0);
}

/// The outline a selected shape is traced along, as page-space polylines
/// (`true` when closed), or `None` when its box is its outline (rectangles,
/// text frames, pictures, tables, groups, WordArt).
fn selection_outline(shape: &repub_model::shape::Shape, scale: f32) -> Option<Vec<(Vec<Point>, bool)>> {
    use repub_model::geometry::ShapeGeometry;
    match &shape.kind {
        ShapeKind::AutoShape(auto) => {
            let boxy = auto.preset.is_none()
                && matches!(auto.geometry, ShapeGeometry::Rectangle { corner_radius } if corner_radius <= 0.0);
            if boxy {
                return None;
            }
            Some(shape_subpaths(shape, scale).into_iter().map(|(points, closed, _)| (points, closed)).collect())
        }
        ShapeKind::Line(line) => {
            let c = shape.common.bounds.center();
            let (sin, cos) = shape.common.rotation.to_radians().sin_cos();
            let turn = |p: Point| {
                let (dx, dy) = (p.x - c.x, p.y - c.y);
                Point::new(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos)
            };
            Some(vec![(crate::hit_test::line_polyline(line).into_iter().map(turn).collect(), false)])
        }
        _ => None,
    }
}

/// A preset shape's adjust handles on the page, flipped and turned with it.
fn adjust_handles(shape: &repub_model::shape::Shape) -> Vec<Point> {
    use repub_escher::PresetExt;
    let ShapeKind::AutoShape(auto) = &shape.kind else { return Vec::new() };
    let Some(geometry) = &auto.preset else { return Vec::new() };
    let b = shape.common.bounds;
    let c = b.center();
    let (sin, cos) = shape.common.rotation.to_radians().sin_cos();
    geometry
        .handles(b)
        .into_iter()
        .map(|mut p| {
            if shape.common.flip_h {
                p.x = 2.0 * c.x - p.x;
            }
            if shape.common.flip_v {
                p.y = 2.0 * c.y - p.y;
            }
            let (dx, dy) = (p.x - c.x, p.y - c.y);
            Point::new(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos)
        })
        .collect()
}

/// A page point in a shape's own frame: turned back and unflipped (the
/// inverse of how its outline and handles are placed).
fn shape_frame_point(shape: &repub_model::shape::Shape, p: Point) -> Point {
    let b = shape.common.bounds;
    let c = b.center();
    let (sin, cos) = (-shape.common.rotation).to_radians().sin_cos();
    let (dx, dy) = (p.x - c.x, p.y - c.y);
    let mut q = Point::new(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos);
    if shape.common.flip_h {
        q.x = 2.0 * c.x - q.x;
    }
    if shape.common.flip_v {
        q.y = 2.0 * c.y - q.y;
    }
    q
}

/// A box turned by `rotation` degrees (clockwise) about its centre: where the
/// selection outline and handles go.
#[derive(Debug, Clone, Copy)]
struct SelectionFrame {
    rect: repub_geometry::Rect,
    rotation: f64,
}

impl SelectionFrame {
    fn turn(&self, p: Point, degrees: f64) -> Point {
        if degrees == 0.0 {
            return p;
        }
        let c = self.rect.center();
        let (sin, cos) = degrees.to_radians().sin_cos();
        let (dx, dy) = (p.x - c.x, p.y - c.y);
        Point::new(c.x + dx * cos - dy * sin, c.y + dx * sin + dy * cos)
    }

    /// A point in the box's own axes -> the page.
    fn to_page(&self, p: Point) -> Point {
        self.turn(p, self.rotation)
    }

    /// A page point -> the box's own axes.
    fn to_local(&self, p: Point) -> Point {
        self.turn(p, -self.rotation)
    }

    /// The upright box around the turned one.
    fn page_bounds(&self) -> repub_geometry::Rect {
        let r = self.rect;
        let pts = [r.origin, Point::new(r.origin.x + r.size.width, r.origin.y), Point::new(r.origin.x + r.size.width, r.origin.y + r.size.height), Point::new(r.origin.x, r.origin.y + r.size.height)]
            .map(|p| self.to_page(p));
        let (x0, x1) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
        let (y0, y1) = pts.iter().fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.y), b.max(p.y)));
        repub_geometry::Rect::new(x0, y0, x1 - x0, y1 - y0)
    }
}

/// Selection highlight and blinking caret of the text edit `edit` in frame
/// `tf` (laid out as `layout`). Used for table cells.
fn draw_edit_marks(painter: &egui::Painter, tf: &Shape, layout: &crate::sdf_text::FrameLayout, edit: &text_edit::TextEditState, origin: Pos2, zoom: f32) {
    let to_screen = |x: f32, y: f32| {
        let (dx, dy) = text_edit::from_frame_space(tf, x, y);
        origin + Vec2::new(dx, dy) * zoom
    };
    if edit.has_selection() {
        for [x0, y0, x1, y1] in text_edit::selection_rects(layout, edit.caret, edit.anchor) {
            let quad = vec![to_screen(x0, y0), to_screen(x1, y0), to_screen(x1, y1), to_screen(x0, y1)];
            painter.add(egui::Shape::convex_polygon(quad, colors::text_selection().gamma_multiply(0.7), Stroke::NONE));
        }
        return;
    }
    let time = painter.ctx().input(|i| i.time);
    const PERIOD: f64 = 1.06;
    let phase = (time - edit.blink_epoch).max(0.0) % PERIOD;
    if phase < PERIOD / 2.0 {
        if let Some((x, top, bottom)) = text_edit::caret_geometry(layout, edit.caret) {
            painter.line_segment([to_screen(x, top), to_screen(x, bottom)], Stroke::new(1.5, colors::text_primary()));
        }
    }
    painter.ctx().request_repaint_after(std::time::Duration::from_secs_f64(PERIOD / 2.0 - phase % (PERIOD / 2.0)));
}

/// The overflow mark of text box `tf` on screen: a small box just below its
/// bottom-right corner (clicking it starts Create Link).
fn overflow_mark_rect(tf: &Shape, origin: Pos2, zoom: f32) -> Rect {
    let b = tf.common.bounds;
    let (x, y) = text_edit::from_frame_space(tf, (b.origin.x + b.size.width) as f32, (b.origin.y + b.size.height) as f32);
    let corner = origin + Vec2::new(x, y) * zoom;
    Rect::from_min_size(corner + Vec2::new(-22.0, 6.0), Vec2::new(22.0, 14.0))
}

/// Turn shapes about `centre` (screen) by the angle with sine `sin` and cosine
/// `cos`. Rectangles become polygons. Used for rotated tables.
fn rotate_shapes(shapes: Vec<egui::Shape>, centre: Pos2, sin: f32, cos: f32) -> Vec<egui::Shape> {
    if sin == 0.0 && cos == 1.0 {
        return shapes;
    }
    let turn = |p: Pos2| {
        let d = p - centre;
        centre + Vec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos)
    };
    shapes
        .into_iter()
        .map(|shape| match shape {
            egui::Shape::LineSegment { points, stroke } => egui::Shape::LineSegment { points: points.map(turn), stroke },
            egui::Shape::Rect(r) => {
                let corners = [r.rect.left_top(), r.rect.right_top(), r.rect.right_bottom(), r.rect.left_bottom()].map(turn);
                egui::Shape::convex_polygon(corners.to_vec(), r.fill, Stroke::NONE)
            }
            egui::Shape::Path(mut p) => {
                p.points = p.points.into_iter().map(turn).collect();
                egui::Shape::Path(p)
            }
            egui::Shape::Mesh(mut m) => {
                for v in &mut m.vertices {
                    v.pos = turn(v.pos);
                }
                egui::Shape::Mesh(m)
            }
            other => other,
        })
        .collect()
}

/// Fill shapes first (flat, or recoloured as a gradient or pattern), then strokes on top.
fn finish_fills(
    shape: &repub_model::shape::Shape,
    fills: Vec<egui::Shape>,
    strokes: Vec<egui::Shape>,
    gradient: bool,
    map: DocToScreen,
    alpha: f32,
) -> Vec<egui::Shape> {
    let mut out = match &shape.fill {
        _ if !gradient || fills.is_empty() => fills,
        repub_model::styles::Fill::Pattern(p) => {
            vec![crate::pattern_fill::fill_shapes(fills, p, shape.common.bounds, shape.common.rotation, map, alpha)]
        }
        fill => vec![crate::gradient::fill_shapes(fills, fill, shape.common.bounds, shape.common.rotation, map, alpha)],
    };
    out.extend(strokes);
    out
}

#[cfg(test)]
mod tests {
    use super::{halve_image, line_ending_shapes, selection_outline, stroke_path, ImageMips};
    use egui::{Pos2, Stroke, Vec2};

    #[test]
    fn rotating_a_group_turns_its_members_about_the_group_centre() {
        use super::rotate_about;
        use repub_geometry::Rect;
        use repub_model::shape::{Shape, ShapeKind};
        // Two squares side by side: a 30 x 10 box centred at (15, 5)
        let left = Shape::rectangle(Rect::new(0.0, 0.0, 10.0, 10.0));
        let right = Shape::rectangle(Rect::new(20.0, 0.0, 10.0, 10.0));
        let mut group = Shape::group_from(vec![left, right]);
        let pivot = group.bounds().center();
        rotate_about(&mut group, pivot, 90.0);
        let ShapeKind::Group(g) = &group.kind else { unreachable!() };
        // Turned a quarter: the squares now stack vertically around the same centre
        let (a, b) = (g.children[0].bounds().center(), g.children[1].bounds().center());
        assert!((a.x - 15.0).abs() < 1e-6 && (b.x - 15.0).abs() < 1e-6);
        assert!((a.y - b.y).abs() > 19.0);
        assert!(((a.y + b.y) / 2.0 - pivot.y).abs() < 1e-6);
        assert!((g.children[0].common.rotation - 90.0).abs() < 1e-6);
    }

    #[test]
    fn selection_follows_shape_outline() {
        use repub_geometry::{Point, Rect};
        use repub_model::shape::Shape;
        let bounds = Rect::new(0.0, 0.0, 100.0, 50.0);
        assert!(selection_outline(&Shape::rectangle(bounds), 1.0).is_none());
        let ellipse = selection_outline(&Shape::ellipse(bounds), 1.0).expect("ellipse has its own outline");
        assert_eq!(ellipse.len(), 1);
        assert!(ellipse[0].1 && ellipse[0].0.len() > 8);
        let line = selection_outline(&Shape::line(Point::new(0.0, 0.0), Point::new(10.0, 10.0)), 1.0).expect("line");
        assert_eq!(line[0].0.len(), 2);
        assert!(!line[0].1);
    }

    #[test]
    fn picture_fit_modes() {
        use super::picture_placement;
        use egui::Rect;
        use repub_model::shape::ImageFit;
        // A 200x100 image in a 100x100 frame
        let frame = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        let px = [200.0, 100.0];
        let full = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));

        assert_eq!(picture_placement(ImageFit::Stretch, None, px, frame, 1.0), vec![(frame, full)]);

        let fill = picture_placement(ImageFit::Fill, None, px, frame, 1.0);
        assert_eq!(fill[0].0, frame, "covers the frame");
        assert!((fill[0].1.width() - 0.5).abs() < 1e-6 && (fill[0].1.center().x - 0.5).abs() < 1e-6, "middle half of the width");

        let fit = picture_placement(ImageFit::Fit, None, px, frame, 1.0);
        assert_eq!(fit[0].0.size(), Vec2::new(100.0, 50.0), "letterboxed");
        assert_eq!(fit[0].0.center(), frame.center());

        let centre = picture_placement(ImageFit::Center, None, px, frame, 0.75);
        assert_eq!(centre[0].0.size(), Vec2::new(150.0, 75.0), "natural size at 96 dpi");

        assert_eq!(picture_placement(ImageFit::Tile, None, [40.0, 40.0], frame, 1.0).len(), 9, "3 x 3 tiles");
    }

    #[test]
    fn picture_crop_picks_the_part_shown() {
        use super::picture_placement;
        use repub_geometry::Rect as DocRect;
        use repub_model::shape::ImageFit;
        let frame = egui::Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        let crop = DocRect::new(0.25, 0.0, 0.5, 1.0);
        let pieces = picture_placement(ImageFit::Stretch, Some(&crop), [100.0, 100.0], frame, 1.0);
        assert_eq!(pieces[0].1, egui::Rect::from_min_max(Pos2::new(0.25, 0.0), Pos2::new(0.75, 1.0)));
    }

    #[test]
    fn shadows_are_offset_silhouettes() {
        use super::{shadow_shapes, DocToScreen};
        use repub_geometry::Rect as DocRect;
        use repub_model::shape::Shape;
        use repub_model::styles::DropShadow;
        let map = DocToScreen { origin: Pos2::ZERO, scale: 1.0 };
        let mut rect = Shape::rectangle(DocRect::new(10.0, 10.0, 50.0, 30.0));
        assert!(shadow_shapes(&rect, map).is_empty(), "no shadow by default");
        rect.effects.drop_shadow = Some(DropShadow::new(4.0, 6.0, 3.0));
        let shapes = shadow_shapes(&rect, map);
        match &shapes[..] {
            [egui::Shape::Rect(r)] => {
                assert_eq!(r.rect.min, Pos2::new(14.0, 16.0), "offset");
                assert_eq!(r.blur_width, 3.0, "true blur for rectangles");
            }
            other => panic!("one blurred rect, got {other:?}"),
        }
        // Rotated: stacked silhouettes approximate the blur
        rect.common.rotation = 30.0;
        assert_eq!(shadow_shapes(&rect, map).len(), 5);
    }

    #[test]
    fn wordart_text_fills_its_frame() {
        use super::fitted_text_frame;
        use repub_geometry::Rect as DocRect;
        use repub_model::shape::Shape;
        let art = Shape::text_effect(DocRect::new(0.0, 0.0, 400.0, 120.0), "Sale");
        let tf = fitted_text_frame(&art);
        let layout = crate::sdf_text::layout_frame(&tf);
        let line = &layout.lines[0];
        assert!(line.bottom - line.top > 60.0, "text grown to the frame: {}", line.bottom - line.top);
        assert!(line.top >= -0.5 && line.bottom <= 120.5, "and inside it");
        let width = line.xs.last().unwrap() - line.xs[0];
        assert!(((line.xs[0] + width / 2.0) - 200.0).abs() < 2.0, "centred");
    }

    #[test]
    fn rotated_table_shapes_turn_about_the_centre() {
        use super::rotate_shapes;
        let seg = egui::Shape::line_segment([Pos2::new(0.0, 0.0), Pos2::new(10.0, 0.0)], Stroke::new(1.0, egui::Color32::BLACK));
        let (sin, cos) = 90f32.to_radians().sin_cos();
        match &rotate_shapes(vec![seg], Pos2::ZERO, sin, cos)[..] {
            [egui::Shape::LineSegment { points, .. }] => {
                assert!((points[1] - Pos2::new(0.0, 10.0)).length() < 1e-4, "{:?}", points[1]);
            }
            other => panic!("{other:?}"),
        }
        let rect = egui::Shape::rect_filled(egui::Rect::from_min_size(Pos2::ZERO, Vec2::splat(4.0)), 0.0, egui::Color32::RED);
        assert!(matches!(rotate_shapes(vec![rect], Pos2::ZERO, sin, cos)[..], [egui::Shape::Path(_)]), "rects become polygons");
    }

    #[test]
    fn dashed_paths_break_into_segments() {
        let pts = [Pos2::new(0.0, 0.0), Pos2::new(100.0, 0.0)];
        let stroke = Stroke::new(1.0, egui::Color32::BLACK);
        assert_eq!(stroke_path(&pts, false, stroke, None, 1.0).len(), 1, "solid: one line");
        let dashed = stroke_path(&pts, false, stroke, Some(&[4.0, 2.0]), 1.0);
        assert!(dashed.len() >= 15, "about 100 / 6 dashes, got {}", dashed.len());
        // Dash lengths scale with zoom
        assert!(stroke_path(&pts, false, stroke, Some(&[4.0, 2.0]), 2.0).len() < dashed.len());
    }

    #[test]
    fn filled_arrows_shorten_the_line_open_arrows_do_not() {
        use repub_model::shape::{LineEnding, LineEndingStyle};
        let black = egui::Color32::BLACK;
        let (shapes, inset) = line_ending_shapes(&LineEnding::arrow(), Pos2::new(50.0, 0.0), Vec2::X, 1.0, black);
        assert_eq!(shapes.len(), 1);
        assert!(inset > 0.0);
        let (_, inset) = line_ending_shapes(&LineEnding::new(LineEndingStyle::OpenArrow), Pos2::ZERO, Vec2::X, 1.0, black);
        assert_eq!(inset, 0.0);
        let (shapes, _) = line_ending_shapes(&LineEnding::default(), Pos2::ZERO, Vec2::X, 1.0, black);
        assert!(shapes.is_empty(), "no ending by default");
    }

    #[test]
    fn halving_averages_and_ignores_transparent_colour() {
        // Opaque red next to fully transparent "black": the edge stays red, half covered
        let mut src = image::RgbaImage::new(2, 2);
        src.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
        src.put_pixel(0, 1, image::Rgba([255, 0, 0, 255]));
        let out = halve_image(&src);
        assert_eq!(out.dimensions(), (1, 1));
        assert_eq!(out.get_pixel(0, 0).0, [255, 0, 0, 128]);
    }

    #[test]
    fn halving_odd_sizes_keeps_every_pixel() {
        let src = image::RgbaImage::from_pixel(5, 3, image::Rgba([10, 20, 30, 255]));
        let out = halve_image(&src);
        assert_eq!(out.dimensions(), (2, 1));
        assert!(out.pixels().all(|p| p.0 == [10, 20, 30, 255]));
    }

    #[test]
    fn mip_chain_picks_nearest_level_not_smaller_than_screen() {
        let ctx = egui::Context::default();
        let mips = ImageMips::upload(&ctx, "test", image::RgbaImage::new(256, 128));
        let widths: Vec<usize> = mips.levels.iter().map(|t| t.size()[0]).collect();
        assert_eq!(widths, vec![256, 128, 64, 32, 16, 8]);
        // Drawn at 40x20 px: the 64x32 level (never upsampled, at most 2x reduced)
        assert_eq!(mips.for_screen_size(Vec2::new(40.0, 20.0)).size(), [64, 32]);
        // Drawn exactly at a level's size uses that level
        assert_eq!(mips.for_screen_size(Vec2::new(128.0, 64.0)).size(), [128, 64]);
        // Drawn larger than the image: full size
        assert_eq!(mips.for_screen_size(Vec2::new(1000.0, 500.0)).size(), [256, 128]);
    }
}
