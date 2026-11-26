//! Menu bar and menus with platform-aware keyboard shortcuts.

use crate::app::AppAction;
use crate::state::AppState;
use crate::theme::colors;
use egui::RichText;

/// Check if running on macOS.
fn is_macos() -> bool {
    cfg!(target_os = "macos")
}

/// Get the command key symbol for the current platform.
fn cmd_symbol() -> &'static str {
    if is_macos() {
        "⌘"
    } else {
        "Ctrl"
    }
}

/// Get the shift symbol for the current platform.
fn shift_symbol() -> &'static str {
    if is_macos() {
        "⇧"
    } else {
        "Shift"
    }
}

/// Get the option/alt symbol for the current platform.
#[allow(dead_code)]
fn option_symbol() -> &'static str {
    if is_macos() {
        "⌥"
    } else {
        "Alt"
    }
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

/// Create a styled menu item with a shortcut.
/// Returns true if clicked.
fn menu_item(ui: &mut egui::Ui, label: &str, shortcut_text: &str, enabled: bool) -> bool {
    let text_color = if enabled {
        colors::text_primary()
    } else {
        colors::text_muted()
    };
    let shortcut_color = if enabled {
        colors::text_secondary()
    } else {
        colors::text_muted()
    };

    let response = ui.add_enabled_ui(enabled, |ui| {
        ui.set_min_width(180.0);
        ui.horizontal(|ui| {
            // Use selectable label for better hover styling
            let response = ui.add(egui::SelectableLabel::new(
                false,
                RichText::new(label).color(text_color),
            ));

            // Right-align the shortcut
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add(egui::Label::new(
                    RichText::new(shortcut_text)
                        .color(shortcut_color)
                        .size(12.0),
                ));
            });

            response
        })
        .inner
    });

    response.inner.clicked()
}

/// Create a simple menu item without a shortcut.
fn menu_item_simple(ui: &mut egui::Ui, label: &str, enabled: bool) -> bool {
    let text_color = if enabled {
        colors::text_primary()
    } else {
        colors::text_muted()
    };

    ui.add_enabled_ui(enabled, |ui| {
        ui.set_min_width(180.0);
        ui.horizontal(|ui| {
            ui.add(egui::SelectableLabel::new(
                false,
                RichText::new(label).color(text_color),
            ))
        })
        .inner
    })
    .inner
    .clicked()
}

/// Create a checkable menu item.
fn menu_item_check(ui: &mut egui::Ui, label: &str, checked: bool, enabled: bool) -> bool {
    let text_color = if enabled {
        colors::text_primary()
    } else {
        colors::text_muted()
    };

    ui.add_enabled_ui(enabled, |ui| {
        ui.set_min_width(180.0);
        ui.horizontal(|ui| {
            // Checkmark column: SVG icon when checked, same-width gap otherwise
            let (rect, _) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
            if checked {
                crate::icons::Icon::Checkmark.paint(ui, rect, text_color);
            }
            ui.add(egui::SelectableLabel::new(
                false,
                RichText::new(label).color(text_color),
            ))
        })
        .inner
    })
    .inner
    .clicked()
}

/// Show the menu bar.
pub fn show_menu_bar(ui: &mut egui::Ui, state: &AppState) -> Vec<AppAction> {
    let mut actions = Vec::new();

    // Apply menu styling
    let spacing = ui.spacing_mut();
    spacing.menu_margin = egui::Margin::symmetric(8.0, 4.0);

    egui::menu::bar(ui, |ui| {
        // === File Menu ===
        ui.menu_button("File", |ui| {
            ui.set_min_width(200.0);

            if menu_item(ui, "New...", &shortcut("N"), true) {
                actions.push(AppAction::NewDocument);
                ui.close_menu();
            }
            if menu_item(ui, "New from Template...", "", true) {
                actions.push(AppAction::NewFromTemplate);
                ui.close_menu();
            }
            if menu_item(ui, "Open...", &shortcut("O"), true) {
                actions.push(AppAction::OpenDocument);
                ui.close_menu();
            }

            ui.separator();

            if menu_item(ui, "Save", &shortcut("S"), state.has_document()) {
                actions.push(AppAction::SaveDocument);
                ui.close_menu();
            }
            if menu_item(ui, "Save As...", &shortcut_shift("S"), state.has_document()) {
                actions.push(AppAction::SaveDocumentAs);
                ui.close_menu();
            }

            ui.separator();

            if menu_item(ui, "Document Properties...", "", state.has_document()) {
                actions.push(AppAction::ShowDocumentProperties);
                ui.close_menu();
            }

            ui.separator();

            if menu_item(ui, "Export...", "", state.has_document()) {
                actions.push(AppAction::Export);
                ui.close_menu();
            }
            if menu_item(ui, "Print...", &shortcut("P"), state.has_document()) {
                actions.push(AppAction::Print);
                ui.close_menu();
            }

            ui.separator();

            let quit_shortcut = if cfg!(target_os = "macos") {
                "⌘Q"
            } else {
                "Alt+F4"
            };
            if menu_item(ui, "Quit", quit_shortcut, true) {
                actions.push(AppAction::Quit);
                ui.close_menu();
            }
        });

        // === Edit Menu ===
        ui.menu_button("Edit", |ui| {
            ui.set_min_width(200.0);

            if menu_item(ui, "Undo", &shortcut("Z"), state.history.can_undo()) {
                actions.push(AppAction::Undo);
                ui.close_menu();
            }
            let redo_shortcut = if cfg!(target_os = "macos") {
                "⇧⌘Z"
            } else {
                "Ctrl+Y"
            };
            if menu_item(ui, "Redo", redo_shortcut, state.history.can_redo()) {
                actions.push(AppAction::Redo);
                ui.close_menu();
            }

            ui.separator();

            let has_sel = !state.selection.is_empty();
            if menu_item(ui, "Cut", &shortcut("X"), has_sel) {
                actions.push(AppAction::Cut);
                ui.close_menu();
            }
            if menu_item(ui, "Copy", &shortcut("C"), has_sel) {
                actions.push(AppAction::Copy);
                ui.close_menu();
            }
            if menu_item(ui, "Paste", &shortcut("V"), true) {
                actions.push(AppAction::Paste);
                ui.close_menu();
            }
            if menu_item(ui, "Paste in Place", &shortcut_shift("V"), true) {
                actions.push(AppAction::PasteInPlace);
                ui.close_menu();
            }
            if menu_item(ui, "Duplicate", &shortcut("D"), has_sel) {
                actions.push(AppAction::Duplicate);
                ui.close_menu();
            }

            ui.separator();

            let delete_key = if cfg!(target_os = "macos") {
                "⌫"
            } else {
                "Del"
            };
            if menu_item(ui, "Delete", delete_key, has_sel) {
                actions.push(AppAction::Delete);
                ui.close_menu();
            }

            ui.separator();

            if menu_item(ui, "Select All", &shortcut("A"), state.has_document()) {
                actions.push(AppAction::SelectAll);
                ui.close_menu();
            }
        });

        // === View Menu ===
        ui.menu_button("View", |ui| {
            ui.set_min_width(200.0);

            if menu_item(ui, "Zoom In", &shortcut("+"), true) {
                actions.push(AppAction::ZoomIn);
                ui.close_menu();
            }
            if menu_item(ui, "Zoom Out", &shortcut("-"), true) {
                actions.push(AppAction::ZoomOut);
                ui.close_menu();
            }
            if menu_item(ui, "Actual Size", &shortcut("1"), true) {
                actions.push(AppAction::ZoomActual);
                ui.close_menu();
            }
            if menu_item(ui, "Fit to Window", &shortcut("0"), true) {
                actions.push(AppAction::ZoomFit);
                ui.close_menu();
            }

            ui.separator();

            if menu_item_check(ui, "Show Rulers", state.show_rulers, true) {
                actions.push(AppAction::ToggleRulers);
                ui.close_menu();
            }
            if menu_item_check(ui, "Show Guides", state.show_guides, true) {
                actions.push(AppAction::ToggleGuides);
                ui.close_menu();
            }
            if menu_item_check(ui, "Snap to Guides", state.snap_to_guides, true) {
                actions.push(AppAction::ToggleSnapToGuides);
                ui.close_menu();
            }
            if menu_item_simple(
                ui,
                "Clear All Guides",
                !state.guides().is_empty() && !state.guides_locked,
            ) {
                actions.push(AppAction::ClearGuides);
                ui.close_menu();
            }

            ui.separator();

            if menu_item_check(ui, "Show Grid", state.show_grid, true) {
                actions.push(AppAction::ToggleGrid);
                ui.close_menu();
            }
            if menu_item_check(ui, "Snap to Grid", state.snap_to_grid, true) {
                actions.push(AppAction::ToggleSnapToGrid);
                ui.close_menu();
            }
        });

        // === Arrange Menu ===
        ui.menu_button("Arrange", |ui| {
            ui.set_min_width(200.0);

            let has_sel = !state.selection.is_empty();

            if menu_item(ui, "Bring to Front", &shortcut_shift("]"), has_sel) {
                actions.push(AppAction::BringToFront);
                ui.close_menu();
            }
            if menu_item(ui, "Bring Forward", &shortcut("]"), has_sel) {
                actions.push(AppAction::BringForward);
                ui.close_menu();
            }
            if menu_item(ui, "Send Backward", &shortcut("["), has_sel) {
                actions.push(AppAction::SendBackward);
                ui.close_menu();
            }
            if menu_item(ui, "Send to Back", &shortcut_shift("["), has_sel) {
                actions.push(AppAction::SendToBack);
                ui.close_menu();
            }
            ui.separator();
            let is_group = state
                .selected_objects()
                .iter()
                .any(|o| matches!(o.kind, repub_model::shape::ShapeKind::Group(_)));
            if menu_item(ui, "Group", &shortcut("G"), state.selection.len() > 1) {
                actions.push(AppAction::Group);
                ui.close_menu();
            }
            if menu_item(ui, "Ungroup", &shortcut_shift("G"), is_group) {
                actions.push(AppAction::Ungroup);
                ui.close_menu();
            }
        });

        // === Insert Menu ===
        ui.menu_button("Insert", |ui| {
            ui.set_min_width(180.0);

            let has_doc = state.has_document();

            if menu_item_simple(ui, "Text Frame", has_doc) {
                actions.push(AppAction::InsertText);
                ui.close_menu();
            }
            if menu_item_simple(ui, "Image...", has_doc) {
                actions.push(AppAction::InsertImage);
                ui.close_menu();
            }

            ui.separator();

            ui.menu_button("Shape", |ui| {
                ui.set_min_width(140.0);
                if menu_item_simple(ui, "Rectangle", has_doc) {
                    actions.push(AppAction::InsertRectangle);
                    ui.close_menu();
                }
                if menu_item_simple(ui, "Ellipse", has_doc) {
                    actions.push(AppAction::InsertEllipse);
                    ui.close_menu();
                }
                if menu_item_simple(ui, "Line", has_doc) {
                    actions.push(AppAction::InsertLine);
                    ui.close_menu();
                }
                if menu_item_simple(ui, "Polygon", has_doc) {
                    actions.push(AppAction::InsertPolygon);
                    ui.close_menu();
                }
                if menu_item_simple(ui, "Star", has_doc) {
                    actions.push(AppAction::InsertStar);
                    ui.close_menu();
                }
            });

            ui.separator();

            if menu_item_simple(ui, "New Page", has_doc) {
                actions.push(AppAction::AddPage);
                ui.close_menu();
            }
        });

        // === Help Menu ===
        ui.menu_button("Help", |ui| {
            ui.set_min_width(160.0);

            if menu_item_simple(ui, "About Repub", true) {
                actions.push(AppAction::ShowAbout);
                ui.close_menu();
            }
        });
    });

    actions
}
