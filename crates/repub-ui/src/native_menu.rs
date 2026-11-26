//! Native menu bar implementation using muda.
//!
//! This module provides native macOS/Windows/Linux menu bar integration,
//! including context menus (right-click menus).

use muda::{
    accelerator::{Accelerator, Code, Modifiers},
    CheckMenuItem, ContextMenu, Menu, MenuEvent, MenuItem, MenuItemKind, PredefinedMenuItem, Submenu,
};
use std::sync::mpsc;

/// Menu item IDs for custom actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuId {
    // File
    New,
    Open,
    Save,
    SaveAs,
    Export,
    
    // Edit
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Duplicate,
    SelectAll,
    
    // View
    ZoomIn,
    ZoomOut,
    ZoomActual,
    ZoomFit,
    ToggleRulers,
    ToggleGrid,
    ToggleGuides,
    
    // Insert
    InsertText,
    InsertImage,
    InsertRectangle,
    InsertEllipse,
    InsertLine,
    AddPage,
    
    // Arrange
    BringToFront,
    BringForward,
    SendBackward,
    SendToBack,
    Group,
    Ungroup,
    
    // Help
    About,

    // App
    Quit,
    Print,

    // Context menus
    PasteInPlace,
    Delete,
    AddPageAbove,
    AddPageBelow,
    DuplicatePage,
    DeletePage,
    InsertSpread,
    EditText,
    Table(crate::app::TableEdit),
    ChangePicture,
    WrapText,
}

impl MenuId {
    /// Convert a muda MenuId string to our MenuId.
    pub fn from_id(id: &muda::MenuId) -> Option<Self> {
        let s = id.0.as_str();
        match s {
            "new" => Some(Self::New),
            "open" => Some(Self::Open),
            "save" => Some(Self::Save),
            "save_as" => Some(Self::SaveAs),
            "export" => Some(Self::Export),
            "undo" => Some(Self::Undo),
            "redo" => Some(Self::Redo),
            "cut" => Some(Self::Cut),
            "copy" => Some(Self::Copy),
            "paste" => Some(Self::Paste),
            "duplicate" => Some(Self::Duplicate),
            "select_all" => Some(Self::SelectAll),
            "zoom_in" => Some(Self::ZoomIn),
            "zoom_out" => Some(Self::ZoomOut),
            "zoom_actual" => Some(Self::ZoomActual),
            "zoom_fit" => Some(Self::ZoomFit),
            "toggle_rulers" => Some(Self::ToggleRulers),
            "toggle_grid" => Some(Self::ToggleGrid),
            "toggle_guides" => Some(Self::ToggleGuides),
            "insert_text" => Some(Self::InsertText),
            "insert_image" => Some(Self::InsertImage),
            "insert_rectangle" => Some(Self::InsertRectangle),
            "insert_ellipse" => Some(Self::InsertEllipse),
            "insert_line" => Some(Self::InsertLine),
            "add_page" => Some(Self::AddPage),
            "bring_to_front" => Some(Self::BringToFront),
            "bring_forward" => Some(Self::BringForward),
            "send_backward" => Some(Self::SendBackward),
            "send_to_back" => Some(Self::SendToBack),
            "group" => Some(Self::Group),
            "ungroup" => Some(Self::Ungroup),
            "about" => Some(Self::About),
            "quit" => Some(Self::Quit),
            "print" => Some(Self::Print),
            "paste_in_place" => Some(Self::PasteInPlace),
            "delete" => Some(Self::Delete),
            "add_page_above" => Some(Self::AddPageAbove),
            "add_page_below" => Some(Self::AddPageBelow),
            "duplicate_page" => Some(Self::DuplicatePage),
            "insert_spread" => Some(Self::InsertSpread),
            "delete_page" => Some(Self::DeletePage),
            "edit_text" => Some(Self::EditText),
            "table_row_above" => Some(Self::Table(crate::app::TableEdit::RowAbove)),
            "table_row_below" => Some(Self::Table(crate::app::TableEdit::RowBelow)),
            "table_col_left" => Some(Self::Table(crate::app::TableEdit::ColumnLeft)),
            "table_col_right" => Some(Self::Table(crate::app::TableEdit::ColumnRight)),
            "table_delete_row" => Some(Self::Table(crate::app::TableEdit::DeleteRow)),
            "table_delete_col" => Some(Self::Table(crate::app::TableEdit::DeleteColumn)),
            "table_delete" => Some(Self::Table(crate::app::TableEdit::DeleteTable)),
            "change_picture" => Some(Self::ChangePicture),
            "wrap_text" => Some(Self::WrapText),
            _ => None,
        }
    }
}

/// Tick View › Show Rulers / Grid / Guides to match the app.
/// Menu bar commands without a fixed [`MenuId`]: item id -> action.
pub fn command_action(id: &str) -> Option<crate::app::AppAction> {
    use crate::app::AppAction as A;
    Some(match id {
        "cmd.toggle_pages" => A::TogglePagesPane,
        "cmd.toggle_selection" => A::ToggleSelectionPane,
        "cmd.toggle_spread" => A::ToggleTwoPageSpread,
        "cmd.theme_light" => A::SetTheme(crate::theme::ThemeMode::Light),
        "cmd.theme_dark" => A::SetTheme(crate::theme::ThemeMode::Dark),
        "cmd.theme_system" => A::SetTheme(crate::theme::ThemeMode::System),
        "cmd.insert_table" => A::InsertTable,
        "cmd.insert_polygon" => A::InsertPolygon,
        "cmd.insert_star" => A::InsertStar,
        "cmd.insert_page_number" => A::InsertPageNumber,
        "cmd.insert_hyperlink" => A::InsertHyperlink,
        "cmd.bold" => A::TextBold,
        "cmd.italic" => A::TextItalic,
        "cmd.underline" => A::TextUnderline,
        "cmd.superscript" => A::TextSuperscript,
        "cmd.subscript" => A::TextSubscript,
        "cmd.grow_font" => A::GrowFont,
        "cmd.shrink_font" => A::ShrinkFont,
        "cmd.align_left" => A::TextAlignLeft,
        "cmd.align_center" => A::TextAlignCenter,
        "cmd.align_right" => A::TextAlignRight,
        "cmd.align_justify" => A::TextAlignJustify,
        "cmd.bullets" => A::ToggleBullets,
        "cmd.numbering" => A::ToggleNumbering,
        "cmd.indent_more" => A::ChangeIndent(true),
        "cmd.indent_less" => A::ChangeIndent(false),
        "cmd.text_fit" => A::ShowTextFit,
        "cmd.columns" => A::ShowTextColumns,
        "cmd.text_margins" => A::ShowTextBoxMargins,
        "cmd.wrap_text" => A::ShowTextWrap,
        "cmd.format_shape" => A::ShowFormatShape,
        "cmd.document_properties" => A::ShowDocumentProperties,
        "cmd.font" => A::ShowFontDialog,
        "cmd.new_from_template" => A::NewFromTemplate,
        "cmd.paragraph" => A::ShowParagraphDialog,
        "cmd.table_properties" => A::ShowTableProperties,
        "cmd.create_link" => A::CreateTextLink,
        "cmd.break_link" => A::BreakTextLink,
        "cmd.format_painter" => A::FormatPainter,
        "cmd.rotate_right" => A::RotateBy(90.0),
        "cmd.rotate_left" => A::RotateBy(-90.0),
        "cmd.flip_h" => A::Flip(true),
        "cmd.flip_v" => A::Flip(false),
        "cmd.obj_align_left" => A::AlignLeft,
        "cmd.obj_align_center" => A::AlignCenter,
        "cmd.obj_align_right" => A::AlignRight,
        "cmd.obj_align_top" => A::AlignTop,
        "cmd.obj_align_middle" => A::AlignMiddle,
        "cmd.obj_align_bottom" => A::AlignBottom,
        _ => return None,
    })
}

/// Tick the menu bar's check items (`id`, on).
pub fn sync_checks(menu: &Menu, checks: &[(&str, bool)]) {
    fn visit(items: Vec<MenuItemKind>, f: &dyn Fn(&CheckMenuItem)) {
        for item in items {
            match item {
                MenuItemKind::Submenu(sub) => visit(sub.items(), f),
                MenuItemKind::Check(check) => f(&check),
                _ => {}
            }
        }
    }
    visit(menu.items(), &|check| {
        if let Some((_, on)) = checks.iter().find(|(id, _)| *id == check.id().0.as_str()) {
            check.set_checked(*on);
        }
    });
}

pub fn sync_view_checks(menu: &Menu, rulers: bool, grid: bool, guides: bool) {
    fn visit(items: Vec<MenuItemKind>, f: &dyn Fn(&CheckMenuItem)) {
        for item in items {
            match item {
                MenuItemKind::Submenu(sub) => visit(sub.items(), f),
                MenuItemKind::Check(check) => f(&check),
                _ => {}
            }
        }
    }
    visit(menu.items(), &|check| match check.id().0.as_str() {
        "toggle_rulers" => check.set_checked(rulers),
        "toggle_grid" => check.set_checked(grid),
        "toggle_guides" => check.set_checked(guides),
        _ => {}
    });
}

/// Get the command modifier for the current platform.
fn cmd() -> Modifiers {
    if cfg!(target_os = "macos") {
        Modifiers::META
    } else {
        Modifiers::CONTROL
    }
}

/// Create the native menu bar.
pub fn create_menu() -> (Menu, mpsc::Receiver<MenuEvent>) {
    let menu = Menu::new();
    
    // Set up menu event channel
    let (tx, rx) = mpsc::channel();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = tx.send(event);
    }));
    
    // === App menu (macOS only, but muda handles this) ===
    let app_menu = Submenu::new("Repub", true);
    app_menu.append(&PredefinedMenuItem::about(None, None)).ok();
    app_menu.append(&PredefinedMenuItem::separator()).ok();
    app_menu.append(&PredefinedMenuItem::services(None)).ok();
    app_menu.append(&PredefinedMenuItem::separator()).ok();
    app_menu.append(&PredefinedMenuItem::hide(None)).ok();
    app_menu.append(&PredefinedMenuItem::hide_others(None)).ok();
    app_menu.append(&PredefinedMenuItem::show_all(None)).ok();
    app_menu.append(&PredefinedMenuItem::separator()).ok();
    // Our own Quit (not the predefined one, which terminates immediately) so
    // unsaved changes are offered for saving first
    let quit_item = MenuItem::with_id("quit", "Quit Repub", true, Some(Accelerator::new(cmd(), Code::KeyQ)));
    app_menu.append(&quit_item).ok();
    menu.append(&app_menu).ok();
    
    // === File menu ===
    let file_menu = Submenu::new("File", true);
    
    let new_item = MenuItem::with_id("new", "New...", true, Some(Accelerator::new(cmd(), Code::KeyN)));
    let open_item = MenuItem::with_id("open", "Open...", true, Some(Accelerator::new(cmd(), Code::KeyO)));
    let save_item = MenuItem::with_id("save", "Save", true, Some(Accelerator::new(cmd(), Code::KeyS)));
    let save_as_item = MenuItem::with_id("save_as", "Save As...", true, Some(Accelerator::new(cmd() | Modifiers::SHIFT, Code::KeyS)));
    // No shortcut: Cmd+E is Center (text alignment), as in Publisher
    let export_item = MenuItem::with_id("export", "Export...", true, None);
    let print_item = MenuItem::with_id("print", "Print...", true, Some(Accelerator::new(cmd(), Code::KeyP)));
    
    file_menu.append(&new_item).ok();
    file_menu.append(&MenuItem::with_id("cmd.new_from_template", "New from Template…", true, None)).ok();
    file_menu.append(&open_item).ok();
    file_menu.append(&PredefinedMenuItem::separator()).ok();
    file_menu.append(&save_item).ok();
    file_menu.append(&save_as_item).ok();
    file_menu.append(&PredefinedMenuItem::separator()).ok();
    file_menu.append(&MenuItem::with_id("cmd.document_properties", "Document Properties…", true, None)).ok();
    file_menu.append(&PredefinedMenuItem::separator()).ok();
    file_menu.append(&export_item).ok();
    file_menu.append(&print_item).ok();
    file_menu.append(&PredefinedMenuItem::separator()).ok();
    file_menu.append(&PredefinedMenuItem::close_window(None)).ok();
    menu.append(&file_menu).ok();
    
    // === Edit menu ===
    let edit_menu = Submenu::new("Edit", true);
    
    // Use custom menu items for undo/redo so we receive the events
    // (PredefinedMenuItem::undo/redo are handled by the system for text fields only)
    let undo_item = MenuItem::with_id("undo", "Undo", true, Some(Accelerator::new(cmd(), Code::KeyZ)));
    let redo_item = MenuItem::with_id("redo", "Redo", true, Some(Accelerator::new(cmd() | Modifiers::SHIFT, Code::KeyZ)));
    edit_menu.append(&undo_item).ok();
    edit_menu.append(&redo_item).ok();
    edit_menu.append(&PredefinedMenuItem::separator()).ok();
    
    // Use custom menu items for cut/copy/paste so we can handle them for canvas objects
    let cut_item = MenuItem::with_id("cut", "Cut", true, Some(Accelerator::new(cmd(), Code::KeyX)));
    let copy_item = MenuItem::with_id("copy", "Copy", true, Some(Accelerator::new(cmd(), Code::KeyC)));
    let paste_item = MenuItem::with_id("paste", "Paste", true, Some(Accelerator::new(cmd(), Code::KeyV)));
    edit_menu.append(&cut_item).ok();
    edit_menu.append(&copy_item).ok();
    edit_menu.append(&paste_item).ok();
    
    let duplicate_item = MenuItem::with_id("duplicate", "Duplicate", true, Some(Accelerator::new(cmd(), Code::KeyD)));
    edit_menu.append(&duplicate_item).ok();
    
    edit_menu.append(&PredefinedMenuItem::separator()).ok();
    let select_all_item = MenuItem::with_id("select_all", "Select All", true, Some(Accelerator::new(cmd(), Code::KeyA)));
    edit_menu.append(&select_all_item).ok();
    menu.append(&edit_menu).ok();
    
    // === View menu ===
    let view_menu = Submenu::new("View", true);
    
    let zoom_in = MenuItem::with_id("zoom_in", "Zoom In", true, Some(Accelerator::new(cmd(), Code::Equal)));
    let zoom_out = MenuItem::with_id("zoom_out", "Zoom Out", true, Some(Accelerator::new(cmd(), Code::Minus)));
    let zoom_actual = MenuItem::with_id("zoom_actual", "Actual Size", true, Some(Accelerator::new(cmd(), Code::Digit1)));
    let zoom_fit = MenuItem::with_id("zoom_fit", "Fit to Window", true, Some(Accelerator::new(cmd(), Code::Digit0)));
    
    view_menu.append(&zoom_in).ok();
    view_menu.append(&zoom_out).ok();
    view_menu.append(&zoom_actual).ok();
    view_menu.append(&zoom_fit).ok();
    view_menu.append(&PredefinedMenuItem::separator()).ok();
    
    // Ticks follow the app state (`sync_view_checks`)
    let toggle_rulers = CheckMenuItem::with_id("toggle_rulers", "Show Rulers", true, true, None);
    let toggle_grid = CheckMenuItem::with_id("toggle_grid", "Show Grid", true, false, None);
    let toggle_guides = CheckMenuItem::with_id("toggle_guides", "Show Guides", true, true, None);
    
    view_menu.append(&toggle_rulers).ok();
    view_menu.append(&toggle_grid).ok();
    view_menu.append(&toggle_guides).ok();
    view_menu.append(&PredefinedMenuItem::separator()).ok();
    for (id, label) in [("cmd.toggle_pages", "Page Navigation"), ("cmd.toggle_selection", "Selection Pane")] {
        view_menu.append(&CheckMenuItem::with_id(id, label, true, false, None)).ok();
    }
    view_menu.append(&PredefinedMenuItem::separator()).ok();
    view_menu.append(&CheckMenuItem::with_id("cmd.toggle_spread", "Two-Page Spread", true, false, None)).ok();
    view_menu.append(&PredefinedMenuItem::separator()).ok();
    view_menu.append(&PredefinedMenuItem::fullscreen(None)).ok();
    menu.append(&view_menu).ok();
    
    // === Insert menu ===
    let insert_menu = Submenu::new("Insert", true);
    
    // Publisher's names
    let insert_text = MenuItem::with_id("insert_text", "Text Box", true, None);
    let insert_image = MenuItem::with_id("insert_image", "Picture…", true, None);
    
    insert_menu.append(&insert_text).ok();
    insert_menu.append(&insert_image).ok();
    insert_menu.append(&MenuItem::with_id("cmd.insert_table", "Table", true, None)).ok();
    insert_menu.append(&PredefinedMenuItem::separator()).ok();
    
    // Shape submenu
    let shape_menu = Submenu::new("Shape", true);
    let insert_rect = MenuItem::with_id("insert_rectangle", "Rectangle", true, None);
    let insert_ellipse = MenuItem::with_id("insert_ellipse", "Oval", true, None);
    let insert_line = MenuItem::with_id("insert_line", "Line", true, None);
    
    shape_menu.append(&insert_rect).ok();
    shape_menu.append(&insert_ellipse).ok();
    shape_menu.append(&insert_line).ok();
    shape_menu.append(&MenuItem::with_id("cmd.insert_polygon", "Polygon", true, None)).ok();
    shape_menu.append(&MenuItem::with_id("cmd.insert_star", "Star", true, None)).ok();
    insert_menu.append(&shape_menu).ok();
    
    insert_menu.append(&PredefinedMenuItem::separator()).ok();
    let add_page = MenuItem::with_id("add_page", "New Page", true, None);
    insert_menu.append(&add_page).ok();
    insert_menu.append(&MenuItem::with_id("cmd.insert_page_number", "Page Number", true, None)).ok();
    insert_menu.append(&PredefinedMenuItem::separator()).ok();
    insert_menu.append(&MenuItem::with_id("cmd.insert_hyperlink", "Hyperlink…", true, Some(Accelerator::new(cmd(), Code::KeyK)))).ok();
    menu.append(&insert_menu).ok();

    // === Format menu === (shortcuts as on the keyboard; on macOS the menu takes the key)
    let format_menu = Submenu::new("Format", true);
    let item = |id: &str, label: &str, key: Option<(Modifiers, Code)>| MenuItem::with_id(id, label, true, key.map(|(m, c)| Accelerator::new(m, c)));
    let cmd_m = cmd();
    type Entry<'a> = Option<(&'a str, &'a str, Option<(Modifiers, Code)>)>;
    let entries: [Entry; 33] = [
        Some(("cmd.font", "Font…", None)),
        Some(("cmd.paragraph", "Paragraph…", None)),
        None,
        Some(("cmd.bold", "Bold", Some((cmd_m, Code::KeyB)))),
        Some(("cmd.italic", "Italic", Some((cmd_m, Code::KeyI)))),
        Some(("cmd.underline", "Underline", Some((cmd_m, Code::KeyU)))),
        Some(("cmd.superscript", "Superscript", None)),
        Some(("cmd.subscript", "Subscript", None)),
        Some(("cmd.grow_font", "Grow Font", Some((cmd_m | Modifiers::SHIFT, Code::Period)))),
        Some(("cmd.shrink_font", "Shrink Font", Some((cmd_m | Modifiers::SHIFT, Code::Comma)))),
        None,
        Some(("cmd.align_left", "Align Text Left", Some((cmd_m, Code::KeyL)))),
        Some(("cmd.align_center", "Center", Some((cmd_m, Code::KeyE)))),
        Some(("cmd.align_right", "Align Text Right", Some((cmd_m, Code::KeyR)))),
        Some(("cmd.align_justify", "Justify", Some((cmd_m, Code::KeyJ)))),
        None,
        Some(("cmd.bullets", "Bullets", Some((cmd_m | Modifiers::SHIFT, Code::KeyL)))),
        Some(("cmd.numbering", "Numbering", None)),
        Some(("cmd.indent_more", "Increase Indent", None)),
        Some(("cmd.indent_less", "Decrease Indent", None)),
        None,
        Some(("cmd.format_shape", "Format Shape…", None)),
        Some(("cmd.table_properties", "Table Properties…", None)),
        None,
        Some(("cmd.text_fit", "Text Fit…", None)),
        Some(("cmd.columns", "Columns…", None)),
        Some(("cmd.text_margins", "Text Box Margins…", None)),
        Some(("cmd.wrap_text", "Wrap Text…", None)),
        None,
        Some(("cmd.create_link", "Create Text Box Link", None)),
        Some(("cmd.break_link", "Break Text Box Link", None)),
        None,
        Some(("cmd.format_painter", "Format Painter", None)),
    ];
    for entry in entries {
        match entry {
            Some((id, label, key)) => format_menu.append(&item(id, label, key)).ok(),
            None => format_menu.append(&PredefinedMenuItem::separator()).ok(),
        };
    }
    menu.append(&format_menu).ok();
    
    // === Arrange menu ===
    let arrange_menu = Submenu::new("Arrange", true);
    
    let bring_front = MenuItem::with_id("bring_to_front", "Bring to Front", true, Some(Accelerator::new(cmd() | Modifiers::SHIFT, Code::BracketRight)));
    let bring_forward = MenuItem::with_id("bring_forward", "Bring Forward", true, Some(Accelerator::new(cmd(), Code::BracketRight)));
    let send_backward = MenuItem::with_id("send_backward", "Send Backward", true, Some(Accelerator::new(cmd(), Code::BracketLeft)));
    let send_back = MenuItem::with_id("send_to_back", "Send to Back", true, Some(Accelerator::new(cmd() | Modifiers::SHIFT, Code::BracketLeft)));
    
    arrange_menu.append(&bring_front).ok();
    arrange_menu.append(&bring_forward).ok();
    arrange_menu.append(&send_backward).ok();
    arrange_menu.append(&send_back).ok();
    arrange_menu.append(&PredefinedMenuItem::separator()).ok();
    arrange_menu.append(&MenuItem::with_id("group", "Group", true, Some(Accelerator::new(cmd(), Code::KeyG)))).ok();
    arrange_menu.append(&MenuItem::with_id("ungroup", "Ungroup", true, Some(Accelerator::new(cmd() | Modifiers::SHIFT, Code::KeyG)))).ok();
    arrange_menu.append(&PredefinedMenuItem::separator()).ok();
    let align_menu = Submenu::new("Align", true);
    for (id, label) in [
        ("cmd.obj_align_left", "Align Left"),
        ("cmd.obj_align_center", "Align Center"),
        ("cmd.obj_align_right", "Align Right"),
        ("cmd.obj_align_top", "Align Top"),
        ("cmd.obj_align_middle", "Align Middle"),
        ("cmd.obj_align_bottom", "Align Bottom"),
    ] {
        align_menu.append(&MenuItem::with_id(id, label, true, None)).ok();
    }
    arrange_menu.append(&align_menu).ok();
    let rotate_menu = Submenu::new("Rotate", true);
    for (id, label) in [("cmd.rotate_right", "Rotate Right 90°"), ("cmd.rotate_left", "Rotate Left 90°"), ("cmd.flip_v", "Flip Vertical"), ("cmd.flip_h", "Flip Horizontal")] {
        rotate_menu.append(&MenuItem::with_id(id, label, true, None)).ok();
    }
    arrange_menu.append(&rotate_menu).ok();
    menu.append(&arrange_menu).ok();
    
    // === Window menu ===
    let window_menu = Submenu::new("Window", true);
    window_menu.append(&PredefinedMenuItem::minimize(None)).ok();
    window_menu.append(&PredefinedMenuItem::maximize(None)).ok();
    window_menu.append(&PredefinedMenuItem::separator()).ok();
    window_menu.append(&PredefinedMenuItem::fullscreen(None)).ok();
    menu.append(&window_menu).ok();
    
    // === Help menu ===
    let help_menu = Submenu::new("Help", true);
    let about = MenuItem::with_id("about", "About Repub", true, None);
    help_menu.append(&about).ok();
    menu.append(&help_menu).ok();
    
    (menu, rx)
}

/// Initialize the native menu for the current platform.
#[cfg(target_os = "macos")]
pub fn init_menu_for_nsapp(menu: &Menu) {
    menu.init_for_nsapp();
}

#[cfg(not(target_os = "macos"))]
pub fn init_menu_for_nsapp(_menu: &Menu) {
    // On Windows/Linux, menu is attached per-window, not globally
}

/// Force the light (Aqua) appearance for the whole app, so the title bar,
/// menu bar menus and context menus match the light UI even when macOS is in dark mode.
#[cfg(target_os = "macos")]
pub fn force_light_appearance() {
    use objc2_app_kit::{NSApp, NSAppearance, NSAppearanceNameAqua};
    use objc2_foundation::MainThreadMarker;

    let Some(mtm) = MainThreadMarker::new() else { return };
    let app = NSApp(mtm);
    // SAFETY: NSAppearanceNameAqua is a valid, static AppKit appearance name.
    #[allow(unsafe_code)]
    let name = unsafe { NSAppearanceNameAqua };
    if let Some(appearance) = NSAppearance::appearanceNamed(name) {
        app.setAppearance(Some(&appearance));
    }
}

#[cfg(not(target_os = "macos"))]
pub fn force_light_appearance() {}


/// Context menu types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextMenuType {
    /// Context menu for empty canvas (no selection)
    Canvas,
    /// Context menu for selected objects
    Object,
    /// Context menu for page panel items
    Page,
}

/// What the context menus need to know to enable their items.
#[derive(Debug, Clone, Copy, Default)]
pub struct ContextMenuState {
    pub clipboard_empty: bool,
    pub selection_count: usize,
    pub page_count: usize,
    /// A group is among the selected objects.
    pub group_selected: bool,
    /// Exactly one text box is selected.
    pub single_text: bool,
    /// Exactly one picture is selected.
    pub single_picture: bool,
    /// Two-page spread view is on.
    pub two_page_spread: bool,
    /// Exactly one table is selected.
    pub single_table: bool,
    /// A table cell is being edited (row/column deletes act on it).
    pub in_cell: bool,
}

/// Context menu for empty canvas.
pub struct CanvasContextMenu {
    pub menu: Menu,
    paste: MenuItem,
    paste_in_place: MenuItem,
}

impl CanvasContextMenu {
    fn new() -> Self {
        let menu = Menu::new();
        let paste = MenuItem::with_id("paste", "Paste", true, Some(Accelerator::new(cmd(), Code::KeyV)));
        let paste_in_place = MenuItem::with_id("paste_in_place", "Paste in Place", true, Some(Accelerator::new(cmd() | Modifiers::SHIFT, Code::KeyV)));
        let select_all = MenuItem::with_id("select_all", "Select All", true, Some(Accelerator::new(cmd(), Code::KeyA)));
        let zoom_fit = MenuItem::with_id("zoom_fit", "Whole Page", true, Some(Accelerator::new(cmd(), Code::Digit0)));
        let zoom_actual = MenuItem::with_id("zoom_actual", "Actual Size", true, Some(Accelerator::new(cmd(), Code::Digit1)));
        menu.append(&paste).ok();
        menu.append(&paste_in_place).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
        menu.append(&select_all).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
        menu.append(&zoom_fit).ok();
        menu.append(&zoom_actual).ok();
        Self { menu, paste, paste_in_place }
    }

    fn update(&self, state: &ContextMenuState) {
        self.paste.set_enabled(!state.clipboard_empty);
        self.paste_in_place.set_enabled(!state.clipboard_empty);
    }
}

/// Context menu for selected objects.
pub struct ObjectContextMenu {
    pub menu: Menu,
    paste: MenuItem,
    group: MenuItem,
    ungroup: MenuItem,
    edit_text: MenuItem,
    change_picture: MenuItem,
    wrap_text: MenuItem,
    table_insert: Submenu,
    table_delete: Submenu,
    table_delete_row: MenuItem,
    table_delete_col: MenuItem,
    /// The table submenus are in the menu (only while a table is selected).
    table_shown: std::cell::Cell<bool>,
}

impl ObjectContextMenu {
    fn new() -> Self {
        let menu = Menu::new();
        let cut = MenuItem::with_id("cut", "Cut", true, Some(Accelerator::new(cmd(), Code::KeyX)));
        let copy = MenuItem::with_id("copy", "Copy", true, Some(Accelerator::new(cmd(), Code::KeyC)));
        let paste = MenuItem::with_id("paste", "Paste", true, Some(Accelerator::new(cmd(), Code::KeyV)));
        let duplicate = MenuItem::with_id("duplicate", "Duplicate", true, Some(Accelerator::new(cmd(), Code::KeyD)));
        let delete = MenuItem::with_id("delete", "Delete", true, None);
        // Object-specific commands first, as in Publisher
        let edit_text = MenuItem::with_id("edit_text", "Edit Text", true, None);
        let change_picture = MenuItem::with_id("change_picture", "Change Picture…", true, None);
        menu.append(&edit_text).ok();
        menu.append(&change_picture).ok();
        // Table: at the cell right-clicked
        let table_insert = Submenu::new("Insert", true);
        for (id, label) in [("table_row_above", "Insert Row Above"), ("table_row_below", "Insert Row Below"), ("table_col_left", "Insert Column Left"), ("table_col_right", "Insert Column Right")] {
            table_insert.append(&MenuItem::with_id(id, label, true, None)).ok();
        }
        let table_delete = Submenu::new("Delete", true);
        let table_delete_row = MenuItem::with_id("table_delete_row", "Delete Row", true, None);
        let table_delete_col = MenuItem::with_id("table_delete_col", "Delete Column", true, None);
        table_delete.append(&table_delete_row).ok();
        table_delete.append(&table_delete_col).ok();
        table_delete.append(&MenuItem::with_id("table_delete", "Delete Table", true, None)).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
        menu.append(&cut).ok();
        menu.append(&copy).ok();
        menu.append(&paste).ok();
        menu.append(&duplicate).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
        menu.append(&delete).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();

        let arrange_menu = Submenu::new("Arrange", true);
        arrange_menu.append(&MenuItem::with_id("bring_to_front", "Bring to Front", true, Some(Accelerator::new(cmd() | Modifiers::SHIFT, Code::BracketRight)))).ok();
        arrange_menu.append(&MenuItem::with_id("bring_forward", "Bring Forward", true, Some(Accelerator::new(cmd(), Code::BracketRight)))).ok();
        arrange_menu.append(&MenuItem::with_id("send_backward", "Send Backward", true, Some(Accelerator::new(cmd(), Code::BracketLeft)))).ok();
        arrange_menu.append(&MenuItem::with_id("send_to_back", "Send to Back", true, Some(Accelerator::new(cmd() | Modifiers::SHIFT, Code::BracketLeft)))).ok();
        menu.append(&arrange_menu).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
        let group = MenuItem::with_id("group", "Group", true, Some(Accelerator::new(cmd(), Code::KeyG)));
        let ungroup = MenuItem::with_id("ungroup", "Ungroup", true, Some(Accelerator::new(cmd() | Modifiers::SHIFT, Code::KeyG)));
        menu.append(&group).ok();
        menu.append(&ungroup).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
        let wrap_text = MenuItem::with_id("wrap_text", "Wrap Text…", true, None);
        menu.append(&wrap_text).ok();
        Self { menu, paste, group, ungroup, edit_text, change_picture, wrap_text, table_insert, table_delete, table_delete_row, table_delete_col, table_shown: std::cell::Cell::new(false) }
    }

    fn update(&self, state: &ContextMenuState) {
        self.paste.set_enabled(!state.clipboard_empty);
        self.group.set_enabled(state.selection_count > 1);
        self.ungroup.set_enabled(state.group_selected);
        self.edit_text.set_enabled(state.single_text);
        self.change_picture.set_enabled(state.single_picture);
        self.wrap_text.set_enabled(state.selection_count == 1);
        // Table submenus only for a table, after Edit Text / Change Picture
        if state.single_table != self.table_shown.get() {
            if state.single_table {
                self.menu.insert(&self.table_insert, 2).ok();
                self.menu.insert(&self.table_delete, 3).ok();
            } else {
                self.menu.remove(&self.table_insert).ok();
                self.menu.remove(&self.table_delete).ok();
            }
            self.table_shown.set(state.single_table);
        }
        self.table_delete_row.set_enabled(state.in_cell);
        self.table_delete_col.set_enabled(state.in_cell);
    }
}

/// Context menu for a page thumbnail.
pub struct PageContextMenu {
    pub menu: Menu,
    delete: MenuItem,
    insert_spread: MenuItem,
}

impl PageContextMenu {
    fn new() -> Self {
        let menu = Menu::new();
        let delete = MenuItem::with_id("delete_page", "Delete Page", true, None);
        menu.append(&MenuItem::with_id("add_page_above", "Insert Page Before", true, None)).ok();
        menu.append(&MenuItem::with_id("add_page_below", "Insert Page After", true, None)).ok();
        let insert_spread = MenuItem::with_id("insert_spread", "Insert Spread", true, None);
        menu.append(&insert_spread).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
        menu.append(&MenuItem::with_id("duplicate_page", "Duplicate Page", true, None)).ok();
        menu.append(&PredefinedMenuItem::separator()).ok();
        menu.append(&delete).ok();
        Self { menu, delete, insert_spread }
    }

    fn update(&self, state: &ContextMenuState) {
        self.delete.set_enabled(state.page_count > 1);
        self.insert_spread.set_enabled(state.two_page_spread);
    }
}

/// Show a native context menu at the given view position.
/// x, y are in egui coordinates (top-left origin, points).
/// Returns true if the menu was shown, false otherwise.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
pub fn show_context_menu(menu: &Menu, view_x: f64, view_y: f64) -> bool {
    use muda::dpi::LogicalPosition;
    
    // On macOS, we need to get the NSView from the key window
    // This uses objc2 to access the current key window's content view
    unsafe {
        use objc2_app_kit::NSApp;
        use objc2_foundation::MainThreadMarker;
        
        let mtm = match MainThreadMarker::new() {
            Some(m) => m,
            None => return false,
        };
        
        let app = NSApp(mtm);
        let key_window = match app.keyWindow() {
            Some(w) => w,
            None => return false,
        };
        
        let content_view = match key_window.contentView() {
            Some(v) => v,
            None => return false,
        };
        
        // Get the content view frame height
        let view_frame = content_view.frame();
        let view_height = view_frame.size.height;
        
        // egui Y=0 is at top, but muda expects coordinates that it will flip
        // using (view_height - y). Since egui already gives us top-origin coords,
        // we need to pre-flip so that muda's flip gives us the correct result.
        // 
        // If egui says y=100 (100 from top), we want the menu at 100 from top.
        // muda will do: view_height - y, so we pass: view_height - 100
        // Then muda does: view_height - (view_height - 100) = 100 from bottom
        // which in NSView coords (bottom-origin) is 100 from bottom = correct!
        let flipped_y = view_height - view_y;
        
        let position = LogicalPosition::new(view_x, flipped_y);
        
        // Get raw pointer to NSView - muda expects *const c_void
        let ns_view_ptr: *const objc2_app_kit::NSView = &*content_view;
        let ns_view = ns_view_ptr as *const std::ffi::c_void;
        
        menu.show_context_menu_for_nsview(ns_view, Some(position.into()));
    }
    
    true
}

#[cfg(not(target_os = "macos"))]
pub fn show_context_menu(_menu: &Menu, _screen_x: f64, _screen_y: f64) -> bool {
    // TODO: Implement for Windows/Linux
    false
}

/// Context menu state - holds the menus and tracks whether they're active
pub struct NativeContextMenus {
    pub canvas_menu: CanvasContextMenu,
    pub object_menu: ObjectContextMenu,
    pub page_menu: PageContextMenu,
    pending_context_menu: Option<(ContextMenuType, f64, f64)>,
}

impl NativeContextMenus {
    pub fn new() -> Self {
        Self {
            canvas_menu: CanvasContextMenu::new(),
            object_menu: ObjectContextMenu::new(),
            page_menu: PageContextMenu::new(),
            pending_context_menu: None,
        }
    }

    /// Enable or disable items to match the app state; call before showing.
    pub fn update(&self, state: &ContextMenuState) {
        self.canvas_menu.update(state);
        self.object_menu.update(state);
        self.page_menu.update(state);
    }
    
    /// Request to show a context menu at the next opportunity
    pub fn request_context_menu(&mut self, menu_type: ContextMenuType, screen_x: f64, screen_y: f64) {
        self.pending_context_menu = Some((menu_type, screen_x, screen_y));
    }
    
    /// Process pending context menu (call from main update loop)
    pub fn process_pending(&mut self) -> bool {
        if let Some((menu_type, x, y)) = self.pending_context_menu.take() {
            let menu = match menu_type {
                ContextMenuType::Canvas => &self.canvas_menu.menu,
                ContextMenuType::Object => &self.object_menu.menu,
                ContextMenuType::Page => &self.page_menu.menu,
            };
            return show_context_menu(menu, x, y);
        }
        false
    }
}

impl Default for NativeContextMenus {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::{command_action, MenuId};

    /// Every item id used in this file's menus does something: a fixed
    /// `MenuId` or a menu bar command.
    #[test]
    fn every_menu_item_maps_to_a_command() {
        let source = include_str!("native_menu.rs");
        let source = &source[..source.find("#[cfg(test)]\nmod tests").unwrap()];
        let mut ids: Vec<&str> = Vec::new();
        for (i, _) in source.match_indices("with_id(\"") {
            let rest = &source[i + 9..];
            ids.push(&rest[..rest.find('"').unwrap()]);
        }
        for (i, _) in source.match_indices("(\"cmd.") {
            let rest = &source[i + 2..];
            ids.push(&rest[..rest.find('"').unwrap()]);
        }
        assert!(ids.len() > 60, "found the ids: {}", ids.len());
        let dead: Vec<_> = ids
            .iter()
            .filter(|id| command_action(id).is_none() && MenuId::from_id(&muda::MenuId::new(**id)).is_none())
            .collect();
        assert!(dead.is_empty(), "menu items that do nothing: {dead:?}");
    }
}
