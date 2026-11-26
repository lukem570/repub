//! Main application.

use std::path::{Path, PathBuf};

use repub_geometry::Point;
use repub_model::shape::{Shape, ShapeKind};
use repub_model::Document;
use repub_model::ObjectId;

use crate::canvas::Canvas;
use crate::dialogs::{
    AboutDialog, DocumentPropertiesDialog, ErrorAlert, ExportDialog, FindReplaceDialog, FontDialog,
    FormatShapeDialog, HyperlinkDialog, InsertTableDialog, MovePageDialog, NewDocumentDialog,
    PagePropertiesDialog, ParagraphDialog, RenamePageDialog, SaveChangesPrompt, SaveChoice,
    SpellingDialog, TablePropertiesDialog, TemplatesDialog, TextBoxMarginsDialog,
    TextColumnsDialog, TextFitDialog, TextWrapDialog, WordArtDialog,
};
use crate::panels::{PagesPanel, Toolbar};
use crate::settings::Settings;
use crate::start_screen::{DocumentInfo, StartPage, StartScreen, StartScreenAction};
use crate::state::{AppState, Guide, GuideOrientation};
use crate::text_format::{self, CharChange, Toggle};
use crate::tools::Tool;
use crate::widgets::{
    Ruler, RulerInteraction, RulerOrientation, StatusBar, StatusBarAction, StatusBarInfo,
};
use repub_model::styles::TextAlignment;

/// Clipboard for copy/paste operations.
#[derive(Debug, Default)]
pub struct Clipboard {
    /// Copied objects
    pub objects: Vec<Shape>,
    /// Original positions (for paste in place)
    pub original_positions: Vec<Point>,
}

/// Z-order reordering operations.
#[derive(Debug, Clone, Copy)]
enum ObjectReorder {
    BringToFront,
    SendToBack,
    BringForward,
    SendBackward,
}

#[derive(Debug, Clone)]
pub enum AppAction {
    // File actions
    NewDocument,
    OpenDocument,
    SaveDocument,
    SaveDocumentAs,
    Export,
    /// Print: the publication as a PDF, opened in the system's viewer to print.
    Print,
    Quit,

    // Edit actions
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    PasteInPlace,
    Duplicate,
    Delete,
    SelectAll,
    /// Copy formatting for the Format Painter
    FormatPainter,
    /// Open Find and Replace
    Find,
    /// Open Find and Replace in replace mode
    Replace,

    // View actions
    ZoomIn,
    ZoomOut,
    ZoomActual,
    ZoomFit,
    ToggleRulers,
    ToggleGuides,
    ToggleGrid,
    ToggleSnapToGrid,
    ToggleSnapToGuides,
    ClearGuides,
    /// Switch between the light, dark and system themes
    SetTheme(crate::theme::ThemeMode),

    // Insert actions
    InsertText,
    InsertImage,
    InsertRectangle,
    InsertEllipse,
    InsertLine,
    InsertPolygon,
    InsertStar,
    InsertTable,
    InsertPageNumber,
    /// Insert a field (page count, date, time, file name) at the caret or in the selected text box
    InsertField(repub_model::text::Field),
    /// Insert a hyperlink into the selected text or frame
    InsertHyperlink,
    /// Replace the picture in the selected image frame
    ChangePicture,
    /// Check spelling
    CheckSpelling,
    /// Edit the selected shape's path points
    EditShapePoints,
    AddPage,
    /// Insert a blank page at this index.
    InsertPageAt(usize),
    /// Insert two blank pages after the spread holding this page.
    InsertSpread(usize),
    /// Copy this page (objects included) to right after it.
    DuplicatePage(usize),
    /// Delete this page (never the last one).
    DeletePage(usize),
    MovePage,
    RenamePage,

    // Arrange actions
    Group,
    Ungroup,
    BringToFront,
    SendToBack,
    BringForward,
    SendBackward,
    AlignLeft,
    AlignCenter,
    AlignRight,
    AlignTop,
    AlignMiddle,
    AlignBottom,

    // Text formatting
    TextAlignLeft,
    TextAlignCenter,
    TextAlignRight,
    TextAlignJustify,
    TextBold,
    TextItalic,
    TextUnderline,
    TextSuperscript,
    TextSubscript,
    /// Bullets / numbering on the target paragraphs (toggle).
    ToggleBullets,
    ToggleNumbering,
    /// Next size up / down in the font size list.
    GrowFont,
    ShrinkFont,
    /// Set the font size (points).
    SetFontSize(f64),
    /// Font family for the selected text (or whole boxes)
    SetFontFamily(String),
    SetFontColor(repub_color::Color),
    /// Increase (`true`) or decrease the paragraph indent.
    ChangeIndent(bool),
    /// Line spacing as a multiple of single spacing.
    SetLineSpacing(f64),
    /// Pick a picture file and place it on the page.
    InsertPicture,
    /// Page size for every page, portrait width and height in points (orientation kept).
    SetPageSize {
        width: f64,
        height: f64,
    },
    /// Orientation for every page (`true` = landscape).
    SetOrientation(bool),
    /// Equal margins on every side of every page, points.
    SetMargins(f64),
    /// Background colour of the current page (`None` = no background).
    SetPageBackground(Option<repub_color::Color>),
    /// Fill colour of the selected shapes (`None` = no fill).
    SetShapeFill(Option<repub_color::Color>),
    /// Outline colour of the selected shapes and lines (`None` = no outline).
    SetShapeOutline(Option<repub_color::Color>),
    /// Outline weight of the selected shapes and lines, points.
    SetOutlineWeight(f64),
    /// Mirror the selection left to right (`true`) or top to bottom.
    Flip(bool),
    /// Start linking the selected text box to the next one clicked.
    CreateTextLink,
    /// Stop the selected text box's text flowing on to the next box.
    BreakTextLink,
    /// Add a row / column at the end of the selected table.
    InsertTableRow,
    InsertTableColumn,
    /// Change the selected table's rows/columns at the cell being edited.
    TableEdit(TableEdit),
    /// Leave a one-click mode (Create Link, Format Painter).
    CancelMode,
    /// Rotate the selection by this many degrees (clockwise).
    RotateBy(f64),
    /// Start editing the selected text box's text (caret at the end).
    EditText,

    // View modes
    ToggleTwoPageSpread,

    // Help
    ShowAbout,

    // Dialogs
    ShowPageProperties,
    /// Text wrapping options for the selected object
    ShowTextWrap,
    /// Fill, line, effects, size and properties of the selected objects
    ShowFormatShape,
    /// Title, author, units, grid and snapping for the document
    ShowDocumentProperties,
    /// Every character property of the selected text
    ShowFontDialog,
    /// Alignment, indents, spacing and lists of the selected paragraphs
    ShowParagraphDialog,
    /// Borders, sizes and cell options of the selected table
    ShowTableProperties,
    /// Add WordArt of this warp shape and open it for editing
    InsertWordArt(repub_model::shape::WordArtPreset),
    /// Text, font, shape and 3-D look of the selected WordArt
    ShowWordArt,
    /// Do something to the selected point while editing a shape's points
    EditPoint(crate::edit_points::PointOp),
    /// Space the selected objects evenly across (`true`) or down the page
    Distribute(bool),
    /// What Align lines objects up to
    SetAlignTarget(crate::state::AlignTarget),
    /// Choose a template and fill it in to start a new publication
    NewFromTemplate,
    /// Auto-fit options for the selected text frame
    ShowTextFit,
    /// Column options for the selected text frame
    ShowTextColumns,
    /// Margin options for the selected text frame
    ShowTextBoxMargins,
    /// File tab: open the Backstage view
    ShowBackstage,

    // Panes
    TogglePagesPane,
    /// Show or hide the Selection pane.
    ToggleSelectionPane,

    // Master Pages
    /// Edit a master page (switch to master page editing mode)
    EditMasterPage(Option<repub_model::MasterPageId>),
    /// Apply a master page to the current page
    ApplyMasterPage(Option<repub_model::MasterPageId>),
    /// Create a new master page
    NewMasterPage,
    /// Exit master page editing mode
    ExitMasterPageEdit,
}

/// Table row/column commands (Table Layout tab, right-click menu). They act
/// at the cell being edited; without one, inserts go at the table's end
/// (Above/Left at its start) and deletes aren't available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableEdit {
    RowAbove,
    RowBelow,
    ColumnLeft,
    ColumnRight,
    DeleteRow,
    DeleteColumn,
    DeleteTable,
}

impl TableEdit {
    pub fn label(self) -> &'static str {
        match self {
            TableEdit::RowAbove => "Insert Row Above",
            TableEdit::RowBelow => "Insert Row Below",
            TableEdit::ColumnLeft => "Insert Column Left",
            TableEdit::ColumnRight => "Insert Column Right",
            TableEdit::DeleteRow => "Delete Row",
            TableEdit::DeleteColumn => "Delete Column",
            TableEdit::DeleteTable => "Delete Table",
        }
    }

    /// Whether it can run now: a table is selected, and deleting a row or
    /// column needs a cell being edited.
    pub fn enabled(self, state: &AppState) -> bool {
        let table =
            matches!(state.selected_objects()[..], [o] if matches!(o.kind, ShapeKind::Table(_)));
        let in_cell = state.text_edit.as_ref().is_some_and(|e| e.cell.is_some());
        match self {
            TableEdit::DeleteRow | TableEdit::DeleteColumn => table && in_cell,
            _ => table,
        }
    }
}

/// The native menu bar and its event channel (desktop only).
#[cfg(not(target_arch = "wasm32"))]
type NativeMenu = (muda::Menu, std::sync::mpsc::Receiver<muda::MenuEvent>);
/// There is no native menu bar in the browser.
#[cfg(target_arch = "wasm32")]
type NativeMenu = ();

#[cfg(target_arch = "wasm32")]
mod web_io;

/// Main Repub application.
pub struct RepubApp {
    /// Application state
    state: AppState,
    /// Settings
    settings: Settings,
    /// Canvas
    canvas: Canvas,
    /// Toolbar
    toolbar: Toolbar,
    /// New document dialog
    new_doc_dialog: NewDocumentDialog,
    /// Start screen (no document) and Backstage (File tab)
    pub start_screen: StartScreen,
    /// Export dialog
    export_dialog: ExportDialog,
    /// About dialog
    about_dialog: AboutDialog,
    /// Page properties dialog
    page_properties_dialog: PagePropertiesDialog,
    /// Insert table dialog
    insert_table_dialog: InsertTableDialog,
    /// Text fit (auto-size) dialog
    text_fit_dialog: TextFitDialog,
    /// Text columns dialog
    text_columns_dialog: TextColumnsDialog,
    /// Text wrap dialog
    text_wrap_dialog: TextWrapDialog,
    format_shape_dialog: FormatShapeDialog,
    document_properties_dialog: DocumentPropertiesDialog,
    font_dialog: FontDialog,
    paragraph_dialog: ParagraphDialog,
    table_properties_dialog: TablePropertiesDialog,
    wordart_dialog: WordArtDialog,
    templates_dialog: TemplatesDialog,
    /// Text box margins dialog
    text_box_margins_dialog: TextBoxMarginsDialog,
    /// Find and replace dialog
    find_replace_dialog: FindReplaceDialog,
    /// Insert hyperlink dialog
    hyperlink_dialog: HyperlinkDialog,
    /// Spelling dialog
    spelling_dialog: SpellingDialog,
    /// Move page dialog
    move_page_dialog: MovePageDialog,
    /// Rename page dialog
    rename_page_dialog: RenamePageDialog,
    /// Pending file save path (for file dialogs)
    pending_save_path: Option<PathBuf>,
    /// "Save changes?" prompt shown before unsaved work would be discarded
    save_prompt: SaveChangesPrompt,
    /// Operation waiting on the save prompt's answer
    pending_discard: Option<DiscardOp>,
    /// Set once the user agreed to quit, so the window close isn't intercepted again
    allow_close: bool,
    /// Error message box (open/save/export failures)
    error_alert: ErrorAlert,
    /// Page whose thumbnail was last right-clicked (target of the page context menu)
    context_page: usize,
    /// Rulers/grid/guides as last shown in the native View menu
    view_checks: Option<[bool; 6]>,
    /// Clipboard for copy/paste
    clipboard: Clipboard,
    /// Format panel state
    /// Pages panel
    pages_panel: PagesPanel,
    /// Viewport size used by the last whole-page fit, to detect when layout has settled
    last_fit_viewport: Option<egui::Vec2>,
    /// Native menu bar (macOS/Windows/Linux)
    #[cfg(not(target_arch = "wasm32"))]
    native_menu: Option<muda::Menu>,
    /// Native menu event receiver
    #[cfg(not(target_arch = "wasm32"))]
    native_menu_rx: Option<std::sync::mpsc::Receiver<muda::MenuEvent>>,
    /// Native context menus (macOS)
    #[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
    native_context_menus: Option<crate::native_menu::NativeContextMenus>,
}

impl RepubApp {
    /// Create a new application.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::install_graphics(cc);
        Self::build(&cc.egui_ctx, Settings::load(), None)
    }

    /// Create a new application with native menu support.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn with_native_menu(
        cc: &eframe::CreationContext<'_>,
        menu: muda::Menu,
        menu_rx: std::sync::mpsc::Receiver<muda::MenuEvent>,
    ) -> Self {
        Self::install_graphics(cc);
        Self::build(&cc.egui_ctx, Settings::load(), Some((menu, menu_rx)))
    }

    /// Create an application with no window, for tests driving it with synthetic input
    /// through [`Self::frame`]. Settings stay in memory and are never written to disk,
    /// and no native (AppKit) menus are created.
    pub fn new_headless(ctx: &egui::Context) -> Self {
        Self::build(ctx, Settings::in_memory(), None)
    }

    fn install_graphics(cc: &eframe::CreationContext<'_>) {
        // SDF canvas text needs the wgpu backend; without it the canvas falls back to egui text
        if let Some(render_state) = cc.wgpu_render_state.as_ref() {
            crate::sdf_text::install(render_state);
        }
    }

    fn build(ctx: &egui::Context, settings: Settings, native_menu: Option<NativeMenu>) -> Self {
        crate::theme::init(ctx);
        crate::theme::apply_theme(ctx, settings.theme);

        let mut state = AppState::new();
        state.show_rulers = settings.show_rulers;
        state.show_guides = settings.show_guides;
        state.zoom = settings.default_zoom;
        state.units = crate::state::DisplayUnit::from_key(&settings.measurement_unit);

        #[cfg(target_arch = "wasm32")]
        crate::platform::web::set_context(ctx);

        #[cfg(not(target_arch = "wasm32"))]
        let has_native_menu = native_menu.is_some();
        #[cfg(not(target_arch = "wasm32"))]
        let (native_menu, native_menu_rx) = match native_menu {
            Some((menu, rx)) => (Some(menu), Some(rx)),
            None => (None, None),
        };
        #[cfg(target_arch = "wasm32")]
        let _ = native_menu;

        Self {
            state,
            settings,
            canvas: Canvas::new(),
            toolbar: Toolbar::new(),
            new_doc_dialog: NewDocumentDialog::new(),
            start_screen: StartScreen::new(),
            export_dialog: ExportDialog::new(),
            about_dialog: AboutDialog::new(),
            page_properties_dialog: PagePropertiesDialog::new(),
            insert_table_dialog: InsertTableDialog::new(),
            text_fit_dialog: TextFitDialog::new(),
            text_columns_dialog: TextColumnsDialog::new(),
            text_wrap_dialog: TextWrapDialog::new(),
            format_shape_dialog: FormatShapeDialog::new(),
            document_properties_dialog: DocumentPropertiesDialog::new(),
            font_dialog: FontDialog::new(),
            paragraph_dialog: ParagraphDialog::new(),
            table_properties_dialog: TablePropertiesDialog::new(),
            wordart_dialog: WordArtDialog::new(),
            templates_dialog: TemplatesDialog::new(),
            text_box_margins_dialog: TextBoxMarginsDialog::new(),
            find_replace_dialog: FindReplaceDialog::new(),
            hyperlink_dialog: HyperlinkDialog::new(),
            spelling_dialog: SpellingDialog::new(),
            move_page_dialog: MovePageDialog::new(),
            rename_page_dialog: RenamePageDialog::new(),
            pending_save_path: None,
            save_prompt: SaveChangesPrompt::default(),
            pending_discard: None,
            allow_close: false,
            error_alert: ErrorAlert::default(),
            context_page: 0,
            view_checks: None,
            clipboard: Clipboard::default(),
            pages_panel: PagesPanel::new(),
            last_fit_viewport: None,
            #[cfg(not(target_arch = "wasm32"))]
            native_menu,
            #[cfg(not(target_arch = "wasm32"))]
            native_menu_rx,
            // AppKit menus need the main thread; only create them alongside the menu bar
            #[cfg(all(not(target_arch = "wasm32"), target_os = "macos"))]
            native_context_menus: has_native_menu.then(crate::native_menu::NativeContextMenus::new),
        }
    }

    /// Read-only access to the application state (for tests and integrations).
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Mutable access to the application state (for tests and integrations).
    pub fn state_mut(&mut self) -> &mut AppState {
        &mut self.state
    }

    /// Run a command as if chosen from a menu or the ribbon.
    pub fn perform(&mut self, action: AppAction, ctx: &egui::Context) {
        self.process_action(action, ctx);
    }

    /// Where the page was drawn last frame, in screen points.
    pub fn page_screen_rect(&self) -> Option<egui::Rect> {
        self.canvas.last_page_rect
    }

    /// Objects currently on the internal clipboard.
    pub fn clipboard_len(&self) -> usize {
        self.clipboard.objects.len()
    }

    /// Read-only access to settings.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// The error alert being shown, if any (title, detail).
    pub fn error_alert_message(&self) -> Option<&(String, String)> {
        self.error_alert.message()
    }

    /// Whether the about dialog is open.
    pub fn about_dialog_open(&self) -> bool {
        self.about_dialog.open
    }

    /// Whether the page properties dialog is open.
    pub fn page_properties_dialog_open(&self) -> bool {
        self.page_properties_dialog.open
    }

    /// Whether the text fit dialog is open.
    pub fn text_fit_dialog_open(&self) -> bool {
        self.text_fit_dialog.open
    }

    /// Whether the text columns dialog is open.
    pub fn text_columns_dialog_open(&self) -> bool {
        self.text_columns_dialog.open
    }

    /// Whether the text wrap dialog is open.
    pub fn text_wrap_dialog_open(&self) -> bool {
        self.text_wrap_dialog.open
    }

    /// Whether the insert table dialog is open.
    pub fn insert_table_dialog_open(&self) -> bool {
        self.insert_table_dialog.open
    }

    /// Whether the move page dialog is open.
    pub fn move_page_dialog_open(&self) -> bool {
        self.move_page_dialog.open
    }

    /// Whether the rename page dialog is open.
    pub fn rename_page_dialog_open(&self) -> bool {
        self.rename_page_dialog.open
    }

    /// Whether the text box margins dialog is open.
    pub fn text_box_margins_dialog_open(&self) -> bool {
        self.text_box_margins_dialog.open
    }

    /// Insert a hyperlink with the given display text and address (tests).
    pub fn insert_hyperlink_for_test(&mut self, text: &str, address: &str) -> bool {
        self.hyperlink_dialog.text = text.to_string();
        self.hyperlink_dialog.address = address.to_string();
        self.hyperlink_dialog.open_for_state(&self.state);
        self.hyperlink_dialog.text = text.to_string();
        self.hyperlink_dialog.address = address.to_string();
        self.hyperlink_dialog.insert(&mut self.state)
    }

    /// Whether the find/replace dialog is open.
    pub fn find_replace_dialog_open(&self) -> bool {
        self.find_replace_dialog.open
    }

    /// Whether the spelling dialog is open.
    pub fn spelling_dialog_open(&self) -> bool {
        self.spelling_dialog.open
    }

    /// Spell-check the document and return the words flagged (tests).
    pub fn spelling_check_for_test(&mut self) -> Vec<String> {
        self.spelling_dialog.open(&self.state);
        self.spelling_dialog
            .session
            .errors
            .iter()
            .map(|e| e.word.clone())
            .collect()
    }

    /// Apply the spelling dialog's suggested change (tests).
    pub fn spelling_change_for_test(&mut self, replacement: &str) -> bool {
        self.spelling_dialog
            .session
            .change_current(&mut self.state, replacement)
    }

    /// Whether the hyperlink dialog is open.
    pub fn hyperlink_dialog_open(&self) -> bool {
        self.hyperlink_dialog.open
    }

    /// Mutable access to the find/replace dialog (tests).
    pub fn find_replace_dialog_mut(&mut self) -> &mut crate::dialogs::FindReplaceDialog {
        &mut self.find_replace_dialog
    }

    /// Run the dialog's search against the document (tests).
    pub fn find_replace_search(&mut self) {
        self.find_replace_dialog.search(&self.state);
    }

    /// Select the dialog's current hit (tests).
    pub fn find_replace_select_current(&mut self, now: f64) {
        self.find_replace_dialog
            .select_current(&mut self.state, now);
    }

    /// Replace the dialog's current hit (tests).
    pub fn find_replace_replace_current(&mut self, now: f64) {
        self.find_replace_dialog
            .replace_current(&mut self.state, now);
    }

    /// Mutable access to the canvas (for tests).
    pub fn canvas_for_test(&mut self) -> &mut Canvas {
        &mut self.canvas
    }

    /// Process an action.
    fn process_action(&mut self, action: AppAction, ctx: &egui::Context) {
        // While editing canvas text, editing commands (often arriving from the native
        // menu bar, which takes Cmd+C/V/X/A/Z before egui sees them) act on the text.
        if self.state.text_edit.is_some() {
            if self.text_edit_action(&action, ctx) {
                return;
            }
            // Any other command ends text editing first
            let keeps_editing = matches!(
                action,
                AppAction::ZoomIn
                    | AppAction::ZoomOut
                    | AppAction::ZoomActual
                    | AppAction::ZoomFit
                    | AppAction::SaveDocument
                    | AppAction::SaveDocumentAs
                    | AppAction::TextBold
                    | AppAction::TextItalic
                    | AppAction::TextUnderline
                    | AppAction::TextSuperscript
                    | AppAction::TextSubscript
                    | AppAction::ToggleBullets
                    | AppAction::ToggleNumbering
                    | AppAction::GrowFont
                    | AppAction::ShrinkFont
                    | AppAction::SetFontSize(_)
                    | AppAction::SetFontFamily(_)
                    | AppAction::TextAlignLeft
                    | AppAction::TextAlignCenter
                    | AppAction::TextAlignRight
                    | AppAction::TextAlignJustify
                    | AppAction::SetFontColor(_)
                    | AppAction::ChangeIndent(_)
                    | AppAction::SetLineSpacing(_)
                    | AppAction::TableEdit(_)
                    | AppAction::CancelMode
                    | AppAction::TogglePagesPane
                    | AppAction::ToggleSelectionPane
                    | AppAction::SetTheme(_)
            );
            if !keeps_editing {
                crate::text_edit::end(&mut self.state);
            }
        }

        match action {
            AppAction::NewDocument => {
                if self.state.has_document() {
                    self.start_screen.open_backstage_new();
                } else {
                    self.start_screen.show_page(StartPage::New);
                }
            }
            AppAction::OpenDocument => {
                if self.confirm_discard(DiscardOp::Open(None)) {
                    self.open_document();
                }
            }
            AppAction::SaveDocument => {
                self.save_document(false);
            }
            AppAction::SaveDocumentAs => {
                self.save_document(true);
            }
            AppAction::Export => {
                self.export_dialog.open();
            }
            AppAction::Print => self.print(),
            AppAction::Quit => {
                if self.confirm_discard(DiscardOp::Quit) {
                    self.allow_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }

            AppAction::Undo => {
                self.state.undo();
            }
            AppAction::Redo => {
                self.state.redo();
            }
            AppAction::Delete => {
                self.delete_selected_objects();
            }
            AppAction::SelectAll => {
                self.select_all_objects();
            }
            AppAction::Cut => {
                self.copy_selected_objects();
                self.delete_selected_objects();
            }
            AppAction::Copy => {
                self.copy_selected_objects();
            }
            AppAction::Paste => {
                self.paste_objects(false);
            }
            AppAction::PasteInPlace => {
                self.paste_objects(true);
            }
            AppAction::Duplicate => {
                self.duplicate_selected_objects();
            }

            AppAction::ZoomIn => self.state.zoom_in(),
            AppAction::ZoomOut => self.state.zoom_out(),
            AppAction::ZoomActual => self.state.zoom_actual(),
            AppAction::ZoomFit => {
                // Canvas applies this next frame, once it knows its viewport size
                self.state.fit_page_pending = true;
            }

            AppAction::ToggleRulers => {
                self.state.show_rulers = !self.state.show_rulers;
            }
            AppAction::ToggleGuides => {
                self.state.show_guides = !self.state.show_guides;
            }
            AppAction::ToggleGrid => {
                self.state.show_grid = !self.state.show_grid;
            }
            AppAction::ToggleSnapToGrid => {
                self.state.snap_to_grid = !self.state.snap_to_grid;
            }
            AppAction::ToggleSnapToGuides => {
                self.state.snap_to_guides = !self.state.snap_to_guides;
            }
            AppAction::ClearGuides => {
                self.state.clear_guides();
            }

            AppAction::InsertText => {
                self.state.current_tool = Tool::Text;
            }
            AppAction::InsertImage => self.insert_picture(),
            AppAction::InsertRectangle => {
                self.state.current_tool = Tool::Rectangle;
            }
            AppAction::InsertEllipse => {
                self.state.current_tool = Tool::Ellipse;
            }
            AppAction::InsertLine => {
                self.state.current_tool = Tool::Line;
            }
            AppAction::InsertPolygon => {
                self.state.current_tool = Tool::Polygon;
            }
            AppAction::InsertStar => {
                self.state.current_tool = Tool::Star;
            }
            AppAction::InsertTable => {
                self.insert_table_dialog.open();
            }
            AppAction::EditShapePoints => {
                self.toggle_edit_shape_points();
            }
            AppAction::CheckSpelling => {
                self.spelling_dialog.open(&self.state);
            }
            AppAction::ChangePicture => {
                self.change_picture();
            }
            AppAction::InsertHyperlink => {
                self.hyperlink_dialog.open_for_state(&self.state);
            }
            AppAction::InsertPageNumber => {
                let now = ctx.input(|i| i.time);
                self.insert_field(repub_model::text::Field::page_number(), now);
            }
            AppAction::InsertField(field) => {
                let now = ctx.input(|i| i.time);
                self.insert_field(field, now);
            }
            AppAction::AddPage => {
                if self.state.has_document() {
                    self.state.record_undo("Insert Page");
                }
                if let Some(ref mut doc) = self.state.document {
                    doc.add_page();
                    self.state.mark_dirty();
                }
            }

            AppAction::InsertPageAt(index) => self.state.insert_page_at(index),
            AppAction::InsertSpread(index) => self.state.insert_spread_after(index),
            AppAction::DuplicatePage(index) => self.state.duplicate_pages(&[index]),
            AppAction::DeletePage(index) => self.state.delete_pages(&[index]),
            AppAction::Group => {
                let ids = self.state.selection.clone();
                if ids.len() >= 2 {
                    self.state.record_undo("Group");
                    if let Some(id) = self
                        .state
                        .current_page_mut()
                        .and_then(|p| p.group_shapes(&ids))
                    {
                        self.state.selection = vec![id];
                        self.state.mark_dirty();
                    }
                }
            }
            AppAction::Ungroup => {
                let groups: Vec<_> = self
                    .state
                    .selected_objects()
                    .iter()
                    .filter(|o| matches!(o.kind, ShapeKind::Group(_)))
                    .map(|o| o.id())
                    .collect();
                if !groups.is_empty() {
                    self.state.record_undo("Ungroup");
                    let mut members = Vec::new();
                    if let Some(page) = self.state.current_page_mut() {
                        for id in groups {
                            members.extend(page.ungroup(id));
                        }
                    }
                    self.state.selection = members;
                    self.state.mark_dirty();
                }
            }
            AppAction::BringToFront => {
                self.reorder_selected_objects(ObjectReorder::BringToFront);
            }
            AppAction::SendToBack => {
                self.reorder_selected_objects(ObjectReorder::SendToBack);
            }
            AppAction::BringForward => {
                self.reorder_selected_objects(ObjectReorder::BringForward);
            }
            AppAction::SendBackward => {
                self.reorder_selected_objects(ObjectReorder::SendBackward);
            }
            AppAction::AlignLeft
            | AppAction::AlignCenter
            | AppAction::AlignRight
            | AppAction::AlignTop
            | AppAction::AlignMiddle
            | AppAction::AlignBottom => {
                self.align_selected_objects(&action);
            }

            AppAction::TextAlignLeft => {
                text_format::set_alignment(&mut self.state, TextAlignment::Left);
            }
            AppAction::TextAlignCenter => {
                text_format::set_alignment(&mut self.state, TextAlignment::Center);
            }
            AppAction::TextAlignRight => {
                text_format::set_alignment(&mut self.state, TextAlignment::Right);
            }
            AppAction::TextAlignJustify => {
                text_format::set_alignment(&mut self.state, TextAlignment::Justify);
            }

            AppAction::ToggleTwoPageSpread => {
                self.state.two_page_spread = !self.state.two_page_spread;
                self.state.fit_page_pending = true;
            }

            AppAction::TextBold => {
                text_format::toggle(&mut self.state, Toggle::Bold);
            }
            AppAction::TextItalic => {
                text_format::toggle(&mut self.state, Toggle::Italic);
            }
            AppAction::TextUnderline => {
                text_format::toggle(&mut self.state, Toggle::Underline);
            }
            AppAction::TextSuperscript => {
                text_format::toggle(&mut self.state, Toggle::Superscript);
            }
            AppAction::TextSubscript => {
                text_format::toggle(&mut self.state, Toggle::Subscript);
            }
            AppAction::ToggleBullets => {
                text_format::toggle_list(&mut self.state, text_format::ListKind::Bullets);
            }
            AppAction::ToggleNumbering => {
                text_format::toggle_list(&mut self.state, text_format::ListKind::Numbering);
            }
            AppAction::GrowFont => {
                text_format::apply(&mut self.state, CharChange::StepSize(true));
            }
            AppAction::ShrinkFont => {
                text_format::apply(&mut self.state, CharChange::StepSize(false));
            }
            AppAction::SetFontSize(size) => {
                text_format::apply(&mut self.state, CharChange::Size(size));
            }
            AppAction::SetFontFamily(family) => {
                text_format::apply(&mut self.state, CharChange::Family(family));
            }
            AppAction::SetFontColor(color) => {
                text_format::apply(&mut self.state, CharChange::Color(color));
            }
            AppAction::ChangeIndent(increase) => {
                text_format::change_indent(&mut self.state, increase);
            }
            AppAction::SetLineSpacing(multiple) => {
                text_format::set_line_spacing(&mut self.state, multiple);
            }
            AppAction::InsertPicture => self.insert_picture(),
            AppAction::SetPageSize { width, height } => {
                self.page_setup("Page Size", |page| {
                    let landscape = page.size.width > page.size.height;
                    page.size = if landscape {
                        repub_geometry::Size::new(height, width)
                    } else {
                        repub_geometry::Size::new(width, height)
                    };
                });
                if let Some(doc) = self.state.document.as_mut() {
                    doc.default_page_size = repub_model::PageSize::Custom { width, height };
                }
                self.state.fit_page_pending = true;
            }
            AppAction::SetOrientation(landscape) => {
                self.page_setup("Page Orientation", |page| {
                    let (w, h) = (page.size.width, page.size.height);
                    if (w > h) != landscape {
                        page.size = repub_geometry::Size::new(h, w);
                    }
                });
                if let Some(doc) = self.state.document.as_mut() {
                    doc.default_orientation = if landscape {
                        repub_model::PageOrientation::Landscape
                    } else {
                        repub_model::PageOrientation::Portrait
                    };
                }
                self.state.fit_page_pending = true;
            }
            AppAction::SetMargins(margin) => {
                self.page_setup("Margins", |page| {
                    page.margins = repub_model::Margins {
                        top: margin,
                        right: margin,
                        bottom: margin,
                        left: margin,
                    };
                });
            }
            AppAction::SetPageBackground(color) => {
                if self.state.has_document() {
                    self.state.record_undo("Page Background");
                    if let Some(page) = self.state.current_page_mut() {
                        page.background = color.map(repub_model::styles::Fill::Solid);
                    }
                    self.state.mark_dirty();
                }
            }
            AppAction::SetShapeFill(color) => self.edit_selected("Shape Fill", |obj| {
                if has_fill(obj) {
                    obj.fill = color.map_or(
                        repub_model::styles::Fill::None,
                        repub_model::styles::Fill::Solid,
                    );
                }
            }),
            AppAction::SetShapeOutline(color) => self.edit_selected("Shape Outline", |obj| {
                if has_outline(obj) {
                    match (color, obj.line.as_mut()) {
                        (None, _) => obj.line = None,
                        (Some(c), Some(stroke)) => stroke.color = c,
                        (Some(c), None) => {
                            obj.line = Some(repub_model::styles::Stroke::solid(c, 1.0))
                        }
                    }
                }
            }),
            AppAction::CreateTextLink => {
                let source = match self.state.selected_objects()[..] {
                    [o] => match &o.kind {
                        ShapeKind::TextFrame(tf) if tf.linked_frame.is_none() => Some(o.common.id),
                        _ => None,
                    },
                    _ => None,
                };
                crate::text_edit::end(&mut self.state);
                self.state.link_source = source;
            }
            AppAction::BreakTextLink => {
                if let [id] = self.state.selection[..] {
                    self.state.begin_edit();
                    if let Some(page) = self.state.current_page_mut() {
                        crate::text_chain::break_link(page, id);
                    }
                    self.state.end_edit("Break Text Box Link");
                }
            }
            AppAction::Flip(horizontal) => {
                let label = if horizontal {
                    "Flip Horizontal"
                } else {
                    "Flip Vertical"
                };
                let locked: Vec<_> = self
                    .state
                    .selection
                    .iter()
                    .copied()
                    .filter(|id| self.state.is_locked(*id))
                    .collect();
                self.state.selection.retain(|id| !locked.contains(id));
                self.edit_selected(label, |obj| obj.flip(horizontal));
                self.state.selection.extend(locked);
            }
            AppAction::SetOutlineWeight(weight) => self.edit_selected("Outline Weight", |obj| {
                if has_outline(obj) {
                    match obj.line.as_mut() {
                        Some(stroke) => stroke.width = weight,
                        None => {
                            obj.line = Some(repub_model::styles::Stroke::solid(
                                repub_color::Color::BLACK,
                                weight,
                            ))
                        }
                    }
                }
            }),
            AppAction::RotateBy(degrees) => {
                crate::panels::toolbar::rotate_selection(&mut self.state, degrees)
            }
            AppAction::CancelMode => {
                self.state.link_source = None;
                self.state.deactivate_format_painter();
            }
            AppAction::TableEdit(op) => {
                let now = ctx.input(|i| i.time);
                self.edit_table(op, now);
            }
            AppAction::InsertTableRow | AppAction::InsertTableColumn => {
                let row = matches!(action, AppAction::InsertTableRow);
                self.edit_selected(if row { "Insert Row" } else { "Insert Column" }, |obj| {
                    if let ShapeKind::Table(table) = &mut obj.kind {
                        // The grid fills the bounds, so grow the bounds by the new track
                        if row {
                            let before = table.total_height().max(1.0);
                            table.add_row();
                            let scale = table.total_height() / before;
                            obj.common.bounds.size.height *= scale;
                        } else {
                            let before = table.total_width().max(1.0);
                            table.add_col();
                            let scale = table.total_width() / before;
                            obj.common.bounds.size.width *= scale;
                        }
                    }
                });
            }
            AppAction::EditText => {
                if let [id] = self.state.selection[..] {
                    let now = ctx.input(|i| i.time);
                    if crate::text_edit::is_text_frame(&self.state, id) {
                        crate::text_edit::begin(&mut self.state, id, None, now);
                        self.state.current_tool = Tool::Select;
                    } else if matches!(self.state.selected_objects()[..], [o] if matches!(o.kind, ShapeKind::Table(_)))
                    {
                        crate::text_edit::begin_cell(&mut self.state, id, 0, 0, None, now);
                        self.state.current_tool = Tool::Select;
                    }
                }
            }

            AppAction::ShowAbout => {
                self.about_dialog.open();
            }

            AppAction::ShowPageProperties => {
                self.page_properties_dialog.open_for_page(&self.state);
            }
            AppAction::ShowTextWrap => {
                if let Some(&obj_id) = self.state.selection.first() {
                    self.text_wrap_dialog.open_with_object(&self.state, obj_id);
                }
            }
            AppAction::ShowFontDialog => self.font_dialog.open_for(&self.state),
            AppAction::ShowParagraphDialog => self.paragraph_dialog.open_for(&self.state),
            AppAction::Distribute(horizontal) => self.distribute_selected_objects(horizontal),
            AppAction::SetAlignTarget(target) => self.state.align_target = target,
            AppAction::EditPoint(op) => {
                crate::edit_points::apply_op(&mut self.state, op);
            }
            AppAction::NewFromTemplate => self.templates_dialog.open(),
            AppAction::ShowWordArt => self.wordart_dialog.open_for_selection(&self.state),
            AppAction::InsertWordArt(preset) => self.insert_wordart(preset),
            AppAction::ShowTableProperties => {
                self.table_properties_dialog.open_for_selection(&self.state)
            }
            AppAction::ShowDocumentProperties => {
                self.document_properties_dialog.open_for(&self.state);
            }
            AppAction::ShowFormatShape => {
                self.format_shape_dialog.open_for_selection(&self.state);
            }
            AppAction::ShowTextFit => {
                if let Some(&obj_id) = self.state.selection.first() {
                    self.text_fit_dialog
                        .open_for_text_frame(&self.state, obj_id);
                }
            }
            AppAction::ShowTextColumns => {
                if let Some(&obj_id) = self.state.selection.first() {
                    self.text_columns_dialog
                        .open_for_text_frame(&self.state, obj_id);
                }
            }
            AppAction::ShowTextBoxMargins => {
                if let Some(&obj_id) = self.state.selection.first() {
                    self.text_box_margins_dialog
                        .open_for_text_frame(&self.state, obj_id);
                }
            }
            AppAction::MovePage => {
                if self.state.has_document() {
                    self.move_page_dialog
                        .open_for_page(&self.state, self.state.current_page);
                }
            }
            AppAction::RenamePage => {
                if self.state.has_document() {
                    self.rename_page_dialog
                        .open_for_page(&self.state, self.state.current_page);
                }
            }
            AppAction::FormatPainter => {
                if self.state.copy_format_from_selection() {
                    self.state.current_tool = Tool::Select;
                }
            }
            AppAction::Find => self.find_replace_dialog.open(),
            AppAction::Replace => self.find_replace_dialog.open_replace(),
            AppAction::ShowBackstage => {
                if self.state.has_document() {
                    self.open_backstage();
                }
            }
            AppAction::TogglePagesPane => {
                self.settings.left_panel_visible = !self.settings.left_panel_visible;
                self.settings.save();
            }
            AppAction::ToggleSelectionPane => {
                self.settings.selection_pane_visible = !self.settings.selection_pane_visible;
                self.settings.save();
            }
            AppAction::SetTheme(mode) => {
                self.settings.theme = mode;
                self.settings.save();
                crate::theme::apply_theme(ctx, mode);
            }

            AppAction::EditMasterPage(master_id) => {
                self.edit_master_page(master_id);
            }
            AppAction::ApplyMasterPage(master_id) => {
                self.apply_master_page_to_current(master_id);
            }
            AppAction::NewMasterPage => {
                self.create_and_edit_master_page();
            }
            AppAction::ExitMasterPageEdit => {
                self.exit_master_page_edit();
            }
        }
    }

    /// Move selected objects by the given delta.
    fn move_selected_objects(&mut self, delta_x: f64, delta_y: f64) {
        // Repeated arrow-key nudges collapse into one undo step
        self.state.record_undo_coalesced("Nudge");
        let selected_ids: Vec<_> = self
            .state
            .selection
            .iter()
            .copied()
            .filter(|id| !self.state.is_locked(*id))
            .collect();
        let current_page = self.state.current_page;

        {
            if let Some(page) = self.state.current_page_mut() {
                for layer in page.layers_mut() {
                    for obj in layer.shapes_mut() {
                        if selected_ids.contains(&obj.id()) {
                            let pos = obj.position();
                            obj.set_position(Point::new(pos.x + delta_x, pos.y + delta_y));
                        }
                    }
                }
            }
        }
        self.state.mark_dirty();
    }

    /// Delete selected objects.
    fn delete_selected_objects(&mut self) {
        if self.state.selection.is_empty() {
            return;
        }

        self.state.record_undo("Delete");
        // Locked objects stay
        let selected_ids: Vec<_> = self
            .state
            .selection
            .iter()
            .copied()
            .filter(|id| !self.state.is_locked(*id))
            .collect();
        let current_page = self.state.current_page;

        {
            if let Some(page) = self.state.current_page_mut() {
                // Linked text boxes keep their story
                crate::text_chain::before_delete(page, &selected_ids);
                for id in &selected_ids {
                    page.remove_shape(*id);
                }
            }
        }

        self.state.selection.clear();
        self.state.mark_dirty();
    }

    /// Select all objects on the current page.
    fn select_all_objects(&mut self) {
        let ids: Vec<_> = self
            .state
            .current_page()
            .map(|page| {
                page.layers()
                    .iter()
                    .filter(|l| l.visible && !l.locked)
                    .flat_map(|l| {
                        l.shapes()
                            .iter()
                            .filter(|o| o.common.visible)
                            .map(|o| o.id())
                    })
                    .collect()
            })
            .unwrap_or_default();
        self.state.selection = ids;
    }

    /// Copy selected objects to the clipboard.
    fn copy_selected_objects(&mut self) {
        self.clipboard.objects.clear();
        self.clipboard.original_positions.clear();

        // Selected objects (group members too) in stacking order
        for obj in self.state.selected_objects() {
            self.clipboard.objects.push(obj.clone());
            self.clipboard.original_positions.push(obj.position());
        }
    }

    /// Paste objects from clipboard.
    /// If `in_place` is true, paste at original positions.
    /// Otherwise, offset by a small amount.
    fn paste_objects(&mut self, in_place: bool) {
        if self.clipboard.objects.is_empty() {
            return;
        }

        self.state.record_undo("Paste");
        let current_page = self.state.current_page;
        let offset = if in_place { 0.0 } else { 20.0 }; // 20pt offset for regular paste

        // Clear selection and prepare new IDs
        self.state.selection.clear();
        let mut new_ids = Vec::new();

        {
            if let Some(page) = self.state.current_page_mut() {
                for (i, obj) in self.clipboard.objects.iter().enumerate() {
                    // Clone the object and assign a new ID
                    let mut new_obj = obj.clone();
                    let new_id = ObjectId::new();
                    new_obj.common.id = new_id;
                    // A copy isn't part of the original's chain of linked boxes
                    if let ShapeKind::TextFrame(tf) = &mut new_obj.kind {
                        tf.linked_frame = None;
                        tf.is_continuation = false;
                        tf.overflow = Default::default();
                    }

                    // Apply offset for regular paste
                    if !in_place {
                        let pos = new_obj.position();
                        new_obj.set_position(Point::new(pos.x + offset, pos.y + offset));
                    }

                    page.add_shape(new_obj);
                    new_ids.push(new_id);
                }
            }
        }

        // Select the newly pasted objects
        self.state.selection = new_ids;
        self.state.mark_dirty();
    }

    /// Duplicate selected objects in place with a small offset.
    fn duplicate_selected_objects(&mut self) {
        // Copy then paste with offset
        self.copy_selected_objects();
        self.paste_objects(false);
    }

    /// Reorder selected objects in z-order.
    fn reorder_selected_objects(&mut self, order: ObjectReorder) {
        if self.state.selection.is_empty() {
            return;
        }

        // Recorded only if something moves (e.g. not for an object already in front)
        self.state.begin_edit();
        let selected_ids = self.state.selection.clone();
        let current_page = self.state.current_page;

        {
            if let Some(page) = self.state.current_page_mut() {
                for layer in page.layers_mut() {
                    for id in &selected_ids {
                        match order {
                            ObjectReorder::BringToFront => {
                                layer.bring_to_front(*id);
                            }
                            ObjectReorder::SendToBack => {
                                layer.send_to_back(*id);
                            }
                            ObjectReorder::BringForward => {
                                layer.bring_forward(*id);
                            }
                            ObjectReorder::SendBackward => {
                                layer.send_backward(*id);
                            }
                        }
                    }
                }
            }
        }

        self.state.end_edit("Arrange");
    }

    /// Align the selection. Several objects align to each other; a single object
    /// aligns to the page, as in Publisher.
    fn align_selected_objects(&mut self, action: &AppAction) {
        let objects = self.state.selected_objects();
        if objects.is_empty() {
            return;
        }
        let page_rect = self
            .state
            .current_page()
            .map(|p| repub_geometry::Rect::new(0.0, 0.0, p.size.width, p.size.height));
        let margins_rect = self.state.current_page().map(|p| p.content_rect());
        let target = match self.state.align_target {
            crate::state::AlignTarget::Page => page_rect,
            crate::state::AlignTarget::Margins => margins_rect,
            crate::state::AlignTarget::Auto if objects.len() == 1 => page_rect,
            crate::state::AlignTarget::Auto => objects
                .iter()
                .map(|o| o.bounds())
                .reduce(|a, b| a.union(&b)),
        };
        let Some(target) = target else { return };
        // Recorded only if something moves (e.g. not for an object already in front)
        self.state.begin_edit();
        let ids = self.state.selection.clone();
        if let Some(page) = self.state.current_page_mut() {
            for id in ids {
                let Some(obj) = page.get_shape_mut(id) else {
                    continue;
                };
                let b = obj.bounds();
                let (mut x, mut y) = (b.origin.x, b.origin.y);
                match action {
                    AppAction::AlignLeft => x = target.origin.x,
                    AppAction::AlignCenter => {
                        x = target.origin.x + (target.size.width - b.size.width) / 2.0
                    }
                    AppAction::AlignRight => x = target.origin.x + target.size.width - b.size.width,
                    AppAction::AlignTop => y = target.origin.y,
                    AppAction::AlignMiddle => {
                        y = target.origin.y + (target.size.height - b.size.height) / 2.0
                    }
                    AppAction::AlignBottom => {
                        y = target.origin.y + target.size.height - b.size.height
                    }
                    _ => {}
                }
                obj.set_position(Point::new(x, y));
            }
        }
        self.state.end_edit("Align");
    }

    /// Space three or more selected objects so the gaps between them are equal, keeping the
    /// outermost two where they are.
    fn distribute_selected_objects(&mut self, horizontal: bool) {
        let mut items: Vec<(repub_model::ObjectId, repub_geometry::Rect)> = self
            .state
            .selected_objects()
            .iter()
            .map(|o| (o.id(), o.bounds()))
            .collect();
        if items.len() < 3 {
            return;
        }
        let (start, size): (
            fn(&repub_geometry::Rect) -> f64,
            fn(&repub_geometry::Rect) -> f64,
        ) = if horizontal {
            (|r| r.origin.x, |r| r.size.width)
        } else {
            (|r| r.origin.y, |r| r.size.height)
        };
        items.sort_by(|a, b| start(&a.1).total_cmp(&start(&b.1)));
        let first = start(&items[0].1);
        let last = items
            .iter()
            .map(|(_, r)| start(r) + size(r))
            .fold(f64::MIN, f64::max);
        let total: f64 = items.iter().map(|(_, r)| size(r)).sum();
        let gap = (last - first - total) / (items.len() - 1) as f64;
        self.state.begin_edit();
        let mut at = first;
        if let Some(page) = self.state.current_page_mut() {
            for (id, rect) in &items {
                if let Some(obj) = page.get_shape_mut(*id) {
                    let pos = if horizontal {
                        Point::new(at, rect.origin.y)
                    } else {
                        Point::new(rect.origin.x, at)
                    };
                    obj.set_position(pos);
                }
                at += size(rect) + gap;
            }
        }
        self.state.end_edit(if horizontal {
            "Distribute Horizontally"
        } else {
            "Distribute Vertically"
        });
    }

    /// Add WordArt in the middle of the page (inside the margins), select it and open it for editing.
    fn insert_wordart(&mut self, preset: repub_model::shape::WordArtPreset) {
        use repub_model::shape::{TextEffectData, TextTransform};
        let Some(page) = self.state.current_page() else {
            return;
        };
        let area = page.content_rect();
        let (width, height) = (area.width().min(288.0), area.height().min(96.0));
        let bounds = repub_geometry::Rect::new(
            area.x() + (area.width() - width) / 2.0,
            area.y() + (area.height() - height) / 2.0,
            width,
            height,
        );
        let mut data = TextEffectData::new("Your Text Here");
        data.style.font.weight = repub_model::styles::FontWeight::Bold;
        data.style.font_size = 48.0;
        if preset != repub_model::shape::WordArtPreset::Plain {
            data.transform = TextTransform::Preset(preset);
        }
        let mut shape = Shape::new(bounds, ShapeKind::TextEffect(data));
        shape.fill =
            repub_model::styles::Fill::Solid(repub_color::Color::from_rgb8(0x44, 0x72, 0xC4));
        shape.line = Some(repub_model::styles::Stroke::new(
            repub_color::Color::from_rgb8(0x2F, 0x52, 0x96),
            1.0,
        ));
        self.state.record_undo("Insert WordArt");
        let Some(page) = self.state.current_page_mut() else {
            return;
        };
        let id = page.add_shape(shape);
        self.state.selection = vec![id];
        self.state.mark_dirty();
        self.wordart_dialog.open_for_selection(&self.state);
    }

    /// Insert a field at the caret, else at the end of the selected text frame, else in a
    /// small text frame near the bottom of the page.
    fn insert_field(&mut self, field: repub_model::text::Field, now: f64) {
        if !self.state.has_document() {
            return;
        }

        let selected_text = self.state.selection.iter().copied().find(|id| {
            self.state
                .current_page()
                .and_then(|p| p.get_shape(*id))
                .is_some_and(|o| matches!(o.kind, ShapeKind::TextFrame(_)))
        });

        if self.state.text_edit.is_some() {
            crate::text_edit::insert_field(&mut self.state, field, now);
            return;
        }

        self.state.record_undo("Insert Field");

        if let Some(id) = selected_text {
            if let Some(page) = self.state.current_page_mut() {
                if let Some(Shape {
                    kind: ShapeKind::TextFrame(tf),
                    ..
                }) = page.get_shape_mut(id)
                {
                    if tf.content.paragraphs.is_empty() {
                        tf.content
                            .paragraphs
                            .push(repub_model::text::Paragraph::new());
                    }
                    if let Some(para) = tf.content.paragraphs.last_mut() {
                        para.add_field(field.clone());
                    }
                }
            }
        } else {
            // Place a centred page-number frame near the bottom of the page.
            let size = self.state.current_page().map(|p| p.size);
            let Some(size) = size else { return };
            let width = 120.0;
            let height = 24.0;
            let x = (size.width - width) / 2.0;
            let y = size.height - height - 36.0;
            let bounds = repub_geometry::Rect::new(x, y, width, height);
            let mut shape = Shape::text_frame(bounds);
            let ShapeKind::TextFrame(tf) = &mut shape.kind else {
                unreachable!()
            };
            let mut para = repub_model::text::Paragraph::new();
            para.style.alignment = repub_model::styles::TextAlignment::Center;
            para.add_field(field.clone());
            tf.content.paragraphs.push(para);
            let id = shape.common.id;
            if let Some(page) = self.state.current_page_mut() {
                page.add_shape(shape);
            }
            self.state.selection = vec![id];
        }

        self.state.mark_dirty();
    }

    /// Edit a master page in place of the current page. With no id: the
    /// current page's master; else the first master; else a new "Master Page A"
    /// that every page without a master starts using (as in Publisher, where
    /// every page has one).
    fn edit_master_page(&mut self, master_id: Option<repub_model::MasterPageId>) {
        if !self.state.has_document() {
            return;
        }
        crate::text_edit::end(&mut self.state);
        let current = self
            .state
            .document
            .as_ref()
            .and_then(|d| d.page(self.state.current_page))
            .and_then(|p| p.master_page);
        let first = self
            .state
            .document
            .as_ref()
            .and_then(|d| d.master_pages().first().map(|m| m.id));
        let id = match master_id.or(current).or(first) {
            Some(id) => id,
            None => {
                self.state.record_undo("New Master Page");
                let Some(doc) = self.state.document.as_mut() else {
                    return;
                };
                let id = doc.add_master_page("Master Page A");
                for page in doc.pages_mut() {
                    if page.master_page.is_none() {
                        page.master_page = Some(id);
                    }
                }
                self.state.mark_dirty();
                id
            }
        };
        self.state.begin_master_edit(id);
    }

    /// Apply a master page to the current page (or clear it with None).
    fn apply_master_page_to_current(&mut self, master_id: Option<repub_model::MasterPageId>) {
        if !self.state.has_document() {
            return;
        }
        self.state.record_undo("Apply Master Page");
        let page_index = self.state.current_page;
        if let Some(doc) = self.state.document.as_mut() {
            match master_id {
                Some(id) => {
                    doc.apply_master_page(page_index, id);
                }
                None => {
                    if let Some(page) = doc.page_mut(page_index) {
                        page.master_page = None;
                    }
                }
            }
        }
        self.state.mark_dirty();
    }

    /// Create a new master page and edit it.
    fn create_and_edit_master_page(&mut self) {
        if !self.state.has_document() {
            return;
        }
        crate::text_edit::end(&mut self.state);
        self.state.record_undo("New Master Page");
        let name = {
            let count = self
                .state
                .document
                .as_ref()
                .map(|d| d.master_pages().len())
                .unwrap_or(0);
            format!("Master Page {}", (b'A' + (count % 26) as u8) as char)
        };
        let new_id = match self.state.document.as_mut() {
            Some(doc) => doc.add_master_page(name),
            None => return,
        };
        self.state.mark_dirty();
        self.state.begin_master_edit(new_id);
    }

    /// Exit master page editing mode.
    fn exit_master_page_edit(&mut self) {
        crate::text_edit::end(&mut self.state);
        self.state.end_master_edit();
    }

    /// Route an editing command to the canvas text being edited.
    /// Returns true if the command was handled as a text command.
    fn text_edit_action(&mut self, action: &AppAction, ctx: &egui::Context) -> bool {
        use crate::text_edit;
        let now = ctx.input(|i| i.time);
        match action {
            AppAction::Copy | AppAction::Cut => {
                let selected = text_edit::selected_text(&self.state);
                if !selected.is_empty() {
                    ctx.copy_text(selected.clone());
                    self.state.text_clipboard = selected;
                    if matches!(action, AppAction::Cut) {
                        text_edit::delete_forward(&mut self.state, false, now);
                    }
                }
            }
            AppAction::Paste | AppAction::PasteInPlace => {
                let text = self
                    .system_clipboard_text()
                    .unwrap_or_else(|| self.state.text_clipboard.clone());
                text_edit::type_text(&mut self.state, &text, now);
            }
            AppAction::SelectAll => text_edit::select_all(&mut self.state, now),
            AppAction::Delete => text_edit::delete_forward(&mut self.state, false, now),
            AppAction::Undo => text_edit::undo(&mut self.state, false, now),
            AppAction::Redo => text_edit::undo(&mut self.state, true, now),
            _ => return false,
        }
        true
    }

    /// Text on the system clipboard (only in the real app; tests use the in-app copy).
    fn system_clipboard_text(&self) -> Option<String> {
        #[cfg(not(target_arch = "wasm32"))]
        if self.native_menu.is_some() {
            return arboard::Clipboard::new().ok()?.get_text().ok();
        }
        None
    }

    /// Replace the current document with a new blank one.
    fn create_document(
        &mut self,
        page_size: repub_model::PageSize,
        orientation: repub_model::PageOrientation,
        pages: u32,
    ) {
        let mut doc = Document::new(page_size, orientation);
        for _ in 1..pages {
            doc.add_page();
        }
        self.install_new_document(doc);
    }

    /// Replace the current document with the unsaved `doc`.
    fn install_new_document(&mut self, mut doc: Document) {
        doc.metadata.created = unix_now();
        doc.metadata.modified = doc.metadata.created;
        self.state.text_edit = None;
        self.state.document = Some(doc);
        self.state.fit_page_pending = true;
        self.state.document_path = None;
        self.state.dirty = false;
        self.state.current_page = 0;
        self.state.selection.clear();
        self.state.history.clear();
    }

    /// Open Backstage (File tab). Call from the ribbon's File tab.
    pub fn open_backstage(&mut self) {
        self.start_screen.open_backstage();
    }

    fn backstage_document_info(&self) -> DocumentInfo {
        let name = self
            .state
            .document_path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".to_owned());
        let page_size = self
            .state
            .document
            .as_ref()
            .and_then(|d| d.pages().first().map(|p| (p.size.width, p.size.height)))
            .unwrap_or((0.0, 0.0));
        DocumentInfo {
            name,
            path: self.state.document_path.clone(),
            pages: self.state.page_count(),
            page_size,
            dirty: self.state.dirty,
        }
    }

    fn handle_start_screen_action(&mut self, action: StartScreenAction, ctx: &egui::Context) {
        self.start_screen.close_backstage();
        match action {
            StartScreenAction::CreateBlank {
                page_size,
                orientation,
            } => {
                if self.confirm_discard(DiscardOp::New {
                    page_size,
                    orientation,
                    pages: 1,
                }) {
                    self.create_document(page_size, orientation, 1);
                }
            }
            StartScreenAction::MoreSizes => self.new_doc_dialog.open(),
            StartScreenAction::OpenRecent(path) => {
                if self.confirm_discard(DiscardOp::Open(Some(path.clone()))) {
                    self.open_path(path);
                }
            }
            StartScreenAction::Command(action) => self.process_action(action, ctx),
            StartScreenAction::CloseDocument => {
                if self.confirm_discard(DiscardOp::Close) {
                    self.close_document();
                }
            }
        }
    }

    /// Apply `f` to every page (page setup is publication-wide, as in Publisher), as one undo step.
    fn page_setup(&mut self, label: &str, f: impl Fn(&mut repub_model::Page)) {
        if !self.state.has_document() {
            return;
        }
        // Undo step only if a page actually changes
        self.state.begin_edit();
        if let Some(doc) = self.state.document.as_mut() {
            for page in doc.pages_mut() {
                f(page);
            }
        }
        self.state.end_edit(label);
    }

    /// Apply `f` to each selected object on the current page, as one undo step.
    fn edit_selected(&mut self, label: &str, f: impl Fn(&mut Shape)) {
        if self.state.selection.is_empty() {
            return;
        }
        // Undo step only if an object actually changes
        self.state.begin_edit();
        let ids = self.state.selection.clone();
        if let Some(page) = self.state.current_page_mut() {
            for id in ids {
                if let Some(obj) = page.get_shape_mut(id) {
                    f(obj);
                }
            }
        }
        self.state.end_edit(label);
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Pick a picture file and place it on the current page: at its natural size
    /// (96 dpi), shrunk to fit inside the margins, centred, and selected.
    fn insert_picture(&mut self) {
        if !self.state.has_document() {
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Pictures",
                &["png", "jpg", "jpeg", "gif", "bmp", "webp", "tif", "tiff"],
            )
            .add_filter("All Files", &["*"])
            .pick_file()
        else {
            return;
        };
        if let Err(e) = self.place_picture(&path) {
            self.error_alert.open(
                "Couldn't Insert Picture",
                format!("Repub couldn't read \"{}\".\n\n{e}", path.display()),
            );
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn place_picture(&mut self, path: &Path) -> Result<(), String> {
        let (px_w, px_h) = image::image_dimensions(path).map_err(|e| e.to_string())?;
        let data = repub_model::shape::PictureData::from_file(path.to_string_lossy().into_owned());
        self.place_picture_data(data, px_w, px_h)
    }

    /// Place a picture of `px_w` x `px_h` pixels on the current page at its natural size (96 dpi),
    /// shrunk to fit inside the margins and centred, selected, as one undo step.
    fn place_picture_data(
        &mut self,
        data: repub_model::shape::PictureData,
        px_w: u32,
        px_h: u32,
    ) -> Result<(), String> {
        let Some(page) = self.state.current_page() else {
            return Err("No page".into());
        };
        let area = page.content_rect();
        let (mut w, mut h) = (px_w as f64 * 0.75, px_h as f64 * 0.75);
        let fit = (area.size.width / w).min(area.size.height / h).min(1.0);
        w *= fit;
        h *= fit;
        let bounds = repub_geometry::Rect::new(
            area.origin.x + (area.size.width - w) / 2.0,
            area.origin.y + (area.size.height - h) / 2.0,
            w,
            h,
        );
        let mut frame = Shape::picture(bounds);
        frame.kind = ShapeKind::Picture(data);
        let id = frame.common.id;
        self.state.record_undo("Insert Picture");
        if let Some(page) = self.state.current_page_mut() {
            page.add_shape(frame);
        }
        self.state.select(id, false);
        self.state.current_tool = Tool::Select;
        self.state.mark_dirty();
        Ok(())
    }

    /// Insert a `rows` x `cols` table in the middle of the page's margins, as
    /// Publisher does: a quarter-inch-tall row and up to an inch per column,
    /// never wider than the margins. Selected, one undo step.
    pub fn insert_table(&mut self, rows: u32, cols: u32) {
        let (rows, cols) = (rows.max(1), cols.max(1));
        let Some(area) = self.state.current_page().map(|p| p.content_rect()) else {
            return;
        };
        let width = (cols as f64 * 72.0).min(area.size.width);
        let height = (rows as f64 * 24.0).min(area.size.height);
        let bounds = repub_geometry::Rect::new(
            area.origin.x + (area.size.width - width) / 2.0,
            area.origin.y + (area.size.height - height) / 2.0,
            width,
            height,
        );
        let table = Shape::table(bounds, rows, cols);
        let id = table.common.id;
        self.state.record_undo("Insert Table");
        if let Some(page) = self.state.current_page_mut() {
            page.add_shape(table);
        }
        self.state.select(id, false);
        self.state.current_tool = Tool::Select;
        self.state.mark_dirty();
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Print by way of PDF: write the publication to a temporary PDF and open it
    /// in the system's viewer (Preview on macOS), whose Print command handles
    /// printers, copies and page ranges.
    fn print(&mut self) {
        let Some(doc) = self.state.document.as_ref() else {
            return;
        };
        let stem = self
            .state
            .document_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_owned());
        let path = std::env::temp_dir().join(format!("{stem} (print).pdf"));
        let fonts = match repub_fonts::SystemFontProvider::new() {
            Ok(fonts) => fonts,
            Err(e) => {
                self.error_alert.open(
                    "Couldn't Print",
                    format!("Repub couldn't prepare the publication for printing.\n\n{e}"),
                );
                return;
            }
        };
        if let Err(e) =
            repub_format::export_pdf(doc, &path, &repub_format::PdfOptions::default(), &fonts)
        {
            self.error_alert.open(
                "Couldn't Print",
                format!("Repub couldn't prepare the publication for printing.\n\n{e}"),
            );
            return;
        }
        if let Err(e) = open_in_system_viewer(&path) {
            self.error_alert.open(
                "Couldn't Print",
                format!(
                    "The PDF for printing is at \"{}\", but it couldn't be opened.\n\n{e}",
                    path.display()
                ),
            );
        }
    }

    /// Run a table row/column command (see [`TableEdit`]) as one undo step;
    /// editing carries on in the same cell (or its nearest neighbour).
    fn edit_table(&mut self, op: TableEdit, now: f64) {
        let Some(table_shape) = self.state.selected_objects().first().copied() else {
            return;
        };
        let ShapeKind::Table(table) = &table_shape.kind else {
            return;
        };
        if !op.enabled(&self.state) {
            return;
        }
        let (id, rows, cols) = (
            table_shape.common.id,
            table.rows as usize,
            table.cols as usize,
        );
        let editing = self
            .state
            .text_edit
            .clone()
            .filter(|e| e.frame == id && e.cell.is_some());
        let (row, col) = editing.as_ref().and_then(|e| e.cell).unwrap_or(match op {
            TableEdit::RowAbove | TableEdit::ColumnLeft => (0, 0),
            _ => (rows - 1, cols - 1),
        });
        // Removing the only row or column removes the table, as in Publisher
        let whole = op == TableEdit::DeleteTable
            || (op == TableEdit::DeleteRow && rows <= 1)
            || (op == TableEdit::DeleteColumn && cols <= 1);
        crate::text_edit::end(&mut self.state);
        if whole {
            self.state.selection = vec![id];
            self.delete_selected_objects();
            return;
        }
        self.state.record_undo(op.label());
        let mut cell = (row, col);
        if let Some(Shape {
            kind: ShapeKind::Table(t),
            ..
        }) = self
            .state
            .current_page_mut()
            .and_then(|p| p.get_shape_mut(id))
        {
            match op {
                TableEdit::RowAbove => {
                    t.insert_row_at(row as u32);
                    cell.0 += 1;
                }
                TableEdit::RowBelow => t.insert_row_at(row as u32 + 1),
                TableEdit::ColumnLeft => {
                    t.insert_col_at(col as u32);
                    cell.1 += 1;
                }
                TableEdit::ColumnRight => t.insert_col_at(col as u32 + 1),
                TableEdit::DeleteRow => {
                    t.remove_row(row as u32);
                    cell.0 = row.min(t.rows as usize - 1);
                }
                TableEdit::DeleteColumn => {
                    t.remove_col(col as u32);
                    cell.1 = col.min(t.cols as usize - 1);
                }
                TableEdit::DeleteTable => {}
            }
        }
        self.state.mark_dirty();
        if let Some(edit) = editing {
            let caret = (cell == (row, col)
                || matches!(op, TableEdit::RowAbove | TableEdit::ColumnLeft))
            .then_some(edit.caret);
            crate::text_edit::begin_cell(&mut self.state, id, cell.0, cell.1, caret, now);
        }
    }

    /// Follow a hyperlink: open its address, or go to its page.
    fn follow_link(&mut self, link: &repub_model::text::Hyperlink) {
        use crate::hyperlinks::{destination, open_address, Destination};
        match destination(link) {
            Destination::Address(address) => {
                if let Err(e) = open_address(&address) {
                    self.error_alert.open(
                        "Couldn't Open Link",
                        format!("\"{address}\" couldn't be opened.\n\n{e}"),
                    );
                }
            }
            Destination::Page(index) if index < self.state.page_count() => {
                self.state.set_page(index)
            }
            Destination::Page(index) => {
                self.error_alert.open(
                    "Couldn't Follow Link",
                    format!(
                        "The link goes to page {}, which isn't in this publication.",
                        index + 1
                    ),
                );
            }
            Destination::Unusable(target) => {
                self.error_alert.open(
                    "Couldn't Open Link",
                    format!("\"{target}\" isn't a web or email address Repub can open."),
                );
            }
        }
    }

    /// Close the current publication without asking.
    fn close_document(&mut self) {
        self.state.text_edit = None;
        self.state.document = None;
        self.state.document_path = None;
        self.state.dirty = false;
        self.state.current_page = 0;
        self.state.selection.clear();
        self.state.history.clear();
    }

    /// Whether `op` may discard the open document now. With unsaved changes this
    /// asks first (Save / Don't Save / Cancel), holds `op`, and returns false;
    /// [`Self::answer_save_prompt`] runs it once the user agrees.
    fn confirm_discard(&mut self, op: DiscardOp) -> bool {
        if !self.state.has_document() || !self.state.dirty {
            return true;
        }
        self.pending_discard = Some(op);
        let name = self.state.window_title();
        let name = name
            .trim_start_matches("* ")
            .trim_end_matches(" - Repub")
            .to_owned();
        self.save_prompt.open(name);
        false
    }

    /// Act on the save prompt's answer, finishing or dropping the held operation.
    fn answer_save_prompt(&mut self, choice: SaveChoice, ctx: &egui::Context) {
        let Some(op) = self.pending_discard.take() else {
            return;
        };
        match choice {
            SaveChoice::Cancel => return,
            SaveChoice::Save => {
                self.save_document(false);
                // Save dialog cancelled or the save failed: keep the document
                if self.state.dirty {
                    return;
                }
            }
            SaveChoice::DontSave => {}
        }
        match op {
            DiscardOp::Close => self.close_document(),
            DiscardOp::Quit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            DiscardOp::Open(Some(path)) => self.open_path(path),
            DiscardOp::Open(None) => self.open_document(),
            DiscardOp::New {
                page_size,
                orientation,
                pages,
            } => self.create_document(page_size, orientation, pages),
            DiscardOp::Template(doc) => self.install_new_document(*doc),
        }
    }

    /// Whether the save-changes prompt is showing (for tests).
    pub fn save_prompt_open(&self) -> bool {
        self.save_prompt.is_open()
    }

    /// Answer the save-changes prompt as if a button was clicked (for tests).
    pub fn answer_save_prompt_for_test(&mut self, choice: SaveChoice, ctx: &egui::Context) {
        self.save_prompt.close();
        self.answer_save_prompt(choice, ctx);
    }

    /// Title of the error message box, if one is showing (for tests).
    pub fn error_alert_title(&self) -> Option<&str> {
        self.error_alert.title()
    }

    /// Toggle point-editing for the single selected shape.
    fn toggle_edit_shape_points(&mut self) {
        let Some(&id) = self.state.selection.first() else {
            return;
        };
        if self.state.selection.len() != 1 {
            return;
        }
        let is_shape = self
            .state
            .current_page()
            .and_then(|p| p.get_shape(id))
            .is_some_and(|o| matches!(o.kind, ShapeKind::AutoShape(_)));
        if !is_shape {
            return;
        }
        if self
            .state
            .edit_points
            .as_ref()
            .is_some_and(|s| s.shape_id == id)
        {
            self.state.edit_points = None;
        } else {
            self.state.text_edit = None;
            self.state.edit_points = Some(crate::state::EditPointsState {
                shape_id: id,
                selected: None,
                dragging: false,
                dragging_control: None,
            });
            self.state.current_tool = Tool::Select;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    /// Replace the selected image frame's picture with a file chosen by the user.
    fn change_picture(&mut self) {
        use repub_model::shape::ImageSource;
        let Some(id) = self.state.selection.iter().copied().find(|id| {
            self.state
                .current_page()
                .and_then(|p| p.get_shape(*id))
                .is_some_and(|o| matches!(o.kind, ShapeKind::Picture(_)))
        }) else {
            return;
        };
        let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Images",
                &["png", "jpg", "jpeg", "gif", "bmp", "tiff", "webp"],
            )
            .pick_file()
        else {
            return;
        };
        self.state.record_undo("Change Picture");
        if let Some(page) = self.state.current_page_mut() {
            if let Some(Shape {
                kind: ShapeKind::Picture(img),
                ..
            }) = page.get_shape_mut(id)
            {
                img.source = ImageSource::File(path.to_string_lossy().into_owned());
            }
        }
        self.state.mark_dirty();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open_document(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("All Publications", &["repub", "repubz", "pub"])
            .add_filter("Repub Documents", &["repub", "repubz"])
            .add_filter("Microsoft Publisher", &["pub"])
            .add_filter("All Files", &["*"])
            .pick_file()
        {
            self.open_path(path);
        }
    }

    /// Make `doc`, opened from `path`, the current document.
    fn install_opened_document(&mut self, doc: Document, path: Option<PathBuf>) {
        self.state.text_edit = None;
        self.state.document = Some(doc);
        self.state.fit_page_pending = true;
        self.state.document_path = path;
        self.state.dirty = false;
        self.state.current_page = 0;
        self.state.selection.clear();
        self.state.history.clear();
        self.state.apply_document_settings();
    }

    /// Open a document from a path (file dialog or command-line argument).
    pub fn open_path(&mut self, path: PathBuf) {
        // Try to detect file format from magic bytes
        // TODO(port): the facade's `load` dispatches by file extension, so the
        // OLE magic-byte detection no longer changes behaviour; a Publisher
        // file must carry a `.pub` extension to import. Both branches route
        // through the facade's `load`.
        let result = if Self::is_ole_file(&path) {
            // Microsoft Publisher format (OLE Compound Document)
            log::info!("Detected MS Publisher format, importing...");
            repub_format::load(&path)
        } else {
            // Native JSON format
            repub_format::load(&path)
        };

        match result {
            Ok(doc) => {
                self.install_opened_document(doc, Some(path.clone()));
                self.settings.add_recent_file(path);
                self.settings.save();
            }
            Err(e) => {
                log::error!("Failed to open document {}: {}", path.display(), e);
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default();
                self.error_alert.open(
                    "Couldn't Open File",
                    format!("Repub couldn't open \"{name}\".\n\n{e}"),
                );
            }
        }
    }

    /// Check if a file is an OLE Compound Document (MS Publisher format).
    fn is_ole_file(path: &Path) -> bool {
        // OLE magic bytes: D0 CF 11 E0 A1 B1 1A E1
        const OLE_MAGIC: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];

        if let Ok(mut file) = std::fs::File::open(path) {
            use std::io::Read;
            let mut magic = [0u8; 8];
            if file.read_exact(&mut magic).is_ok() {
                return magic == OLE_MAGIC;
            }
        }
        false
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn save_document(&mut self, save_as: bool) {
        // Save writes Repub's own format, so a document opened from another
        // format (e.g. a Publisher .pub) is never overwritten in place: Save asks
        // for a new name, as Save As does.
        let native_path = self
            .state
            .document_path
            .clone()
            .filter(|p| is_native_document(p));
        let path = if save_as || native_path.is_none() {
            let stem = self
                .state
                .document_path
                .as_ref()
                .and_then(|p| p.file_stem())
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Untitled".to_owned());
            let mut dialog = rfd::FileDialog::new()
                .add_filter("Repub Document", &["repub"])
                .add_filter("Compressed Repub", &["repubz"])
                .set_file_name(format!("{stem}.repub"));
            if let Some(dir) = self.state.document_path.as_ref().and_then(|p| p.parent()) {
                dialog = dialog.set_directory(dir);
            }
            dialog.save_file()
        } else {
            native_path
        };

        if let Some(path) = path {
            if let Some(doc) = self.state.document.as_mut() {
                doc.metadata.modified = unix_now();
            }
            if let Some(ref doc) = self.state.document {
                // The facade's `save` dispatches native formats by extension
                // (`.repub` plain, `.repubz` compressed, `.json` compact); it
                // ignores the font provider for native saves, but the signature
                // still requires one.
                let fonts = match repub_fonts::SystemFontProvider::new() {
                    Ok(fonts) => fonts,
                    Err(e) => {
                        log::error!("Failed to initialise fonts for save: {}", e);
                        self.error_alert.open(
                            "Couldn't Save",
                            format!("Repub couldn't save to \"{}\".\n\n{e}", path.display()),
                        );
                        return;
                    }
                };

                match repub_format::save(doc, &path, &fonts) {
                    Ok(()) => {
                        self.state.mark_saved(path.clone());
                        self.settings.add_recent_file(path);
                        self.settings.save();
                    }
                    Err(e) => {
                        log::error!("Failed to save document: {}", e);
                        self.error_alert.open(
                            "Couldn't Save",
                            format!("Repub couldn't save to \"{}\".\n\n{e}", path.display()),
                        );
                    }
                }
            }
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &egui::Context) -> Vec<AppAction> {
        let mut actions = Vec::new();

        // Check if any text field has focus - if so, skip most shortcuts
        // to allow normal text editing (cut/copy/paste in text fields)
        let widget_focused = ctx.memory(|m| m.focused().is_some())
            && ctx.input(|i| !i.modifiers.command || i.key_pressed(egui::Key::A));
        // Editing text on the canvas: keys belong to the text (the canvas handles them),
        // so only file and zoom shortcuts stay active here.
        let canvas_text = self.state.text_edit.is_some();
        let text_edit_focused = widget_focused || canvas_text;

        ctx.input(|i| {
            // Use command_only() which is Cmd on macOS, Ctrl on other platforms
            let cmd = i.modifiers.command;
            let cmd_shift = cmd && i.modifiers.shift;

            if i.key_pressed(egui::Key::F7) {
                actions.push(AppAction::CheckSpelling);
            }

            // File shortcuts (always active)
            if cmd && i.key_pressed(egui::Key::N) {
                actions.push(AppAction::NewDocument);
            }
            if cmd && i.key_pressed(egui::Key::O) {
                actions.push(AppAction::OpenDocument);
            }
            if cmd && i.key_pressed(egui::Key::S) {
                if i.modifiers.shift {
                    actions.push(AppAction::SaveDocumentAs);
                } else {
                    actions.push(AppAction::SaveDocument);
                }
            }
            if cmd && i.key_pressed(egui::Key::P) {
                actions.push(AppAction::Print);
            }
            if cmd && i.key_pressed(egui::Key::Q) {
                actions.push(AppAction::Quit);
            }
            if cmd && i.key_pressed(egui::Key::F) {
                actions.push(AppAction::Find);
            }
            if cmd && !text_edit_focused && i.key_pressed(egui::Key::G) {
                actions.push(if i.modifiers.shift {
                    AppAction::Ungroup
                } else {
                    AppAction::Group
                });
            }
            // Cmd+H hides the app on macOS, so Replace has no shortcut there
            if cmd && i.key_pressed(egui::Key::H) && !cfg!(target_os = "macos") {
                actions.push(AppAction::Replace);
            }

            // Text formatting (Publisher's shortcuts); also while editing text on
            // the canvas, but not while typing in a field such as the font size box
            if cmd && !widget_focused {
                let format_keys = [
                    (egui::Key::B, false, AppAction::TextBold),
                    (egui::Key::I, false, AppAction::TextItalic),
                    (egui::Key::U, false, AppAction::TextUnderline),
                    (egui::Key::L, false, AppAction::TextAlignLeft),
                    (egui::Key::E, false, AppAction::TextAlignCenter),
                    (egui::Key::R, false, AppAction::TextAlignRight),
                    (egui::Key::J, false, AppAction::TextAlignJustify),
                    (egui::Key::Period, true, AppAction::GrowFont),
                    (egui::Key::L, true, AppAction::ToggleBullets),
                    (egui::Key::Comma, true, AppAction::ShrinkFont),
                ];
                for (key, shift, action) in format_keys {
                    if i.key_pressed(key) && i.modifiers.shift == shift {
                        actions.push(action);
                    }
                }
            }
            if cmd && i.key_pressed(egui::Key::K) {
                actions.push(AppAction::InsertHyperlink);
            }

            // Edit shortcuts - only when no text field is focused
            // (let text fields handle their own cut/copy/paste/undo)
            if !text_edit_focused {
                if cmd && i.key_pressed(egui::Key::Z) {
                    if i.modifiers.shift {
                        actions.push(AppAction::Redo);
                    } else {
                        actions.push(AppAction::Undo);
                    }
                }
                if cmd && i.key_pressed(egui::Key::Y) {
                    actions.push(AppAction::Redo);
                }
                if cmd && i.key_pressed(egui::Key::X) {
                    actions.push(AppAction::Cut);
                }
                if cmd && i.key_pressed(egui::Key::C) {
                    actions.push(AppAction::Copy);
                }
                if cmd && i.key_pressed(egui::Key::V) {
                    if i.modifiers.shift {
                        actions.push(AppAction::PasteInPlace);
                    } else {
                        actions.push(AppAction::Paste);
                    }
                }
            }
            // Object editing shortcuts: not while a text field or canvas text has the keyboard
            // (Backspace in a text field must not delete the selected objects)
            if !text_edit_focused && cmd && i.key_pressed(egui::Key::D) {
                actions.push(AppAction::Duplicate);
            }
            if !text_edit_focused
                && (i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
            {
                // While editing points, Delete removes the selected point, not the shape
                if self
                    .state
                    .edit_points
                    .as_ref()
                    .is_some_and(|s| s.selected.is_some())
                {
                    actions.push(AppAction::EditPoint(crate::edit_points::PointOp::Delete));
                } else {
                    actions.push(AppAction::Delete);
                }
            }
            if !text_edit_focused && cmd && i.key_pressed(egui::Key::A) {
                actions.push(AppAction::SelectAll);
            }

            // Object ordering shortcuts (when objects selected)
            // Cmd+Shift+]: Bring to Front
            // Cmd+Shift+[: Send to Back
            // Cmd+]: Bring Forward
            // Cmd+[: Send Backward
            if !text_edit_focused && cmd && i.key_pressed(egui::Key::CloseBracket) {
                if i.modifiers.shift {
                    actions.push(AppAction::BringToFront);
                } else {
                    actions.push(AppAction::BringForward);
                }
            }
            if !text_edit_focused && cmd && i.key_pressed(egui::Key::OpenBracket) {
                if i.modifiers.shift {
                    actions.push(AppAction::SendToBack);
                } else {
                    actions.push(AppAction::SendBackward);
                }
            }

            // View shortcuts
            if cmd && (i.key_pressed(egui::Key::Plus) || i.key_pressed(egui::Key::Equals)) {
                actions.push(AppAction::ZoomIn);
            }
            if cmd && i.key_pressed(egui::Key::Minus) {
                actions.push(AppAction::ZoomOut);
            }
            if cmd && i.key_pressed(egui::Key::Num1) {
                actions.push(AppAction::ZoomActual);
            }
            if cmd && i.key_pressed(egui::Key::Num0) {
                actions.push(AppAction::ZoomFit);
            }

            // Tool shortcuts (only when no text field is focused)
            if !text_edit_focused {
                if i.key_pressed(egui::Key::V) && !cmd {
                    self.state.current_tool = Tool::Select;
                }
                if i.key_pressed(egui::Key::T) && !cmd {
                    self.state.current_tool = Tool::Text;
                }
                if i.key_pressed(egui::Key::R) && !cmd {
                    self.state.current_tool = Tool::Rectangle;
                }
                if i.key_pressed(egui::Key::E) && !cmd {
                    self.state.current_tool = Tool::Ellipse;
                }
                if i.key_pressed(egui::Key::L) && !cmd {
                    self.state.current_tool = Tool::Line;
                }
                if i.key_pressed(egui::Key::P) && !cmd {
                    self.state.current_tool = Tool::Polygon;
                }
                if i.key_pressed(egui::Key::S) && !cmd {
                    self.state.current_tool = Tool::Star;
                }
                if i.key_pressed(egui::Key::H) && !cmd {
                    self.state.current_tool = Tool::Pan;
                }
                if i.key_pressed(egui::Key::Z) && !cmd {
                    self.state.current_tool = Tool::Zoom;
                }
            }

            // Escape to cancel / deselect (also unfocuses text fields). While editing
            // canvas text, Escape only ends editing (handled by the canvas).
            if !canvas_text && i.key_pressed(egui::Key::Escape) {
                if self.state.edit_points.is_some() {
                    self.state.edit_points = None;
                }
                if self.state.is_format_painter_active() {
                    self.state.deactivate_format_painter();
                }
                self.state.clear_selection();
                self.canvas.draw_state.cancel();
                self.canvas.select_state.cancel();
            }

            // Arrow keys to move selected objects (only when no text field is focused)
            if !text_edit_focused && !self.state.selection.is_empty() {
                let step = if i.modifiers.shift { 10.0 } else { 1.0 };
                let mut delta_x = 0.0;
                let mut delta_y = 0.0;

                if i.key_pressed(egui::Key::ArrowUp) {
                    delta_y = -step;
                }
                if i.key_pressed(egui::Key::ArrowDown) {
                    delta_y = step;
                }
                if i.key_pressed(egui::Key::ArrowLeft) {
                    delta_x = -step;
                }
                if i.key_pressed(egui::Key::ArrowRight) {
                    delta_x = step;
                }

                if delta_x != 0.0 || delta_y != 0.0 {
                    self.move_selected_objects(delta_x, delta_y);
                }
            }
        });

        actions
    }

    /// Handle events from native menu bar.
    #[cfg(not(target_arch = "wasm32"))]
    fn handle_native_menu_events(&mut self, ctx: &egui::Context) {
        use crate::native_menu::MenuId;

        // Take the receiver temporarily to process events
        if let Some(rx) = self.native_menu_rx.take() {
            while let Ok(event) = rx.try_recv() {
                if let Some(action) = crate::native_menu::command_action(&event.id.0) {
                    self.process_action(action, ctx);
                    continue;
                }
                if let Some(menu_id) = MenuId::from_id(&event.id) {
                    let action = match menu_id {
                        MenuId::New => Some(AppAction::NewDocument),
                        MenuId::Open => Some(AppAction::OpenDocument),
                        MenuId::Save => Some(AppAction::SaveDocument),
                        MenuId::SaveAs => Some(AppAction::SaveDocumentAs),
                        MenuId::Export => Some(AppAction::Export),
                        MenuId::Undo => Some(AppAction::Undo),
                        MenuId::Redo => Some(AppAction::Redo),
                        MenuId::Cut => Some(AppAction::Cut),
                        MenuId::Copy => Some(AppAction::Copy),
                        MenuId::Paste => Some(AppAction::Paste),
                        MenuId::Duplicate => Some(AppAction::Duplicate),
                        MenuId::SelectAll => Some(AppAction::SelectAll),
                        MenuId::ZoomIn => Some(AppAction::ZoomIn),
                        MenuId::ZoomOut => Some(AppAction::ZoomOut),
                        MenuId::ZoomActual => Some(AppAction::ZoomActual),
                        MenuId::ZoomFit => Some(AppAction::ZoomFit),
                        MenuId::ToggleRulers => Some(AppAction::ToggleRulers),
                        MenuId::ToggleGrid => Some(AppAction::ToggleGrid),
                        MenuId::ToggleGuides => Some(AppAction::ToggleGuides),
                        MenuId::InsertText => Some(AppAction::InsertText),
                        MenuId::InsertImage => Some(AppAction::InsertImage),
                        MenuId::InsertRectangle => Some(AppAction::InsertRectangle),
                        MenuId::InsertEllipse => Some(AppAction::InsertEllipse),
                        MenuId::InsertLine => Some(AppAction::InsertLine),
                        MenuId::AddPage => Some(AppAction::AddPage),
                        MenuId::BringToFront => Some(AppAction::BringToFront),
                        MenuId::BringForward => Some(AppAction::BringForward),
                        MenuId::SendBackward => Some(AppAction::SendBackward),
                        MenuId::SendToBack => Some(AppAction::SendToBack),
                        MenuId::Group => Some(AppAction::Group),
                        MenuId::Ungroup => Some(AppAction::Ungroup),
                        MenuId::About => Some(AppAction::ShowAbout),
                        MenuId::Quit => Some(AppAction::Quit),
                        MenuId::Print => Some(AppAction::Print),
                        MenuId::PasteInPlace => Some(AppAction::PasteInPlace),
                        MenuId::Delete => Some(AppAction::Delete),
                        MenuId::AddPageAbove => Some(AppAction::InsertPageAt(self.context_page)),
                        MenuId::AddPageBelow => {
                            Some(AppAction::InsertPageAt(self.context_page + 1))
                        }
                        MenuId::DuplicatePage => Some(AppAction::DuplicatePage(self.context_page)),
                        MenuId::InsertSpread => Some(AppAction::InsertSpread(self.context_page)),
                        MenuId::DeletePage => Some(AppAction::DeletePage(self.context_page)),
                        MenuId::EditText => Some(AppAction::EditText),
                        MenuId::Table(op) => Some(AppAction::TableEdit(op)),
                        MenuId::ChangePicture => Some(AppAction::ChangePicture),
                        MenuId::WrapText => Some(AppAction::ShowTextWrap),
                    };

                    if let Some(action) = action {
                        self.process_action(action, ctx);
                    }
                }
            }
            // Put the receiver back
            self.native_menu_rx = Some(rx);
        }

        // Request repaint to ensure UI updates after menu action
        ctx.request_repaint();
    }
}

impl eframe::App for RepubApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame(ctx);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.settings.save();
    }

    #[cfg(target_arch = "wasm32")]
    fn on_exit(&mut self) {
        self.settings.save();
    }
}

impl RepubApp {
    /// Build one frame of the whole UI. Called by eframe every frame, and by
    /// tests that drive the app with synthetic input through `egui::Context::run`.
    pub fn frame(&mut self, ctx: &egui::Context) {
        // Match System Appearance follows the desktop as it changes
        crate::theme::sync(ctx, self.settings.theme);
        // Document text slots are shared by the page thumbnails and the canvas
        crate::sdf_text::begin_frame();
        #[cfg(target_arch = "wasm32")]
        self.process_web_events();
        // A master page being edited and the document stay in step (undo, edits)
        self.state.sync_master_edit();
        if let Some(link) = self.state.follow_link.take() {
            self.follow_link(&link);
        }
        if let Some((title, text)) = self.state.alert.take() {
            self.error_alert.open(title, text);
        }
        // Groups enclose their members again after a member was moved on its own
        if let Some(page) = self.state.current_page_mut() {
            page.refresh_group_bounds();
        }
        // Remember the measurement unit when it's changed (Format pane)
        if self.state.units.key() != self.settings.measurement_unit {
            self.settings.measurement_unit = self.state.units.key().to_owned();
            self.settings.save();
        }

        // Closing the window with unsaved changes asks first
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            if self.save_prompt.is_open() || !self.confirm_discard(DiscardOp::Quit) {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            }
        }

        // Update window title
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.state.window_title()));

        // Handle native menu events (macOS/Windows/Linux)
        #[cfg(not(target_arch = "wasm32"))]
        self.handle_native_menu_events(ctx);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let checks = [
                self.state.show_rulers,
                self.state.show_grid,
                self.state.show_guides,
                self.settings.left_panel_visible,
                self.settings.selection_pane_visible,
                self.state.two_page_spread,
            ];
            if self.view_checks != Some(checks) {
                if let Some(menu) = &self.native_menu {
                    let ids = [
                        "toggle_rulers",
                        "toggle_grid",
                        "toggle_guides",
                        "cmd.toggle_pages",
                        "cmd.toggle_selection",
                        "cmd.toggle_spread",
                    ];
                    let pairs: Vec<(&str, bool)> = ids.into_iter().zip(checks).collect();
                    crate::native_menu::sync_checks(menu, &pairs);
                }
                self.view_checks = Some(checks);
            }
        }

        // Handle keyboard shortcuts. Backstage covers the canvas, so editing
        // shortcuts (e.g. Delete) must not act on objects the user can't see.
        if !self.start_screen.is_backstage_open() {
            let kb_actions = self.handle_keyboard_shortcuts(ctx);
            for action in kb_actions {
                self.process_action(action, ctx);
            }
        }

        // macOS: our own title row under the transparent native title bar
        #[cfg(target_os = "macos")]
        {
            let fullscreen = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            let inset = crate::panels::title_bar::toolbar_inset(fullscreen);
            let title_actions = egui::TopBottomPanel::top("title_bar")
                .frame(egui::Frame::none().fill(crate::theme::colors::title_bar()))
                .exact_height(crate::theme::metrics::TITLE_BAR_HEIGHT)
                .show_separator_line(false)
                .show(ctx, |ui| {
                    crate::panels::title_bar::show(ui, &self.state, inset)
                })
                .inner;
            for action in title_actions {
                self.process_action(action, ctx);
            }
        }

        // Ribbon (tab row + command body)
        let ribbon_actions = egui::TopBottomPanel::top("toolbar")
            .frame(crate::theme::ribbon_frame())
            .resizable(false)
            .show_separator_line(false)
            .show(ctx, |ui| {
                // The ribbon sizes itself (it shrinks to the tab row when collapsed)
                let ribbon_ctx = crate::panels::toolbar::RibbonContext {
                    clipboard_empty: self.clipboard.objects.is_empty(),
                    pages_pane_visible: self.settings.left_panel_visible,
                    selection_pane_visible: self.settings.selection_pane_visible,
                };
                self.toolbar.show(ui, &mut self.state, &ribbon_ctx)
            })
            .inner;
        for action in ribbon_actions {
            self.process_action(action, ctx);
        }

        // Status bar with custom frame
        let status_bar_action = egui::TopBottomPanel::bottom("status_bar")
            .frame(crate::theme::status_bar_frame())
            .exact_height(crate::theme::metrics::STATUS_BAR_HEIGHT)
            .show(ctx, |ui| {
                // Union of the selected objects' bounds
                let selection = self
                    .state
                    .selected_objects()
                    .iter()
                    .map(|o| o.bounds())
                    .reduce(|a, b| a.union(&b));
                StatusBar::show_full(
                    ui,
                    StatusBarInfo {
                        pages_on_view: self.state.visible_pages(),
                        total_pages: self.state.page_count(),
                        zoom: self.state.zoom,
                        two_page_spread: self.state.two_page_spread,
                        position: selection
                            .map(|r| (r.origin.x, r.origin.y))
                            .or(self.state.pointer_doc.map(|p| (p.x, p.y))),
                        selection_size: selection.map(|r| (r.size.width, r.size.height)),
                        units: self.state.units,
                    },
                )
            })
            .inner;

        // Handle status bar actions
        if let Some(action) = status_bar_action {
            match action {
                StatusBarAction::SetZoom(zoom) => {
                    if zoom < 0.0 {
                        // Negative is the "Fit to Window" sentinel
                        self.state.fit_page_pending = true;
                    } else {
                        self.state.set_zoom_centered(zoom);
                    }
                }
                StatusBarAction::TogglePagesPane => {
                    self.process_action(AppAction::TogglePagesPane, ctx)
                }
                StatusBarAction::SetTwoPageSpread(on) => {
                    if self.state.two_page_spread != on {
                        self.process_action(AppAction::ToggleTwoPageSpread, ctx);
                    }
                }
            }
        }

        // Pages panel (left) with custom frame
        if self.settings.left_panel_visible && self.state.has_document() {
            egui::SidePanel::left("pages_panel")
                .frame(crate::theme::side_panel_frame())
                .default_width(self.settings.left_panel_width)
                .resizable(true)
                .show(ctx, |ui| {
                    self.pages_panel.show(ui, &mut self.state, &mut self.canvas);
                });
            if std::mem::take(&mut self.pages_panel.collapse_requested) {
                self.process_action(AppAction::TogglePagesPane, ctx);
            }
        } else if self.state.has_document() {
            // Collapsed: a narrow strip with an expand chevron and a vertical "Pages" label
            egui::SidePanel::left("pages_collapsed")
                .frame(crate::theme::side_panel_frame())
                .exact_width(26.0)
                .resizable(false)
                .show(ctx, |ui| {
                    let strip = ui.max_rect();
                    let response =
                        ui.interact(strip, ui.id().with("expand_pages"), egui::Sense::click());
                    if response.hovered() {
                        ui.painter()
                            .rect_filled(strip, 0.0, crate::theme::colors::hover());
                    }
                    let chevron = egui::Rect::from_center_size(
                        egui::pos2(strip.center().x, strip.top() + 16.0),
                        egui::Vec2::splat(12.0),
                    );
                    crate::icons::Icon::ChevronRight.paint(
                        ui,
                        chevron,
                        crate::theme::colors::text_secondary(),
                    );
                    let galley = ui.painter().layout_no_wrap(
                        "Pages".to_owned(),
                        egui::FontId::proportional(14.0),
                        crate::theme::colors::accent(),
                    );
                    // Rotating +90° swings the text to the left of its anchor
                    let anchor =
                        egui::pos2(strip.center().x + galley.size().y / 2.0, strip.top() + 34.0);
                    ui.painter().add(
                        egui::epaint::TextShape::new(
                            anchor,
                            galley,
                            crate::theme::colors::accent(),
                        )
                        .with_angle(std::f32::consts::FRAC_PI_2),
                    );
                    if response
                        .on_hover_text("Expand the Page Navigation pane")
                        .clicked()
                    {
                        self.process_action(AppAction::TogglePagesPane, ctx);
                    }
                });
        }

        // Selection pane (right, outside the Format pane)
        if self.settings.selection_pane_visible && self.state.has_document() {
            egui::SidePanel::right("selection_pane")
                .frame(crate::theme::side_panel_frame())
                .default_width(220.0)
                .resizable(true)
                .show(ctx, |ui| {
                    crate::panels::selection::show(ui, &mut self.state)
                });
        }

        // Central canvas with rulers, or the start screen when no document is open
        let canvas_action = if self.state.has_document() {
            egui::CentralPanel::default()
                .frame(crate::theme::canvas_frame())
                .show(ctx, |ui| self.show_canvas_with_rulers(ui))
                .inner
        } else {
            let start_action = egui::CentralPanel::default()
                .frame(egui::Frame::none().fill(crate::theme::colors::backstage_body()))
                .show(ctx, |ui| {
                    self.start_screen
                        .show_start(ui, &self.settings.recent_files)
                })
                .inner;
            if let Some(action) = start_action {
                self.handle_start_screen_action(action, ctx);
            }
            None
        };

        if let Some(action) = canvas_action {
            self.process_action(action, ctx);
        }

        // Process native context menu requests (macOS)
        #[cfg(target_os = "macos")]
        {
            let canvas_request = self.canvas.pending_context_menu.take();
            let page_request = self.pages_panel.pending_context_menu.take();
            if let Some(page_index) = page_request.map(|(i, _, _)| i) {
                self.context_page = page_index;
            }
            let menu_state = crate::native_menu::ContextMenuState {
                clipboard_empty: self.clipboard.objects.is_empty(),
                selection_count: self.state.selection.len(),
                page_count: self.state.page_count(),
                group_selected: self
                    .state
                    .selected_objects()
                    .iter()
                    .any(|o| matches!(o.kind, ShapeKind::Group(_))),
                single_text: matches!(self.state.selected_objects()[..], [o] if matches!(o.kind, ShapeKind::TextFrame(_) | ShapeKind::Table(_))),
                single_picture: matches!(self.state.selected_objects()[..], [o] if matches!(o.kind, ShapeKind::Picture(_))),
                two_page_spread: self.state.two_page_spread,
                single_table: matches!(self.state.selected_objects()[..], [o] if matches!(o.kind, ShapeKind::Table(_))),
                in_cell: self
                    .state
                    .text_edit
                    .as_ref()
                    .is_some_and(|e| e.cell.is_some()),
            };
            if let Some(menus) = self.native_context_menus.as_mut() {
                menus.update(&menu_state);
                // Canvas context menu
                if let Some((menu_type, x, y)) = canvas_request {
                    menus.request_context_menu(menu_type, x, y);
                    menus.process_pending();
                }
                // Pages panel context menu
                if let Some((_, x, y)) = page_request {
                    menus.request_context_menu(crate::native_menu::ContextMenuType::Page, x, y);
                    menus.process_pending();
                }
            }
        }

        // Dialogs
        if let Some(result) = self.new_doc_dialog.show(ctx) {
            let op = DiscardOp::New {
                page_size: result.page_size,
                orientation: result.orientation,
                pages: result.pages,
            };
            if self.confirm_discard(op) {
                self.create_document(result.page_size, result.orientation, result.pages);
            }
        }

        if let Some(doc) = self.templates_dialog.show(ctx) {
            if self.confirm_discard(DiscardOp::Template(Box::new(doc.clone()))) {
                self.install_new_document(doc);
            }
        }

        // Unsaved-changes prompt and error messages sit above everything else
        if let Some(choice) = self.save_prompt.show(ctx) {
            self.answer_save_prompt(choice, ctx);
        }
        self.error_alert.show(ctx);

        // Backstage (File tab) over the whole window
        if self.state.has_document() && self.start_screen.is_backstage_open() {
            let info = self.backstage_document_info();
            if let Some(action) =
                self.start_screen
                    .show_backstage(ctx, &self.settings.recent_files, info)
            {
                self.handle_start_screen_action(action, ctx);
            }
        }

        if let Some(options) =
            self.export_dialog
                .show(ctx, self.state.page_count(), self.state.current_page)
        {
            // Handle export
            self.export_with_options(options);
        }

        self.about_dialog.show(ctx);

        // Page properties dialog
        self.page_properties_dialog.show(ctx, &mut self.state);

        // Insert table dialog
        if let Some((rows, cols)) = self.insert_table_dialog.show(ctx) {
            self.insert_table(rows, cols);
        }

        // Text fit dialog
        self.text_fit_dialog.show(ctx, &mut self.state);

        // Text columns dialog
        self.text_columns_dialog.show(ctx, &mut self.state);

        // Text wrap dialog
        self.text_wrap_dialog.show(ctx, &mut self.state);
        self.format_shape_dialog.show(ctx, &mut self.state);
        self.document_properties_dialog.show(ctx, &mut self.state);
        self.font_dialog.show(ctx, &mut self.state);
        self.paragraph_dialog.show(ctx, &mut self.state);
        if let Some(index) = self.state.page_properties_requested.take() {
            self.page_properties_dialog
                .open_for_index(&self.state, index);
        }
        if std::mem::take(&mut self.state.edit_wordart_requested) {
            self.wordart_dialog.open_for_selection(&self.state);
        }
        self.table_properties_dialog.show(ctx, &mut self.state);
        self.wordart_dialog.show(ctx, &mut self.state);

        // Text box margins dialog
        self.text_box_margins_dialog.show(ctx, &mut self.state);

        // Find and replace
        self.find_replace_dialog.show(ctx, &mut self.state);

        // Insert hyperlink
        self.hyperlink_dialog.show(ctx, &mut self.state);

        // Spelling
        self.spelling_dialog.show(ctx, &mut self.state);

        // Move / rename page dialogs
        self.move_page_dialog.show(ctx, &mut self.state);
        self.rename_page_dialog.show(ctx, &mut self.state);
    }
}

impl RepubApp {
    /// Show the canvas with optional rulers.
    fn show_canvas_with_rulers(&mut self, ui: &mut egui::Ui) -> Option<AppAction> {
        use crate::canvas::{page_step_button, workspace_scrollbar, PageLayout, SCROLLBAR_SIZE};

        let show_rulers = self.state.show_rulers && self.state.has_document();
        let ruler_size = if show_rulers { Ruler::SIZE } else { 0.0 };
        let bar_size = if self.state.has_document() {
            SCROLLBAR_SIZE
        } else {
            0.0
        };

        // Apply a pending whole-page fit now that the viewport size is known,
        // before rulers and canvas read the zoom for this frame
        // Panels and the window can still change size during the first frames,
        // so keep fitting until the viewport is the same size two frames running.
        if self.state.fit_page_pending && self.state.has_document() {
            let viewport = ui.available_rect_before_wrap().size();
            let inset = ruler_size + bar_size;
            self.state
                .zoom_to_fit(viewport.x - inset, viewport.y - inset);
            self.state.pan_offset = Point::new(0.0, 0.0);
            if self.last_fit_viewport == Some(viewport) {
                self.state.fit_page_pending = false;
                self.last_fit_viewport = None;
            } else {
                self.last_fit_viewport = Some(viewport);
                ui.ctx().request_repaint();
            }
        }

        if !self.state.has_document() {
            return self.canvas.show(ui, &mut self.state);
        }

        // Layout: corner + horizontal ruler on top, vertical ruler on the left,
        // scroll bars on the right and bottom, previous/next page buttons under
        // the vertical scroll bar, canvas in the middle.
        let full = ui.available_rect_before_wrap();
        ui.advance_cursor_after_rect(full);
        let canvas_rect = egui::Rect::from_min_max(
            full.min + egui::vec2(ruler_size, ruler_size),
            full.max - egui::vec2(bar_size, bar_size),
        );
        let corner_rect = egui::Rect::from_min_size(full.min, egui::Vec2::splat(ruler_size));
        let h_ruler_rect = egui::Rect::from_x_y_ranges(
            canvas_rect.left()..=full.right(),
            full.top()..=canvas_rect.top(),
        );
        let v_ruler_rect = egui::Rect::from_x_y_ranges(
            full.left()..=canvas_rect.left(),
            canvas_rect.top()..=full.bottom(),
        );
        let page_button_h = SCROLLBAR_SIZE + 2.0;
        let v_bar_rect = egui::Rect::from_x_y_ranges(
            canvas_rect.right()..=full.right(),
            canvas_rect.top()..=canvas_rect.bottom() - 2.0 * page_button_h,
        );
        let prev_rect = egui::Rect::from_x_y_ranges(
            canvas_rect.right()..=full.right(),
            v_bar_rect.bottom()..=v_bar_rect.bottom() + page_button_h,
        );
        let next_rect = prev_rect.translate(egui::vec2(0.0, page_button_h));
        let h_bar_rect = egui::Rect::from_x_y_ranges(
            canvas_rect.left()..=canvas_rect.right(),
            canvas_rect.bottom()..=full.bottom(),
        );
        let bar_corner = egui::Rect::from_min_max(canvas_rect.max, full.max);

        // Page placement must match Canvas::show (rulers measure from the current
        // page, which may be either side of a two-page spread)
        let layout = PageLayout::compute(canvas_rect, &self.state);
        let page_rect = layout
            .rect_of(self.state.current_page)
            .unwrap_or(egui::Rect::NOTHING);
        let (page_x, page_y, page_w, page_h) = (
            page_rect.left(),
            page_rect.top(),
            page_rect.width(),
            page_rect.height(),
        );
        let view_size = layout.bounds().map_or(egui::Vec2::ZERO, |b| b.size());

        // Ruler mouse indicator only while the pointer is over the workspace
        let mouse_pos = ui
            .ctx()
            .input(|i| i.pointer.hover_pos())
            .filter(|p| canvas_rect.contains(*p));
        let mouse_doc_x = mouse_pos.map(|p| (p.x - page_x) / self.state.zoom);
        let mouse_doc_y = mouse_pos.map(|p| (p.y - page_y) / self.state.zoom);

        if show_rulers {
            // Corner square where the rulers meet
            let painter = ui.painter_at(corner_rect);
            painter.rect_filled(corner_rect, 0.0, crate::theme::colors::ruler_bg());
            painter.rect_stroke(
                corner_rect.shrink(0.5),
                0.0,
                egui::Stroke::new(1.0, crate::theme::colors::ruler_border()),
            );

            let zoom = self.state.zoom;
            let (h_response, h_interaction) = ui
                .allocate_new_ui(egui::UiBuilder::new().max_rect(h_ruler_rect), |ui| {
                    Ruler::new(RulerOrientation::Horizontal)
                        .page_origin(page_x)
                        .page_size(page_w)
                        .cross_origin(page_y)
                        .zoom(zoom)
                        .mouse_doc_pos(mouse_doc_x)
                        .show(ui, h_ruler_rect.width())
                })
                .inner;
            let (v_response, v_interaction) = ui
                .allocate_new_ui(egui::UiBuilder::new().max_rect(v_ruler_rect), |ui| {
                    Ruler::new(RulerOrientation::Vertical)
                        .page_origin(page_y)
                        .page_size(page_h)
                        .cross_origin(page_x)
                        .zoom(zoom)
                        .mouse_doc_pos(mouse_doc_y)
                        .show(ui, v_ruler_rect.height())
                })
                .inner;

            // Dragging off a ruler creates a guide
            for (interaction, response, orientation, extent) in [
                (
                    h_interaction,
                    h_response,
                    GuideOrientation::Horizontal,
                    page_h,
                ),
                (
                    v_interaction,
                    v_response,
                    GuideOrientation::Vertical,
                    page_w,
                ),
            ] {
                match interaction {
                    Some(RulerInteraction::CreateGuide(pos)) => {
                        self.canvas.guide_state.creating_guide = Some(orientation);
                        self.canvas.guide_state.preview_position = Some(pos);
                    }
                    Some(RulerInteraction::FinishGuide(pos)) => {
                        self.canvas.guide_state.creating_guide = None;
                        self.canvas.guide_state.preview_position = None;
                        // Only create the guide if it is on or near the page
                        if pos >= -50.0 && pos <= (extent / zoom) as f64 + 50.0 {
                            self.state.add_guide(match orientation {
                                GuideOrientation::Horizontal => Guide::horizontal(pos),
                                GuideOrientation::Vertical => Guide::vertical(pos),
                            });
                        }
                    }
                    None => {
                        if !response.dragged()
                            && self.canvas.guide_state.creating_guide == Some(orientation)
                        {
                            self.canvas.guide_state.creating_guide = None;
                            self.canvas.guide_state.preview_position = None;
                        }
                    }
                }
            }
        }

        // Scroll bars (extent = page plus one viewport of margin on each side)
        workspace_scrollbar(
            ui,
            v_bar_rect,
            true,
            view_size.y,
            canvas_rect.height(),
            &mut self.state.pan_offset.y,
        );
        workspace_scrollbar(
            ui,
            h_bar_rect,
            false,
            view_size.x,
            canvas_rect.width(),
            &mut self.state.pan_offset.x,
        );
        ui.painter()
            .rect_filled(bar_corner, 0.0, crate::theme::colors::scrollbar_track());

        // Previous/next page, or previous/next spread in two-page view
        let on_view = self.state.visible_pages();
        let first = on_view.first().copied().unwrap_or(0);
        let last = on_view.last().copied().unwrap_or(0);
        let count = self.state.page_count();
        if page_step_button(ui, prev_rect, true, first > 0) {
            self.state.set_page(first - 1);
        }
        if page_step_button(ui, next_rect, false, last + 1 < count) {
            self.state.set_page(last + 1);
        }

        let action = ui
            .allocate_new_ui(egui::UiBuilder::new().max_rect(canvas_rect), |ui| {
                self.canvas.show(ui, &mut self.state)
            })
            .inner;

        // Banners over the canvas for modes, each saying what's going on and
        // giving the way out: master page editing, Create Link, Format Painter
        let mut action = action;
        let mut banners: Vec<(String, &str, &str, AppAction)> = Vec::new();
        if let Some(edit) = &self.state.master_edit {
            let name = edit
                .page
                .name
                .clone()
                .unwrap_or_else(|| "Master Page".to_owned());
            banners.push((
                format!("Editing {name}"),
                "Changes appear on every page that uses this master.",
                "Close Master Page",
                AppAction::ExitMasterPageEdit,
            ));
        }
        if self.state.link_source.is_some() {
            banners.push((
                "Create Link".to_owned(),
                "Click an empty text box for the text to continue in, or empty space for a new box. Esc cancels.",
                "Cancel",
                AppAction::CancelMode,
            ));
        }
        if self.state.is_format_painter_active() {
            banners.push((
                "Format Painter".to_owned(),
                "Click an object to give it the copied formatting. Esc cancels.",
                "Cancel",
                AppAction::CancelMode,
            ));
        }
        for (i, (title, detail, button, on_click)) in banners.into_iter().enumerate() {
            let bar = egui::Rect::from_min_size(
                canvas_rect.min + egui::vec2(0.0, 30.0 * i as f32),
                egui::vec2(canvas_rect.width(), 30.0),
            );
            ui.painter()
                .rect_filled(bar, 0.0, crate::theme::colors::accent_light());
            ui.painter().hline(
                bar.x_range(),
                bar.bottom(),
                egui::Stroke::new(1.0, crate::theme::colors::accent()),
            );
            ui.allocate_new_ui(
                egui::UiBuilder::new().max_rect(bar.shrink2(egui::vec2(10.0, 3.0))),
                |ui| {
                    // The button first (from the right), so long text truncates instead of hiding it
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(button).clicked() {
                            action = Some(on_click);
                        }
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(title)
                                    .strong()
                                    .color(crate::theme::colors::text_primary()),
                            );
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(detail)
                                        .color(crate::theme::colors::text_secondary()),
                                )
                                .truncate(),
                            );
                        });
                    });
                },
            );
        }

        // Keep panning within the scrollable extent, as Publisher does
        if let Some((w, h)) = self
            .state
            .visible_pages_size()
            .map(|(w, h)| (w as f32 * self.state.zoom, h as f32 * self.state.zoom))
        {
            let max_x = ((canvas_rect.width() + w) / 2.0) as f64;
            let max_y = ((canvas_rect.height() + h) / 2.0) as f64;
            self.state.pan_offset.x = self.state.pan_offset.x.clamp(-max_x, max_x);
            self.state.pan_offset.y = self.state.pan_offset.y.clamp(-max_y, max_y);
        }

        action
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn export_with_options(&mut self, options: crate::dialogs::export::ExportOptions) {
        use crate::dialogs::export::ExportFormat;

        let Some(doc) = self.state.document.as_ref() else {
            return;
        };
        if options.pages.is_empty() {
            self.error_alert.open(
                "Nothing to Export",
                "The page range doesn't include any pages of this publication.",
            );
            return;
        }
        let (filter, ext) = (options.format.label(), options.format.extension());
        let stem = self
            .state
            .document_path
            .as_ref()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_owned());
        let Some(path) = rfd::FileDialog::new()
            .add_filter(filter, &[ext])
            .set_file_name(format!("{stem}.{ext}"))
            .save_file()
        else {
            return;
        };

        if let Err(e) = export_pages(doc, &options, &path) {
            log::error!("Export failed: {e}");
            self.error_alert.open(
                "Couldn't Export",
                format!("Repub couldn't export to \"{}\".\n\n{e}", path.display()),
            );
        }
    }
}

/// An operation that would discard the open document, held while the user
/// answers the save-changes prompt.
#[derive(Debug, Clone)]
enum DiscardOp {
    Close,
    Quit,
    /// Open a file; `None` shows the file picker.
    Open(Option<PathBuf>),
    /// Start a document made from a template.
    Template(Box<Document>),
    New {
        page_size: repub_model::PageSize,
        orientation: repub_model::PageOrientation,
        pages: u32,
    },
}

/// Whether `path` is in Repub's own format, so Save can write it in place.
fn is_native_document(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("repub") || e.eq_ignore_ascii_case("repubz"))
}

fn export_pages(
    doc: &Document,
    options: &crate::dialogs::export::ExportOptions,
    path: &Path,
) -> Result<(), String> {
    for (page, bytes) in export_files(doc, options)? {
        let target = match page {
            Some(page) if options.pages.len() != 1 => {
                let stem = path
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let ext = path
                    .extension()
                    .map(|e| e.to_string_lossy().into_owned())
                    .unwrap_or_default();
                path.with_file_name(format!("{stem}-{}.{ext}", page + 1))
            }
            _ => path.to_path_buf(),
        };
        std::fs::write(&target, bytes).map_err(|e| format!("{}: {e}", target.display()))?;
    }
    Ok(())
}

/// The contents of each file an export produces: one for PDF and HTML (page `None`), and one per
/// chosen page (`Some(page)`, 0-based) for SVG and the image formats.
fn export_files(
    doc: &Document,
    options: &crate::dialogs::export::ExportOptions,
) -> Result<Vec<(Option<usize>, Vec<u8>)>, String> {
    use crate::dialogs::export::ExportFormat;
    use repub_format::{
        export_html_string, export_image_bytes, export_pdf_bytes, export_svg_bytes,
        ExportFormat as ImageFormat_, HtmlOptions, PdfOptions, RasterExportOptions, SvgOptions,
    };

    // The Scene-driven exporters need a font provider to resolve and measure fonts.
    let fonts = repub_fonts::SystemFontProvider::new().map_err(|e| e.to_string())?;

    // A copy holding only the chosen pages, in order
    let subset = || {
        let mut subset = doc.clone();
        if options.pages.len() != doc.pages().len() {
            for i in (0..doc.pages().len()).rev() {
                if !options.pages.contains(&i) {
                    subset.remove_page(i);
                }
            }
        }
        subset
    };
    match options.format {
        ExportFormat::Pdf => {
            let bytes = export_pdf_bytes(&subset(), &PdfOptions::default(), &fonts)
                .map_err(|e| e.to_string())?;
            Ok(vec![(None, bytes)])
        }
        ExportFormat::Html => Ok(vec![(
            None,
            export_html_string(&subset(), &HtmlOptions::default()).into_bytes(),
        )]),
        ExportFormat::Svg => options
            .pages
            .iter()
            .map(|&page| {
                export_svg_bytes(doc, page, &SvgOptions::default(), &fonts)
                    .map(|bytes| (Some(page), bytes))
                    .map_err(|e| e.to_string())
            })
            .collect(),
        raster => {
            let format = match raster {
                ExportFormat::Jpeg => ImageFormat_::Jpeg,
                ExportFormat::Tiff => ImageFormat_::Tiff,
                _ => ImageFormat_::Png,
            };
            let image_opts = RasterExportOptions {
                format,
                dpi: options.dpi as f64,
                quality: options.quality,
                ..Default::default()
            };
            options
                .pages
                .iter()
                .map(|&page| {
                    export_image_bytes(doc, page, &image_opts, &fonts)
                        .map(|bytes| (Some(page), bytes))
                        .map_err(|e| e.to_string())
                })
                .collect()
        }
    }
}

fn unix_now() -> Option<i64> {
    crate::platform::unix_now()
}

/// Whether the shape's own fill is drawn (auto shapes, text boxes, pictures, WordArt).
fn has_fill(shape: &Shape) -> bool {
    matches!(
        shape.kind,
        ShapeKind::AutoShape(_)
            | ShapeKind::TextFrame(_)
            | ShapeKind::Picture(_)
            | ShapeKind::TextEffect(_)
    )
}

/// Whether the shape has an outline to set (everything with a fill, and lines).
fn has_outline(shape: &Shape) -> bool {
    has_fill(shape) || matches!(shape.kind, ShapeKind::Line(_))
}

#[cfg(test)]
mod export_tests {
    use super::*;
    use crate::dialogs::export::{ExportFormat, ExportOptions};

    fn three_pages() -> Document {
        let mut doc = Document::new(
            repub_model::PageSize::Letter,
            repub_model::PageOrientation::Portrait,
        );
        doc.add_page();
        doc.add_page();
        doc
    }

    fn options(format: ExportFormat, pages: Vec<usize>) -> ExportOptions {
        ExportOptions {
            format,
            dpi: 36,
            pages,
            quality: 90,
        }
    }

    fn out_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("repub-export-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn one_page_writes_exactly_the_chosen_file() {
        let dir = out_dir("single");
        let path = dir.join("doc.png");
        export_pages(&three_pages(), &options(ExportFormat::Png, vec![1]), &path).unwrap();
        assert!(path.exists());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
    }

    #[test]
    fn several_pages_are_numbered_by_page() {
        let dir = out_dir("multi");
        export_pages(
            &three_pages(),
            &options(ExportFormat::Jpeg, vec![0, 2]),
            &dir.join("doc.jpg"),
        )
        .unwrap();
        assert!(dir.join("doc-1.jpg").exists());
        assert!(dir.join("doc-3.jpg").exists());
        assert!(!dir.join("doc-2.jpg").exists());
        export_pages(
            &three_pages(),
            &options(ExportFormat::Svg, vec![1, 2]),
            &dir.join("doc.svg"),
        )
        .unwrap();
        assert!(dir.join("doc-2.svg").exists() && dir.join("doc-3.svg").exists());
    }

    #[test]
    fn every_format_in_the_dialog_exports() {
        let dir = out_dir("formats");
        for format in ExportFormat::ALL {
            let path = dir.join(format!("doc.{}", format.extension()));
            export_pages(&three_pages(), &options(format, vec![0]), &path)
                .unwrap_or_else(|e| panic!("{}: {e}", format.label()));
            assert!(path.exists(), "{}", format.label());
        }
    }

    #[test]
    fn pdf_holds_only_the_chosen_pages() {
        let dir = out_dir("pdf");
        let (all, one) = (dir.join("all.pdf"), dir.join("one.pdf"));
        export_pages(
            &three_pages(),
            &options(ExportFormat::Pdf, vec![0, 1, 2]),
            &all,
        )
        .unwrap();
        export_pages(&three_pages(), &options(ExportFormat::Pdf, vec![1]), &one).unwrap();
        let pdf = |p: &Path| String::from_utf8_lossy(&std::fs::read(p).unwrap()).into_owned();
        assert!(pdf(&all).contains("/Count 3"));
        assert!(pdf(&one).contains("/Count 1"));
    }
}

#[cfg(not(target_arch = "wasm32"))]
/// Open `path` in the system's default application for it.
fn open_in_system_viewer(path: &Path) -> std::io::Result<()> {
    #[cfg(target_os = "macos")]
    let mut command = std::process::Command::new("open");
    #[cfg(target_os = "windows")]
    let mut command = {
        let mut c = std::process::Command::new("cmd");
        c.args(["/C", "start", ""]);
        c
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = std::process::Command::new("xdg-open");
    command.arg(path).spawn().map(|_| ())
}
