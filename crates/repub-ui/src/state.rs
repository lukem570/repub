//! Application state management.

use std::path::PathBuf;

use repub_geometry::Point;
use repub_model::shape::{Shape, ShapeKind};
use repub_model::Document;
use repub_model::ObjectId;

use crate::commands::CommandHistory;
use crate::tools::Tool;

/// A guide line for precise alignment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Guide {
    /// Position in document points
    pub position: f64,
    /// Whether this is a horizontal or vertical guide
    pub orientation: GuideOrientation,
    /// Whether the guide is locked
    pub locked: bool,
}

/// Guide orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuideOrientation {
    Horizontal,
    Vertical,
}

impl Guide {
    fn from_model(g: repub_model::Guide) -> Self {
        Self {
            position: g.position,
            orientation: if g.horizontal {
                GuideOrientation::Horizontal
            } else {
                GuideOrientation::Vertical
            },
            locked: false,
        }
    }

    fn to_model(self) -> repub_model::Guide {
        repub_model::Guide {
            position: self.position,
            horizontal: self.orientation == GuideOrientation::Horizontal,
        }
    }

    /// Create a new horizontal guide at the given Y position.
    pub fn horizontal(y: f64) -> Self {
        Self {
            position: y,
            orientation: GuideOrientation::Horizontal,
            locked: false,
        }
    }

    /// Create a new vertical guide at the given X position.
    pub fn vertical(x: f64) -> Self {
        Self {
            position: x,
            orientation: GuideOrientation::Vertical,
            locked: false,
        }
    }
}

/// What Align lines objects up to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AlignTarget {
    /// Each other when several are selected, the page for one
    #[default]
    Auto,
    Page,
    Margins,
}

/// Drag state for page panel.
#[derive(Debug, Clone, Default)]
pub struct PageDragState {
    /// Index of the page being dragged (if any)
    pub dragging_page: Option<usize>,
    /// Target drop position (between pages)
    pub drop_target: Option<usize>,
    /// Whether Ctrl/Cmd key is held (for duplicate)
    pub duplicate_mode: bool,
}

/// The fill, outline and effects a [`Shape`] carries, copied together by the
/// Format Painter. In the new model these live directly on the shape (the old
/// `ShapeStyle` type is gone), so this bundles them for copying and applying.
#[derive(Debug, Clone)]
pub struct ShapeFormat {
    /// Fill of the shape.
    pub fill: repub_model::styles::Fill,
    /// Outline of the shape (`None` for no outline).
    pub line: Option<repub_model::styles::Stroke>,
    /// Visual effects (shadow, glow, …).
    pub effects: repub_model::styles::Effects,
    /// Overall opacity.
    pub opacity: f64,
    /// How text wraps around the shape.
    pub text_wrap: repub_model::styles::TextWrap,
}

impl Default for ShapeFormat {
    fn default() -> Self {
        Self {
            fill: Default::default(),
            line: None,
            effects: Default::default(),
            opacity: 1.0,
            text_wrap: Default::default(),
        }
    }
}

impl ShapeFormat {
    /// Copy the fill, line and effects from a shape.
    pub fn from_shape(shape: &Shape) -> Self {
        Self {
            fill: shape.fill.clone(),
            line: shape.line.clone(),
            effects: shape.effects.clone(),
            opacity: shape.opacity,
            text_wrap: shape.common.text_wrap.clone(),
        }
    }

    /// Apply the fill, line and effects onto a shape.
    pub fn apply_to(&self, shape: &mut Shape) {
        shape.fill = self.fill.clone();
        shape.line = self.line.clone();
        shape.effects = self.effects.clone();
        shape.opacity = self.opacity;
        shape.common.text_wrap = self.text_wrap.clone();
    }
}

/// Formatting copied by the Format Painter.
#[derive(Debug, Clone)]
pub enum CopiedFormat {
    /// Shape style only.
    Shape(ShapeFormat),
    /// Text frame style plus the default character style.
    Text {
        shape: ShapeFormat,
        character: repub_model::styles::CharacterStyle,
    },
}

/// Shape point-editing session.
#[derive(Debug, Clone)]
pub struct EditPointsState {
    /// Shape being edited
    pub shape_id: ObjectId,
    /// Index of the selected anchor point
    pub selected: Option<usize>,
    /// Whether a point is being dragged
    pub dragging: bool,
    /// The curve handle being dragged
    pub dragging_control: Option<repub_model::path_edit::ControlRef>,
}

/// Main application state.
#[derive(Debug)]
pub struct AppState {
    /// Currently open document
    pub document: Option<Document>,
    /// Path to the document file
    pub document_path: Option<PathBuf>,
    /// Whether the document has unsaved changes
    pub dirty: bool,
    /// Current page index (0-based)
    pub current_page: usize,
    /// Current zoom level (1.0 = 100%)
    pub zoom: f32,
    /// Canvas pan offset
    pub pan_offset: Point,
    /// Selected object IDs
    pub selection: Vec<ObjectId>,
    /// Selected page indices for multi-select
    pub selected_pages: Vec<usize>,
    /// Page drag and drop state
    pub page_drag: PageDragState,
    /// Index of the last clicked page (for shift-click range selection)
    pub last_clicked_page: Option<usize>,
    /// Current tool
    pub current_tool: Tool,
    /// Command history for undo/redo
    pub history: CommandHistory,
    /// Whether rulers are visible
    pub show_rulers: bool,
    /// Whether guides are visible
    pub show_guides: bool,
    /// Whether grid is visible
    pub show_grid: bool,
    /// Grid spacing in points
    pub grid_spacing: f64,
    /// Snap to grid enabled
    pub snap_to_grid: bool,
    /// Snap to guides enabled
    pub snap_to_guides: bool,
    /// Snap to other objects enabled
    pub snap_to_objects: bool,
    /// Whether guides are locked (cannot be moved or deleted)
    pub guides_locked: bool,
    /// Show two facing pages side by side (magazine spread view)
    pub two_page_spread: bool,
    /// Fit the whole page in the viewport on the next canvas frame.
    /// Set when a document is opened or created, and by Zoom > Whole Page.
    pub fit_page_pending: bool,
    /// The canvas asks for the Edit WordArt dialog (double-click on WordArt)
    pub edit_wordart_requested: bool,
    /// What Align lines objects up to
    pub align_target: AlignTarget,
    /// A panel asks for Page Properties on this page
    pub page_properties_requested: Option<usize>,
    /// Pointer position over the canvas in document points (for the status bar).
    pub pointer_doc: Option<Point>,
    /// Document snapshot taken by `begin_edit`, recorded by `end_edit` if changed.
    pending_undo: Option<repub_model::Document>,
    /// Label and time of the last coalesced undo entry.
    last_coalesced_undo: Option<(String, std::time::Instant)>,
    /// In-place text editing session, if a text frame is being edited.
    pub text_edit: Option<crate::text_edit::TextEditState>,
    /// Last text copied or cut while editing (mirrors the system clipboard).
    pub text_clipboard: String,
    /// Master page being edited, if any: while set, the "current page" is the
    /// master (see [`MasterEdit`]).
    pub master_edit: Option<MasterEdit>,
    /// Measurement unit for positions and sizes (Format pane, ribbon, status bar).
    pub units: DisplayUnit,
    /// Create Link is waiting for a click: the text box whose text continues.
    pub link_source: Option<ObjectId>,
    /// A message for the user (title, text), shown as an alert by the app.
    pub alert: Option<(String, String)>,
    /// A hyperlink Cmd/Ctrl+clicked on the canvas, followed by the app.
    pub follow_link: Option<repub_model::text::Hyperlink>,
    /// Formatting held by the Format Painter, if active.
    pub copied_format: Option<CopiedFormat>,
    /// Shape point-editing session, if active.
    pub edit_points: Option<EditPointsState>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            document: None,
            document_path: None,
            dirty: false,
            current_page: 0,
            zoom: 1.0,
            pan_offset: Point::ZERO,
            selection: Vec::new(),
            selected_pages: Vec::new(),
            page_drag: PageDragState::default(),
            last_clicked_page: None,
            current_tool: Tool::Select,
            history: CommandHistory::new(),
            show_rulers: true,
            show_guides: true,
            show_grid: false,
            grid_spacing: 9.0, // 0.125 inch (1/8") at 72 dpi
            snap_to_grid: false,
            snap_to_guides: true,
            snap_to_objects: true,
            guides_locked: false,
            two_page_spread: false,
            edit_wordart_requested: false,
            align_target: AlignTarget::default(),
            page_properties_requested: None,
            fit_page_pending: true,
            pointer_doc: None,
            pending_undo: None,
            last_coalesced_undo: None,
            text_edit: None,
            text_clipboard: String::new(),
            master_edit: None,
            units: DisplayUnit::default(),
            link_source: None,
            alert: None,
            follow_link: None,
            copied_format: None,
            edit_points: None,
        }
    }
}

impl AppState {
    /// Create a new application state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if a document is open.
    pub fn has_document(&self) -> bool {
        self.document.is_some()
    }

    /// Get the document title for the window.
    pub fn window_title(&self) -> String {
        let doc_name = if let Some(ref path) = self.document_path {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Untitled".to_string())
        } else if self.document.is_some() {
            "Untitled".to_string()
        } else {
            return "Repub".to_string();
        };

        let dirty_marker = if self.dirty { "* " } else { "" };
        format!("{}{} - Repub", dirty_marker, doc_name)
    }

    /// Get the current page, if any. While a master page is being edited this
    /// is the master, so every tool and command works on it unchanged.
    pub fn current_page(&self) -> Option<&repub_model::Page> {
        if let Some(edit) = &self.master_edit {
            return Some(&edit.page);
        }
        self.document.as_ref()?.pages().get(self.current_page)
    }

    /// Get the current page mutably, if any (the master while editing one).
    pub fn current_page_mut(&mut self) -> Option<&mut repub_model::Page> {
        if let Some(edit) = &mut self.master_edit {
            return Some(&mut edit.page);
        }
        self.document
            .as_mut()?
            .pages_mut()
            .get_mut(self.current_page)
    }

    /// Whether object `id` on the current page is locked, or on a locked layer.
    pub fn is_locked(&self, id: repub_model::ObjectId) -> bool {
        self.current_page().is_some_and(|page| {
            page.layers().iter().any(|l| {
                l.shapes()
                    .iter()
                    .any(|o| o.id() == id && (l.locked || o.common.locked))
            })
        })
    }

    /// Page `index` as drawn: the master being edited stands in for the current page.
    pub fn page_on_view(&self, index: usize) -> Option<&repub_model::Page> {
        match &self.master_edit {
            Some(edit) if index == self.current_page => Some(&edit.page),
            _ => self.document.as_ref()?.pages().get(index),
        }
    }

    /// Start editing master page `id` in place of the current page.
    pub fn begin_master_edit(&mut self, id: repub_model::MasterPageId) {
        self.end_master_edit();
        let Some(master) = self.document.as_ref().and_then(|d| d.master_page(id)) else {
            return;
        };
        self.master_edit = Some(MasterEdit {
            id,
            page: master.to_page(),
            synced: master.to_page(),
        });
        self.text_edit = None;
        self.selection.clear();
        self.fit_page_pending = true;
    }

    /// Stop editing the master page, keeping the changes.
    pub fn end_master_edit(&mut self) {
        if self.master_edit.is_some() {
            self.sync_master_edit();
            self.master_edit = None;
            self.text_edit = None;
            self.selection.clear();
            self.fit_page_pending = true;
        }
    }

    /// Keep the master being edited and the document in step. Edits made in the
    /// session are written to the document; if the document's master changed
    /// instead (undo, redo), the session reloads it. Called every frame and
    /// before anything snapshots or restores the document.
    pub fn sync_master_edit(&mut self) {
        let Some(edit) = self.master_edit.as_mut() else {
            return;
        };
        let Some(master) = self
            .document
            .as_mut()
            .and_then(|d| d.master_page_mut(edit.id))
        else {
            self.master_edit = None;
            return;
        };
        let doc_now = master.to_page();
        let same = |a: &repub_model::Page, b: &repub_model::Page| {
            a.layers() == b.layers() && a.guides == b.guides && a.margins == b.margins
        };
        if !same(&doc_now, &edit.synced) {
            // Changed in the document: show that
            let id = edit.page.id;
            edit.page = doc_now.clone();
            edit.page.id = id;
            edit.synced = doc_now;
        } else if !same(&edit.page, &edit.synced) {
            master.update_from_page(&edit.page);
            edit.synced = master.to_page();
            self.dirty = true;
        }
    }

    /// Get total page count.
    pub fn page_count(&self) -> usize {
        self.document.as_ref().map(|d| d.pages().len()).unwrap_or(0)
    }

    /// Indices of the pages on view, left to right.
    ///
    /// In two-page spread view pages pair as in Publisher: page 1 stands alone
    /// (it is a right-hand page), then 2–3, 4–5, ... A trailing even page also
    /// stands alone. Otherwise only the current page is on view.
    pub fn visible_pages(&self) -> Vec<usize> {
        let count = self.page_count();
        let current = self.current_page;
        if current >= count {
            return Vec::new();
        }
        if !self.two_page_spread || count < 2 || current == 0 || self.master_edit.is_some() {
            return vec![current];
        }
        let left = if current % 2 == 1 {
            current
        } else {
            current - 1
        };
        if left + 1 < count {
            vec![left, left + 1]
        } else {
            vec![left]
        }
    }

    /// Size in points of the pages on view placed side by side (no gap).
    pub fn visible_pages_size(&self) -> Option<(f64, f64)> {
        let doc = self.document.as_ref()?;
        let sizes: Vec<_> = self
            .visible_pages()
            .into_iter()
            .filter_map(|i| doc.pages().get(i).map(|p| (p.size.width, p.size.height)))
            .collect();
        if sizes.is_empty() {
            return None;
        }
        let width = sizes.iter().map(|s| s.0).sum();
        let height = sizes.iter().map(|s| s.1).fold(0.0, f64::max);
        Some((width, height))
    }

    /// Set the current page (leaving master page editing).
    pub fn set_page(&mut self, index: usize) {
        self.end_master_edit();
        if let Some(ref doc) = self.document {
            if index < doc.pages().len() {
                self.current_page = index;
                self.selection.clear();
            }
        }
    }

    /// Mark the document as modified.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Snapshot the document so the change about to be made can be undone.
    /// Call immediately before mutating the document.
    pub fn record_undo(&mut self, label: &str) {
        self.sync_master_edit();
        if let Some(doc) = &self.document {
            self.history.record(label, doc.clone());
        }
    }

    /// Like [`Self::record_undo`], but successive calls with the same label within
    /// a second collapse into one entry (for continuous edits such as dragging a
    /// value in the format panel).
    pub fn record_undo_coalesced(&mut self, label: &str) {
        let now = std::time::Instant::now();
        let recent = self
            .last_coalesced_undo
            .as_ref()
            .is_some_and(|(l, t)| l == label && now.duration_since(*t).as_secs_f32() < 1.0);
        if !recent {
            self.record_undo(label);
        }
        self.last_coalesced_undo = Some((label.to_owned(), now));
    }

    /// Start an interactive edit (drag, resize, rotate, text editing): snapshot now,
    /// record on [`Self::end_edit`] only if the document actually changed.
    pub fn begin_edit(&mut self) {
        self.sync_master_edit();
        self.pending_undo = self.document.clone();
    }

    /// Finish an interactive edit started with [`Self::begin_edit`].
    pub fn end_edit(&mut self, label: &str) {
        self.sync_master_edit();
        if let Some(before) = self.pending_undo.take() {
            if self.document.as_ref() != Some(&before) {
                self.history.record(label, before);
                self.dirty = true;
            }
        }
    }

    /// Undo the last change. Returns true if something was undone.
    pub fn undo(&mut self) -> bool {
        self.sync_master_edit();
        self.pending_undo = None;
        self.last_coalesced_undo = None;
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let undone = self.history.undo(doc);
        if undone {
            self.after_history_change();
        }
        undone
    }

    /// Redo the last undone change. Returns true if something was redone.
    pub fn redo(&mut self) -> bool {
        self.sync_master_edit();
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let redone = self.history.redo(doc);
        if redone {
            self.after_history_change();
        }
        redone
    }

    /// Keep selection and page index valid after the document was swapped.
    fn after_history_change(&mut self) {
        self.sync_master_edit();
        self.dirty = true;
        let pages = self.page_count();
        if self.current_page >= pages {
            self.current_page = pages.saturating_sub(1);
        }
        let ids: Vec<ObjectId> = self.selection.clone();
        let page = self.current_page();
        self.selection = ids
            .into_iter()
            .filter(|id| page.is_some_and(|p| p.get_shape(*id).is_some()))
            .collect();
        self.selected_pages.retain(|&p| p < pages);
    }

    /// Mark the document as saved.
    pub fn mark_saved(&mut self, path: PathBuf) {
        self.document_path = Some(path);
        self.dirty = false;
    }

    /// Clear selection.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// Select an object.
    /// Copy the first selected object's formatting for the Format Painter.
    pub fn copy_format_from_selection(&mut self) -> bool {
        let Some(&id) = self.selection.first() else {
            return false;
        };
        let Some(obj) = self.current_page().and_then(|p| p.get_shape(id)) else {
            return false;
        };
        self.copied_format = Some(match &obj.kind {
            ShapeKind::TextFrame(tf) => CopiedFormat::Text {
                shape: ShapeFormat::from_shape(obj),
                character: tf
                    .content
                    .paragraphs
                    .first()
                    .map(|p| p.style.default_char_style.clone())
                    .unwrap_or_default(),
            },
            _ => CopiedFormat::Shape(ShapeFormat::from_shape(obj)),
        });
        true
    }

    /// Whether the Format Painter is waiting to apply formatting.
    pub fn is_format_painter_active(&self) -> bool {
        self.copied_format.is_some()
    }

    /// Clear the Format Painter.
    pub fn deactivate_format_painter(&mut self) {
        self.copied_format = None;
    }

    /// Apply the copied formatting to an object. Returns true when applied.
    pub fn apply_copied_format_to(&mut self, id: ObjectId) -> bool {
        let Some(copied) = self.copied_format.clone() else {
            return false;
        };
        let Some(page) = self.current_page_mut() else {
            return false;
        };
        let Some(obj) = page.get_shape_mut(id) else {
            return false;
        };
        let format = match &copied {
            CopiedFormat::Shape(style) => style.clone(),
            CopiedFormat::Text { shape, .. } => shape.clone(),
        };
        format.apply_to(obj);
        if let (ShapeKind::TextFrame(tf), CopiedFormat::Text { character, .. }) =
            (&mut obj.kind, &copied)
        {
            for para in &mut tf.content.paragraphs {
                para.style.default_char_style = character.clone();
            }
        }
        self.mark_dirty();
        true
    }

    pub fn select(&mut self, id: ObjectId, add_to_selection: bool) {
        if add_to_selection {
            if !self.selection.contains(&id) {
                self.selection.push(id);
            }
        } else {
            self.selection.clear();
            self.selection.push(id);
        }
    }

    /// Check if an object is selected.
    pub fn is_selected(&self, id: ObjectId) -> bool {
        self.selection.contains(&id)
    }

    /// Toggle selection of an object.
    pub fn toggle_selection(&mut self, id: ObjectId) {
        if let Some(pos) = self.selection.iter().position(|&x| x == id) {
            self.selection.remove(pos);
        } else {
            self.selection.push(id);
        }
    }

    /// Get selected objects.
    pub fn selected_objects(&self) -> Vec<&Shape> {
        // In stacking order, looking inside groups for selected members
        fn collect<'a>(
            obj: &'a Shape,
            selection: &[repub_model::ObjectId],
            out: &mut Vec<&'a Shape>,
        ) {
            if selection.contains(&obj.id()) {
                out.push(obj);
            } else if let ShapeKind::Group(group) = &obj.kind {
                for child in &group.children {
                    collect(child, selection, out);
                }
            }
        }
        let mut objects = Vec::new();
        if let Some(page) = self.current_page() {
            for layer in page.layers() {
                for obj in layer.shapes() {
                    collect(obj, &self.selection, &mut objects);
                }
            }
        }
        objects
    }

    // ==================== Page Selection ====================

    /// Clear page selection.
    pub fn clear_page_selection(&mut self) {
        self.selected_pages.clear();
    }

    /// Select a page (single or add to selection).
    pub fn select_page(&mut self, index: usize, add_to_selection: bool) {
        if add_to_selection {
            if !self.selected_pages.contains(&index) {
                self.selected_pages.push(index);
            }
        } else {
            self.selected_pages.clear();
            self.selected_pages.push(index);
        }
        self.last_clicked_page = Some(index);
    }

    /// Toggle page selection.
    pub fn toggle_page_selection(&mut self, index: usize) {
        if let Some(pos) = self.selected_pages.iter().position(|&x| x == index) {
            self.selected_pages.remove(pos);
        } else {
            self.selected_pages.push(index);
        }
        self.last_clicked_page = Some(index);
    }

    /// Select a range of pages (for shift-click).
    pub fn select_page_range(&mut self, to_index: usize) {
        let from_index = self.last_clicked_page.unwrap_or(0);
        let (start, end) = if from_index <= to_index {
            (from_index, to_index)
        } else {
            (to_index, from_index)
        };

        self.selected_pages.clear();
        for i in start..=end {
            self.selected_pages.push(i);
        }
    }

    /// Check if a page is selected.
    pub fn is_page_selected(&self, index: usize) -> bool {
        self.selected_pages.contains(&index)
    }

    /// Select all pages.
    pub fn select_all_pages(&mut self) {
        let page_count = self.page_count();
        self.selected_pages.clear();
        for i in 0..page_count {
            self.selected_pages.push(i);
        }
    }

    /// Delete all selected pages (undoable; at least one page always remains).
    pub fn delete_selected_pages(&mut self) {
        let pages = self.selected_pages.clone();
        self.delete_pages(&pages);
    }

    /// Insert two blank pages after the spread holding page `index` (undoable)
    /// and go to the new spread.
    pub fn insert_spread_after(&mut self, index: usize) {
        if !self.has_document() {
            return;
        }
        let previous = self.current_page;
        self.current_page = index;
        let at = self.visible_pages().last().copied().unwrap_or(index) + 1;
        self.current_page = previous;
        self.record_undo("Insert Spread");
        if let Some(doc) = self.document.as_mut() {
            let at = at.min(doc.pages().len());
            doc.insert_page(at);
            doc.insert_page(at + 1);
            self.current_page = at;
        }
        self.selection.clear();
        self.clear_page_selection();
        self.mark_dirty();
    }

    /// Insert a blank page at `index` (undoable) and go to it.
    pub fn insert_page_at(&mut self, index: usize) {
        if !self.has_document() {
            return;
        }
        self.record_undo("Insert Page");
        if let Some(doc) = self.document.as_mut() {
            let index = index.min(doc.pages().len());
            if doc.insert_page(index).is_some() {
                self.current_page = index;
                self.selection.clear();
                self.clear_page_selection();
                self.mark_dirty();
            }
        }
    }

    /// Duplicate each page in `indices` (objects included), placing every copy
    /// right after its original (undoable). Goes to the copy of the first page.
    pub fn duplicate_pages(&mut self, indices: &[usize]) {
        let mut indices: Vec<usize> = indices.to_vec();
        indices.sort_unstable();
        indices.dedup();
        let count = self.page_count();
        indices.retain(|&i| i < count);
        if indices.is_empty() {
            return;
        }
        self.record_undo(if indices.len() == 1 {
            "Duplicate Page"
        } else {
            "Duplicate Pages"
        });
        if let Some(doc) = self.document.as_mut() {
            // From the end, so earlier indices stay valid
            for &i in indices.iter().rev() {
                doc.duplicate_page(i);
            }
        }
        // Copies before the first one shift it down by one each
        self.current_page = indices[0] + 1;
        self.selection.clear();
        self.clear_page_selection();
        self.mark_dirty();
    }

    /// Delete the pages in `indices` (undoable). Nothing happens if that would
    /// leave the publication with no pages.
    pub fn delete_pages(&mut self, indices: &[usize]) {
        let mut indices: Vec<usize> = indices.to_vec();
        indices.sort_unstable();
        indices.dedup();
        let count = self.page_count();
        indices.retain(|&i| i < count);
        if indices.is_empty() || indices.len() >= count {
            return;
        }
        self.record_undo(if indices.len() == 1 {
            "Delete Page"
        } else {
            "Delete Pages"
        });
        let below_current = indices.iter().filter(|&&i| i < self.current_page).count();
        let current_deleted = indices.contains(&self.current_page);
        if let Some(doc) = self.document.as_mut() {
            for &i in indices.iter().rev() {
                doc.remove_page(i);
            }
            let remaining = doc.pages().len();
            self.current_page = (self.current_page - below_current).min(remaining - 1);
            if current_deleted {
                self.current_page = self.current_page.min(remaining - 1);
            }
        }
        self.selection.clear();
        self.clear_page_selection();
        self.mark_dirty();
    }

    /// Set the zoom (clamped to 10%–800%) keeping the centre of the view still.
    /// The pages are centred at the view centre plus `pan_offset`, so the pan
    /// scales with the zoom.
    pub fn set_zoom_centered(&mut self, zoom: f32) {
        let zoom = zoom.clamp(0.1, 8.0);
        let ratio = (zoom / self.zoom) as f64;
        self.pan_offset.x *= ratio;
        self.pan_offset.y *= ratio;
        self.zoom = zoom;
    }

    /// Zoom in one step, about the centre of the view.
    pub fn zoom_in(&mut self) {
        self.set_zoom_centered(self.zoom * 1.25);
    }

    /// Zoom out one step, about the centre of the view.
    pub fn zoom_out(&mut self) {
        self.set_zoom_centered(self.zoom / 1.25);
    }

    /// Zoom to fit.
    pub fn zoom_to_fit(&mut self, viewport_width: f32, viewport_height: f32) {
        if let Some((width, height)) = self.visible_pages_size() {
            let page_width = width as f32;
            let page_height = height as f32;

            // Leave a workspace border around the page, as Publisher's Whole Page view does
            let zoom_x = (viewport_width - 64.0) / page_width;
            let zoom_y = (viewport_height - 64.0) / page_height;

            self.zoom = zoom_x.min(zoom_y).max(0.1).min(8.0);
        }
    }

    /// Set zoom to actual size (100%).
    pub fn zoom_actual(&mut self) {
        self.set_zoom_centered(1.0);
    }

    /// Convert screen coordinates to document coordinates.
    pub fn screen_to_doc(&self, screen_pos: Point) -> Point {
        Point::new(
            (screen_pos.x - self.pan_offset.x) / self.zoom as f64,
            (screen_pos.y - self.pan_offset.y) / self.zoom as f64,
        )
    }

    /// Convert document coordinates to screen coordinates.
    pub fn doc_to_screen(&self, doc_pos: Point) -> Point {
        Point::new(
            doc_pos.x * self.zoom as f64 + self.pan_offset.x,
            doc_pos.y * self.zoom as f64 + self.pan_offset.y,
        )
    }

    /// The current page's ruler guides.
    pub fn guides(&self) -> Vec<Guide> {
        self.current_page().map_or_else(Vec::new, |p| {
            p.guides.iter().map(|g| Guide::from_model(*g)).collect()
        })
    }

    /// Add a guide to the current page.
    pub fn add_guide(&mut self, guide: Guide) {
        self.record_undo("Add Guide");
        if let Some(page) = self.current_page_mut() {
            page.guides.push(guide.to_model());
            self.mark_dirty();
        }
    }

    /// Remove a guide at the given index.
    pub fn remove_guide(&mut self, index: usize) {
        if index < self.guides().len() && !self.guides_locked {
            self.record_undo("Delete Guide");
            if let Some(page) = self.current_page_mut() {
                page.guides.remove(index);
            }
            self.mark_dirty();
        }
    }

    /// Update a guide's position.
    pub fn update_guide(&mut self, index: usize, position: f64) {
        if index < self.guides().len() && !self.guides_locked && !self.guides()[index].locked {
            self.record_undo_coalesced("Move Guide");
            if let Some(page) = self.current_page_mut() {
                page.guides[index].position = position;
            }
            self.mark_dirty();
        }
    }

    /// Find a guide near a document position.
    /// Returns the index and distance if found within tolerance.
    pub fn find_guide_at(&self, doc_pos: Point, tolerance: f64) -> Option<(usize, f64)> {
        for (i, guide) in self.guides().iter().enumerate() {
            let distance = match guide.orientation {
                GuideOrientation::Horizontal => (guide.position - doc_pos.y).abs(),
                GuideOrientation::Vertical => (guide.position - doc_pos.x).abs(),
            };
            if distance <= tolerance {
                return Some((i, distance));
            }
        }
        None
    }

    /// Take the document's saved measurement and snapping settings, unless they are the
    /// defaults of a document that never set any (then the app's own preferences stand).
    pub fn apply_document_settings(&mut self) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let s = doc.settings.clone();
        if s == repub_model::DocumentSettings::default() {
            return;
        }
        self.units = match s.display_unit {
            repub_geometry::Unit::Points => DisplayUnit::Points,
            repub_geometry::Unit::Millimeters | repub_geometry::Unit::Centimeters => {
                DisplayUnit::Millimeters
            }
            _ => DisplayUnit::Inches,
        };
        self.show_rulers = s.show_rulers;
        self.show_guides = s.show_guides;
        self.snap_to_grid = s.snap_to_grid;
        self.grid_spacing = s.grid_spacing;
        self.snap_to_guides = s.snap_to_guides;
        self.snap_to_objects = s.snap_to_objects;
    }

    /// Save the current measurement and snapping settings in the document.
    pub fn store_document_settings(&mut self) {
        let (units, rulers, guides, grid, spacing, snap_guides, snap_objects) = (
            self.units,
            self.show_rulers,
            self.show_guides,
            self.snap_to_grid,
            self.grid_spacing,
            self.snap_to_guides,
            self.snap_to_objects,
        );
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        doc.settings = repub_model::DocumentSettings {
            display_unit: match units {
                DisplayUnit::Points => repub_geometry::Unit::Points,
                DisplayUnit::Inches => repub_geometry::Unit::Inches,
                DisplayUnit::Millimeters => repub_geometry::Unit::Millimeters,
            },
            show_rulers: rulers,
            show_guides: guides,
            snap_to_grid: grid,
            grid_spacing: spacing,
            snap_to_guides: snap_guides,
            snap_to_objects: snap_objects,
        };
    }

    /// Get horizontal guides (for snapping).
    pub fn horizontal_guides(&self) -> Vec<f64> {
        self.guides()
            .iter()
            .filter(|g| g.orientation == GuideOrientation::Horizontal)
            .map(|g| g.position)
            .collect()
    }

    /// Get vertical guides (for snapping).
    pub fn vertical_guides(&self) -> Vec<f64> {
        self.guides()
            .iter()
            .filter(|g| g.orientation == GuideOrientation::Vertical)
            .map(|g| g.position)
            .collect()
    }

    /// Clear all guides.
    pub fn clear_guides(&mut self) {
        if !self.guides_locked && !self.guides().is_empty() {
            self.record_undo("Clear Guides");
            if let Some(page) = self.current_page_mut() {
                page.guides.clear();
            }
            self.mark_dirty();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_model::{PageOrientation, PageSize};

    #[test]
    fn test_app_state_default() {
        let state = AppState::new();
        assert!(!state.has_document());
        assert_eq!(state.zoom, 1.0);
    }

    #[test]
    fn test_window_title() {
        let mut state = AppState::new();
        assert_eq!(state.window_title(), "Repub");

        state.document = Some(Document::new(PageSize::Letter, PageOrientation::Portrait));
        assert_eq!(state.window_title(), "Untitled - Repub");

        state.dirty = true;
        assert_eq!(state.window_title(), "* Untitled - Repub");
    }

    #[test]
    fn test_zoom() {
        let mut state = AppState::new();
        state.zoom_in();
        assert!(state.zoom > 1.0);

        state.zoom_out();
        state.zoom_out();
        assert!(state.zoom < 1.0);

        state.zoom_actual();
        assert_eq!(state.zoom, 1.0);
    }

    #[test]
    fn document_settings_round_trip_through_the_document() {
        let mut state = AppState::new();
        state.document = Some(Document::new(PageSize::Letter, PageOrientation::Portrait));
        // A document that never set anything leaves the app's preferences alone
        state.units = DisplayUnit::Points;
        state.apply_document_settings();
        assert_eq!(state.units, DisplayUnit::Points);

        state.units = DisplayUnit::Millimeters;
        state.grid_spacing = 12.0;
        state.snap_to_objects = false;
        state.store_document_settings();
        state.units = DisplayUnit::Inches;
        state.grid_spacing = 9.0;
        state.snap_to_objects = true;
        state.apply_document_settings();
        assert_eq!(state.units, DisplayUnit::Millimeters);
        assert_eq!(state.grid_spacing, 12.0);
        assert!(!state.snap_to_objects);
    }

    #[test]
    fn guides_belong_to_their_page_and_can_be_undone() {
        let mut state = AppState::new();
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        doc.add_page();
        state.document = Some(doc);
        state.add_guide(Guide::horizontal(100.0));
        assert_eq!(state.guides().len(), 1);
        state.current_page = 1;
        assert!(state.guides().is_empty());
        state.current_page = 0;
        assert!(state.undo());
        assert!(state.guides().is_empty());
    }

    #[test]
    fn test_guides() {
        let mut state = AppState::new();
        state.document = Some(Document::new(PageSize::Letter, PageOrientation::Portrait));
        state.dirty = false;

        // Add guides
        state.add_guide(Guide::horizontal(100.0));
        assert_eq!(state.guides().len(), 1);
        assert!(state.dirty);

        state.dirty = false;
        state.add_guide(Guide::vertical(50.0));
        assert_eq!(state.guides().len(), 2);

        // Test guide retrieval
        assert_eq!(state.horizontal_guides(), vec![100.0]);
        assert_eq!(state.vertical_guides(), vec![50.0]);

        // Test find_guide_at
        let (idx, _) = state.find_guide_at(Point::new(52.0, 200.0), 5.0).unwrap();
        assert_eq!(idx, 1); // vertical guide at x=50

        let (idx, _) = state.find_guide_at(Point::new(200.0, 98.0), 5.0).unwrap();
        assert_eq!(idx, 0); // horizontal guide at y=100

        // Test update_guide
        state.dirty = false;
        state.update_guide(0, 150.0);
        assert_eq!(state.guides()[0].position, 150.0);
        assert!(state.dirty);

        // Test remove_guide
        state.dirty = false;
        state.remove_guide(0);
        assert_eq!(state.guides().len(), 1);
        assert!(state.dirty);

        // Test clear_guides
        state.dirty = false;
        state.add_guide(Guide::horizontal(200.0));
        state.dirty = false;
        state.clear_guides();
        assert!(state.guides().is_empty());
        assert!(state.dirty);
    }

    #[test]
    fn test_guides_locked() {
        let mut state = AppState::new();
        state.document = Some(Document::new(PageSize::Letter, PageOrientation::Portrait));

        state.add_guide(Guide::horizontal(100.0));
        state.guides_locked = true;

        // Should not modify when locked
        state.dirty = false;
        state.update_guide(0, 200.0);
        assert_eq!(state.guides()[0].position, 100.0);
        assert!(!state.dirty);

        state.remove_guide(0);
        assert_eq!(state.guides().len(), 1);

        state.clear_guides();
        assert_eq!(state.guides().len(), 1);
    }
}

/// A master page being edited. `page` is what the tools edit; `synced` is the
/// master's content as last written to (or read from) the document.
#[derive(Debug, Clone)]
pub struct MasterEdit {
    pub id: repub_model::MasterPageId,
    pub page: repub_model::Page,
    synced: repub_model::Page,
}

/// Measurement unit shown in the Format pane, ribbon Size fields and status bar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DisplayUnit {
    Points,
    /// Publisher's default (US)
    #[default]
    Inches,
    Millimeters,
}

impl DisplayUnit {
    /// Convert from points to this unit.
    pub fn from_points(&self, pts: f64) -> f64 {
        match self {
            DisplayUnit::Points => pts,
            DisplayUnit::Inches => pts / 72.0,
            DisplayUnit::Millimeters => pts / 72.0 * 25.4,
        }
    }

    /// Convert to points from this unit.
    pub fn to_points(&self, value: f64) -> f64 {
        match self {
            DisplayUnit::Points => value,
            DisplayUnit::Inches => value * 72.0,
            DisplayUnit::Millimeters => value / 25.4 * 72.0,
        }
    }

    /// Get the unit suffix.
    pub fn suffix(&self) -> &'static str {
        match self {
            DisplayUnit::Points => " pt",
            DisplayUnit::Inches => " in",
            DisplayUnit::Millimeters => " mm",
        }
    }

    /// Get display name.
    pub fn name(&self) -> &'static str {
        match self {
            DisplayUnit::Points => "Points",
            DisplayUnit::Inches => "Inches",
            DisplayUnit::Millimeters => "Millimeters",
        }
    }
}

impl DisplayUnit {
    /// Short name stored in settings.
    pub fn key(&self) -> &'static str {
        match self {
            DisplayUnit::Points => "pt",
            DisplayUnit::Inches => "in",
            DisplayUnit::Millimeters => "mm",
        }
    }

    pub fn from_key(key: &str) -> Self {
        match key {
            "pt" => DisplayUnit::Points,
            "mm" => DisplayUnit::Millimeters,
            _ => DisplayUnit::Inches,
        }
    }

    /// `points` in this unit with the usual precision and suffix ("1.25 in.").
    pub fn format(&self, points: f64) -> String {
        match self {
            DisplayUnit::Points => format!("{:.1} pt", points),
            DisplayUnit::Inches => format!("{:.2} in.", points / 72.0),
            DisplayUnit::Millimeters => format!("{:.1} mm", self.from_points(points)),
        }
    }
}
