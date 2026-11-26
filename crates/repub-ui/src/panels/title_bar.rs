//! Publisher-style title row for macOS.
//!
//! The native title bar is transparent with the content view extended under it,
//! so this row sits beside the native traffic lights. It holds the Quick Access
//! Toolbar (Save, Undo, Redo) and the centered document title, and behaves like a
//! title bar: drag to move the window, double-click to zoom.
//!
//! In full screen the traffic lights are hidden, so the Quick Access Toolbar
//! moves to the far left.

use egui::{Align2, Color32, Rect, Sense, Vec2};

use crate::app::AppAction;
use crate::icons::Icon;
use crate::state::AppState;
use crate::theme::{colors, fonts, metrics, shortcut, super_tooltip};

/// Inset of the Quick Access Toolbar when the traffic lights are hidden.
pub const FULLSCREEN_INSET: f32 = 8.0;

/// Left inset of the Quick Access Toolbar: clear of the traffic lights when
/// windowed, at the far left in full screen.
pub fn toolbar_inset(fullscreen: bool) -> f32 {
    if fullscreen { FULLSCREEN_INSET } else { metrics::TRAFFIC_LIGHTS_WIDTH }
}

/// Show the title row and return the Quick Access Toolbar actions clicked this frame.
pub fn show(ui: &mut egui::Ui, state: &AppState, toolbar_inset: f32) -> Vec<AppAction> {
    let mut actions = Vec::new();
    let rect = ui.max_rect();

    // Whole-row drag/double-click behaves like a native title bar. Registered first,
    // so the QAT buttons added afterwards take priority over it.
    let drag = ui.interact(rect, ui.id().with("title_drag"), Sense::click_and_drag());
    if drag.drag_started() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if drag.double_clicked() {
        let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }

    // Centered document title
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        state.window_title(),
        fonts::body(),
        colors::text_primary(),
    );

    // Quick Access Toolbar after the traffic lights (or at the far left in full screen)
    let mut x = rect.left() + toolbar_inset;
    let has_doc = state.has_document();
    let buttons = [
        (Icon::Save, "Save", Some(shortcut("S")), "Save this publication.", AppAction::SaveDocument, has_doc),
        (Icon::ArrowUndo, "Undo", Some(shortcut("Z")), "Undo the last action.", AppAction::Undo, state.history.can_undo()),
        (Icon::ArrowRedo, "Redo", Some(shortcut("Shift+Z")), "Redo the last undone action.", AppAction::Redo, state.history.can_redo()),
    ];
    for (icon, title, keys, description, action, enabled) in buttons {
        let button_rect = Rect::from_center_size(egui::pos2(x + 12.0, rect.center().y), Vec2::splat(24.0));
        let response = ui.interact(button_rect, ui.id().with(title), if enabled { Sense::click() } else { Sense::hover() });
        if enabled && response.hovered() {
            ui.painter().rect_filled(button_rect, metrics::ROUNDING, colors::hover());
        }
        let tint = if enabled { colors::text_primary() } else { colors::text_disabled() };
        icon.paint(ui, Rect::from_center_size(button_rect.center(), Vec2::splat(16.0)), tint);
        let response = super_tooltip(response, title, keys.as_deref(), description);
        if enabled && response.clicked() {
            actions.push(action);
        }
        x += 26.0;
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toolbar_clears_the_traffic_lights_when_windowed() {
        assert_eq!(toolbar_inset(false), metrics::TRAFFIC_LIGHTS_WIDTH);
    }

    #[test]
    fn toolbar_sits_at_the_far_left_in_full_screen() {
        assert_eq!(toolbar_inset(true), FULLSCREEN_INSET);
    }
}
