//! Context menus for right-click actions with platform-aware styling.

use egui::{RichText, Ui};

use crate::app::AppAction;
use crate::state::AppState;
use crate::theme::colors;

/// Check if running on macOS.
fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

/// Get the command key symbol for the current platform.
fn cmd_symbol() -> &'static str {
    if is_macos() { "⌘" } else { "Ctrl" }
}

/// Get the shift symbol for the current platform.
fn shift_symbol() -> &'static str {
    if is_macos() { "⇧" } else { "Shift" }
}

/// Format a keyboard shortcut for display.
fn shortcut(key: &str) -> String {
    if is_macos() {
        format!("{}{}", cmd_symbol(), key)
    } else {
        format!("{}+{}", cmd_symbol(), key)
    }
}

/// Format a keyboard shortcut with Shift.
fn shortcut_shift(key: &str) -> String {
    if is_macos() {
        format!("{}{}{}", shift_symbol(), cmd_symbol(), key)
    } else {
        format!("{}+{}+{}", cmd_symbol(), shift_symbol(), key)
    }
}

/// Create a styled context menu item with a shortcut.
fn context_item(ui: &mut Ui, label: &str, shortcut_text: &str, enabled: bool) -> bool {
    let text_color = if enabled { colors::text_primary() } else { colors::text_muted() };
    let shortcut_color = if enabled { colors::text_secondary() } else { colors::text_muted() };
    
    let response = ui.add_enabled_ui(enabled, |ui| {
        ui.set_min_width(160.0);
        ui.horizontal(|ui| {
            let response = ui.add(
                egui::SelectableLabel::new(false, RichText::new(label).color(text_color))
            );
            
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::Label::new(RichText::new(shortcut_text).color(shortcut_color).size(11.0)));
            });
            
            response
        }).inner
    });
    
    response.inner.clicked()
}

/// Create a simple context menu item without a shortcut.
fn context_item_simple(ui: &mut Ui, label: &str, enabled: bool) -> bool {
    let text_color = if enabled { colors::text_primary() } else { colors::text_muted() };
    
    ui.add_enabled_ui(enabled, |ui| {
        ui.set_min_width(160.0);
        ui.horizontal(|ui| {
            ui.add(egui::SelectableLabel::new(false, RichText::new(label).color(text_color)))
        }).inner
    }).inner.clicked()
}

/// Show context menu for canvas (empty space).
pub fn canvas_context_menu(ui: &mut Ui, state: &AppState) -> Option<AppAction> {
    let mut action = None;
    
    // Set menu width
    ui.set_min_width(180.0);
    
    if context_item(ui, "Paste", &shortcut("V"), true) {
        action = Some(AppAction::Paste);
        ui.close_menu();
    }
    if context_item(ui, "Paste in Place", &shortcut_shift("V"), true) {
        action = Some(AppAction::PasteInPlace);
        ui.close_menu();
    }
    
    ui.separator();
    
    if context_item(ui, "Select All", &shortcut("A"), state.has_document()) {
        action = Some(AppAction::SelectAll);
        ui.close_menu();
    }
    
    ui.separator();
    
    if context_item(ui, "Zoom to Fit", &shortcut("0"), true) {
        action = Some(AppAction::ZoomFit);
        ui.close_menu();
    }
    if context_item(ui, "Actual Size", &shortcut("1"), true) {
        action = Some(AppAction::ZoomActual);
        ui.close_menu();
    }
    
    ui.separator();
    
    if context_item_simple(ui, "Page Properties...", state.has_document()) {
        action = Some(AppAction::ShowPageProperties);
        ui.close_menu();
    }
    
    action
}

/// Show context menu for selected objects.
pub fn object_context_menu(ui: &mut Ui, state: &AppState) -> Option<AppAction> {
    let mut action = None;
    let has_selection = !state.selection.is_empty();
    let multi_selection = state.selection.len() > 1;
    
    // Set menu width
    ui.set_min_width(180.0);

    // Object-specific commands first, as in Publisher
    use repub_model::shape::ShapeKind;
    let selected = state.selected_objects();
    let editable_text = matches!(selected[..], [o] if matches!(o.kind, ShapeKind::TextFrame(_) | ShapeKind::Table(_)));
    if context_item(ui, "Edit Text", "", editable_text) {
        action = Some(AppAction::EditText);
        ui.close_menu();
    }
    if context_item(ui, "Change Picture…", "", matches!(selected[..], [o] if matches!(o.kind, ShapeKind::Picture(_)))) {
        action = Some(AppAction::ChangePicture);
        ui.close_menu();
    }
    if matches!(selected[..], [o] if matches!(o.kind, ShapeKind::Table(_))) {
        use crate::app::TableEdit;
        ui.menu_button("Insert", |ui| {
            for op in [TableEdit::RowAbove, TableEdit::RowBelow, TableEdit::ColumnLeft, TableEdit::ColumnRight] {
                if context_item(ui, op.label(), "", op.enabled(state)) {
                    action = Some(AppAction::TableEdit(op));
                    ui.close_menu();
                }
            }
        });
        ui.menu_button("Delete", |ui| {
            for op in [TableEdit::DeleteRow, TableEdit::DeleteColumn, TableEdit::DeleteTable] {
                if context_item(ui, op.label(), "", op.enabled(state)) {
                    action = Some(AppAction::TableEdit(op));
                    ui.close_menu();
                }
            }
        });
    }
    ui.separator();

    if context_item(ui, "Cut", &shortcut("X"), has_selection) {
        action = Some(AppAction::Cut);
        ui.close_menu();
    }
    if context_item(ui, "Copy", &shortcut("C"), has_selection) {
        action = Some(AppAction::Copy);
        ui.close_menu();
    }
    if context_item(ui, "Paste", &shortcut("V"), true) {
        action = Some(AppAction::Paste);
        ui.close_menu();
    }
    if context_item(ui, "Duplicate", &shortcut("D"), has_selection) {
        action = Some(AppAction::Duplicate);
        ui.close_menu();
    }
    
    ui.separator();
    
    let delete_key = if is_macos() { "⌫" } else { "Del" };
    if context_item(ui, "Delete", delete_key, has_selection) {
        action = Some(AppAction::Delete);
        ui.close_menu();
    }
    
    ui.separator();
    
    // Arrange submenu
    ui.menu_button("Arrange", |ui| {
        ui.set_min_width(160.0);
        
        if context_item(ui, "Bring to Front", &shortcut_shift("]"), has_selection) {
            action = Some(AppAction::BringToFront);
            ui.close_menu();
        }
        if context_item(ui, "Bring Forward", &shortcut("]"), has_selection) {
            action = Some(AppAction::BringForward);
            ui.close_menu();
        }
        if context_item(ui, "Send Backward", &shortcut("["), has_selection) {
            action = Some(AppAction::SendBackward);
            ui.close_menu();
        }
        if context_item(ui, "Send to Back", &shortcut_shift("["), has_selection) {
            action = Some(AppAction::SendToBack);
            ui.close_menu();
        }
    });
    ui.separator();
    let is_group = state.selected_objects().iter().any(|o| matches!(o.kind, repub_model::shape::ShapeKind::Group(_)));
    if context_item(ui, "Group", &shortcut("G"), multi_selection) {
        action = Some(AppAction::Group);
        ui.close_menu();
    }
    if context_item(ui, "Ungroup", &shortcut_shift("G"), is_group) {
        action = Some(AppAction::Ungroup);
        ui.close_menu();
    }
    ui.separator();
    if crate::text_format::has_target(state) {
        if context_item(ui, "Font…", "", true) {
            action = Some(AppAction::ShowFontDialog);
            ui.close_menu();
        }
        if context_item(ui, "Paragraph…", "", true) {
            action = Some(AppAction::ShowParagraphDialog);
            ui.close_menu();
        }
    }
    if matches!(state.selected_objects()[..], [o] if matches!(o.kind, repub_model::shape::ShapeKind::Table(_))) {
        if context_item(ui, "Table Properties…", "", true) {
            action = Some(AppAction::ShowTableProperties);
            ui.close_menu();
        }
    }
    if matches!(state.selected_objects()[..], [o] if matches!(o.kind, repub_model::shape::ShapeKind::TextEffect(_))) {
        if context_item(ui, "Edit WordArt…", "", true) {
            action = Some(AppAction::ShowWordArt);
            ui.close_menu();
        }
    }
    if context_item(ui, "Format Shape…", "", has_selection) {
        action = Some(AppAction::ShowFormatShape);
        ui.close_menu();
    }
    if context_item(ui, "Wrap Text…", "", state.selection.len() == 1) {
        action = Some(AppAction::ShowTextWrap);
        ui.close_menu();
    }

    action
}

/// Show context menu for pages panel.
pub fn page_context_menu(
    ui: &mut Ui,
    page_index: usize,
    total_pages: usize,
    selected_pages: &[usize],
) -> Option<PageContextAction> {
    let mut action = None;
    let multi_select = selected_pages.len() > 1;
    
    // Set menu width
    ui.set_min_width(160.0);
    
    if context_item_simple(ui, "Add Page", true) {
        action = Some(PageContextAction::AddPage);
        ui.close_menu();
    }
    
    if context_item_simple(ui, "Duplicate", true) {
        action = Some(PageContextAction::DuplicatePage(page_index));
        ui.close_menu();
    }
    
    ui.separator();
    
    // Can only delete if more than one page will remain
    let can_delete = if multi_select {
        total_pages > selected_pages.len()
    } else {
        total_pages > 1
    };
    
    let delete_label = if multi_select {
        format!("Delete {} Pages", selected_pages.len())
    } else {
        "Delete Page".to_string()
    };
    
    if context_item_simple(ui, &delete_label, can_delete) {
        action = Some(PageContextAction::DeletePage(page_index));
        ui.close_menu();
    }
    
    ui.separator();
    
    if context_item_simple(ui, "Page Properties...", !multi_select) {
        action = Some(PageContextAction::PageProperties(page_index));
        ui.close_menu();
    }
    
    action
}

/// Actions from page context menu.
#[derive(Debug, Clone)]
pub enum PageContextAction {
    AddPage,
    DuplicatePage(usize),
    DeletePage(usize),
    PageProperties(usize),
}
