//! Pages panel with drag & drop reordering, multi-select, and page previews.

use egui::{Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

use crate::canvas::Canvas;
use crate::state::AppState;
use crate::theme::{colors, fonts, panel_header};

/// Height of the page-number strip under each thumbnail.
const STRIP_HEIGHT: f32 = 14.0;

/// Thumbnail rows, top to bottom. One page per row, or in two-page spread view
/// the same pairing as the canvas: page 1 alone, then 2–3, 4–5, ...
fn page_rows(page_count: usize, spread: bool) -> Vec<Vec<usize>> {
    if !spread {
        return (0..page_count).map(|i| vec![i]).collect();
    }
    let mut rows = Vec::new();
    if page_count > 0 {
        rows.push(vec![0]);
    }
    let mut i = 1;
    while i < page_count {
        rows.push((i..(i + 2).min(page_count)).collect());
        i += 2;
    }
    rows
}

/// Pages panel showing page thumbnails with drag & drop support.
pub struct PagesPanel {
    /// Whether the panel has focus (for keyboard shortcuts)
    focused: bool,
    /// Set when the collapse chevron was clicked; the app hides the pane and clears this.
    pub collapse_requested: bool,
    /// Pending context menu request (macOS native menus)
    /// Stores (page_index, screen_x, screen_y)
    #[cfg(target_os = "macos")]
    pub pending_context_menu: Option<(usize, f64, f64)>,
}

impl Default for PagesPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl PagesPanel {
    /// Create a new pages panel.
    pub fn new() -> Self {
        Self {
            focused: false,
            collapse_requested: false,
            #[cfg(target_os = "macos")]
            pending_context_menu: None,
        }
    }

    /// Show the pages panel. Thumbnails are drawn with the canvas's renderer.
    pub fn show(&mut self, ui: &mut egui::Ui, state: &mut AppState, canvas: &mut Canvas) {
        // Publisher style: accent-colored "Pages" heading with a collapse control
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Pages")
                    .size(17.0)
                    .color(colors::accent()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                let (rect, response) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
                if response.hovered() {
                    ui.painter().rect_filled(rect, 2.0, colors::hover());
                }
                crate::icons::Icon::ChevronLeft.paint(
                    ui,
                    rect.shrink(4.0),
                    colors::text_secondary(),
                );
                if response
                    .on_hover_text("Collapse the Page Navigation pane")
                    .clicked()
                {
                    self.collapse_requested = true;
                }
            });
        });
        ui.add_space(2.0);

        let page_count = state.page_count();

        // Track panel focus for keyboard shortcuts
        let panel_response = ui.interact(
            ui.available_rect_before_wrap(),
            ui.id().with("pages_panel_focus"),
            Sense::click(),
        );
        if panel_response.clicked() {
            self.focused = true;
        }

        // Handle keyboard shortcuts when panel is focused
        if self.focused {
            // Handle delete key for selected pages
            if ui.input(|i| i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace))
            {
                if !state.selected_pages.is_empty() && page_count > 1 {
                    state.delete_selected_pages();
                }
            }

            // Handle Cmd/Ctrl+A for select all pages
            if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::A)) {
                state.select_all_pages();
            }
        }

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(8.0);

                // Track drop position
                let mut drop_at: Option<usize> = None;
                let mut perform_drop = false;
                let spread = state.two_page_spread && page_count > 1;
                let row_width = (ui.available_width() - 24.0).max(20.0);

                for row in page_rows(page_count, spread) {
                    // Horizontal drop indicator before the row
                    if state.page_drag.drop_target == Some(row[0]) {
                        Self::drop_indicator(ui);
                    }

                    // Facing pages share the row and meet in the middle; a page on its
                    // own (page 1, or a trailing even page) fills the whole row
                    let slot_width = row_width / row.len() as f32;

                    // Size each page by its own aspect ratio; the row is as tall as its tallest page
                    let heights: Vec<f32> = row
                        .iter()
                        .map(|&i| {
                            state
                                .document
                                .as_ref()
                                .and_then(|d| d.pages().get(i))
                                .map(|p| {
                                    slot_width * (p.size.height / p.size.width.max(1.0)) as f32
                                })
                                .unwrap_or(slot_width * 1.294)
                        })
                        .collect();
                    let row_height = heights.iter().copied().fold(0.0, f32::max) + STRIP_HEIGHT;

                    let row_rect = ui
                        .horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.allocate_exact_size(Vec2::new(row_width, row_height), Sense::hover())
                                .0
                        })
                        .inner;

                    let rects: Vec<(usize, Rect)> = row
                        .iter()
                        .zip(&heights)
                        .enumerate()
                        .map(|(slot, (&i, &h))| {
                            let x = row_rect.left() + slot as f32 * slot_width;
                            (
                                i,
                                Rect::from_min_size(
                                    Pos2::new(x, row_rect.top()),
                                    Vec2::new(slot_width, h + STRIP_HEIGHT),
                                ),
                            )
                        })
                        .collect();

                    // Highlight frames first, so a facing page never paints over its neighbour's frame
                    for &(i, full_rect) in &rects {
                        if state.visible_pages().contains(&i) || state.is_page_selected(i) {
                            let alpha = if state.page_drag.dragging_page == Some(i) {
                                0.5
                            } else {
                                1.0
                            };
                            ui.painter().rect_filled(
                                full_rect.expand(2.0),
                                Rounding::ZERO,
                                colors::accent_light().gamma_multiply(alpha),
                            );
                        }
                    }

                    for &(i, full_rect) in &rects {
                        let (dropped_here, target) = self
                            .page_thumbnail(ui, state, canvas, i, full_rect, spread, page_count);
                        if target.is_some() {
                            drop_at = target;
                        }
                        if dropped_here {
                            perform_drop = true;
                        }
                    }

                    // Vertical drop indicator at the spine, when dropping between facing pages
                    if let Some(target) = state.page_drag.drop_target {
                        if row.len() == 2 && target == row[1] {
                            let spine = rects[1].1.left();
                            let bar = Rect::from_center_size(
                                Pos2::new(spine, row_rect.center().y),
                                Vec2::new(4.0, row_rect.height()),
                            );
                            ui.painter()
                                .rect_filled(bar, Rounding::same(2.0), colors::accent());
                        }
                    }

                    ui.add_space(10.0);
                }

                // Show drop indicator after last page
                if state.page_drag.drop_target == Some(page_count) {
                    Self::drop_indicator(ui);
                }

                // Check for drop zone after last page
                let after_last_rect = ui.allocate_exact_size(
                    Vec2::new(ui.available_width() - 24.0, 20.0),
                    Sense::hover(),
                );
                if after_last_rect.1.hovered() && state.page_drag.dragging_page.is_some() {
                    drop_at = Some(page_count);
                }

                // Update drop target
                state.page_drag.drop_target = drop_at;

                // Check for modifier key (Alt for duplicate on drag)
                state.page_drag.duplicate_mode = ui.input(|i| i.modifiers.alt);

                // Handle drop
                if perform_drop
                    || (ui.input(|i| i.pointer.any_released())
                        && state.page_drag.dragging_page.is_some())
                {
                    if let (Some(from), Some(to)) =
                        (state.page_drag.dragging_page, state.page_drag.drop_target)
                    {
                        Self::handle_drop(state, from, to);
                    }
                    // Clear drag state
                    state.page_drag.dragging_page = None;
                    state.page_drag.drop_target = None;
                }

                // Pages are added from the right-click menu or Insert > Page, as in Publisher
                ui.add_space(8.0);
            });
    }

    fn drop_indicator(ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            let (rect, _) =
                ui.allocate_exact_size(Vec2::new(ui.available_width() - 24.0, 4.0), Sense::hover());
            ui.painter()
                .rect_filled(rect, Rounding::same(2.0), colors::accent());
        });
    }

    /// One page thumbnail in `full_rect` (page plus number strip). Returns whether
    /// a dragged page was dropped on it and, while dragging, the drop index the
    /// pointer indicates (before this page, or after it).
    #[allow(clippy::too_many_arguments)]
    fn page_thumbnail(
        &mut self,
        ui: &mut egui::Ui,
        state: &mut AppState,
        canvas: &mut Canvas,
        index: usize,
        full_rect: Rect,
        spread: bool,
        page_count: usize,
    ) -> (bool, Option<usize>) {
        let rect = Rect::from_min_max(
            full_rect.min,
            Pos2::new(full_rect.right(), full_rect.bottom() - STRIP_HEIGHT),
        );
        let strip_rect = Rect::from_min_max(rect.left_bottom(), full_rect.max);

        let is_selected = state.is_page_selected(index);
        let is_dragging = state.page_drag.dragging_page == Some(index);

        let mut dropped_here = false;
        let mut drop_target = None;

        // The clickable area covers the page and the number strip below it
        let response = ui.interact(
            full_rect,
            ui.id().with(("page_thumbnail", index)),
            Sense::click_and_drag(),
        );

        // Handle drag start
        if response.drag_started() {
            state.page_drag.dragging_page = Some(index);
        }

        // Drop target while another page is dragged over this one: the near
        // half means "before this page", the far half "after it". Facing pages
        // sit side by side, so a spread splits left/right instead of top/bottom.
        if let Some(dragging) = state.page_drag.dragging_page {
            if dragging != index && response.hovered() {
                if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                    let before = if spread {
                        pos.x < rect.center().x
                    } else {
                        pos.y < rect.center().y
                    };
                    drop_target = Some(if before { index } else { index + 1 });
                }
                if ui.input(|i| i.pointer.any_released()) {
                    dropped_here = true;
                }
            }
        }

        let highlighted = state.visible_pages().contains(&index) || is_selected;
        let border_color = if highlighted {
            colors::accent_light()
        } else if response.hovered() {
            colors::border_strong()
        } else {
            colors::page_thumb_border()
        };

        // Apply drag opacity
        let alpha = if is_dragging { 0.5 } else { 1.0 };

        // The page as it prints, drawn by the canvas renderer and clipped to the thumbnail.
        // Thumbnails scrolled out of view are skipped.
        if ui.is_rect_visible(rect) {
            if let Some(doc) = state.document.as_ref() {
                if let Some(page) = doc.pages().get(index) {
                    let master = page.master_page.and_then(|id| doc.master_page(id));
                    let painter = ui.painter_at(rect);
                    canvas.draw_page_thumbnail(
                        &painter,
                        rect,
                        page,
                        master,
                        index as u32 + 1,
                        page_count as u32,
                    );
                }
            }
            if is_dragging {
                ui.painter()
                    .rect_filled(rect, Rounding::ZERO, Color32::from_white_alpha(140));
            }
        }
        ui.painter().rect_stroke(
            rect,
            Rounding::ZERO,
            Stroke::new(1.0, border_color.gamma_multiply(alpha)),
        );

        // Page number centered in the strip under the thumbnail
        ui.painter().text(
            strip_rect.center(),
            egui::Align2::CENTER_CENTER,
            format!("{}", index + 1),
            egui::FontId::proportional(10.0),
            colors::text_primary().gamma_multiply(alpha),
        );
        let badge_size = Vec2::new(20.0, 16.0);

        // Master page indicator placeholder (top-right corner)
        // Shows "M" if this page has a master page applied
        if let Some(ref doc) = state.document {
            if let Some(page) = doc.pages().get(index) {
                if page.master_page.is_some() {
                    let master_badge_rect = egui::Rect::from_min_size(
                        rect.right_top() + Vec2::new(-24.0, 4.0),
                        badge_size,
                    );
                    ui.painter().rect_filled(
                        master_badge_rect,
                        Rounding::same(3.0),
                        Color32::from_rgba_unmultiplied(100, 100, 200, (180.0 * alpha) as u8),
                    );
                    ui.painter().text(
                        master_badge_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "M",
                        egui::FontId::proportional(10.0),
                        Color32::WHITE.gamma_multiply(alpha),
                    );
                }
            }
        }

        // Handle clicks with modifiers
        if response.clicked() {
            let modifiers = ui.input(|i| i.modifiers);

            if modifiers.shift {
                // Shift+click: select range
                state.select_page_range(index);
            } else if modifiers.ctrl || modifiers.command {
                // Ctrl/Cmd+click: toggle selection
                state.toggle_page_selection(index);
            } else {
                // Plain click: select this page and make it current
                state.set_page(index);
                state.clear_page_selection();
                state.select_page(index, false);
            }
        }

        // Double-click to go to page (clear selection)
        if response.double_clicked() {
            state.set_page(index);
            state.clear_page_selection();
        }

        // Context menu for selected pages
        // On macOS, use native context menus
        #[cfg(target_os = "macos")]
        if response.secondary_clicked() {
            // If right-clicking on unselected page, select it first
            if !is_selected {
                state.clear_page_selection();
                state.select_page(index, false);
            }
            if let Some(pos) = response.interact_pointer_pos() {
                self.pending_context_menu = Some((index, pos.x as f64, pos.y as f64));
            }
        }

        // On other platforms, use egui's built-in context menu
        #[cfg(not(target_os = "macos"))]
        response.context_menu(|ui| {
            // If right-clicking on unselected page, select it first
            if !is_selected {
                state.clear_page_selection();
                state.select_page(index, false);
            }

            let selected_count = state.selected_pages.len().max(1);

            if ui.button("Add Page Above").clicked() {
                state.insert_page_at(index);
                ui.close_menu();
            }
            if ui.button("Add Page Below").clicked() {
                state.insert_page_at(index + 1);
                ui.close_menu();
            }
            if state.two_page_spread && ui.button("Insert Spread").clicked() {
                state.insert_spread_after(index);
                ui.close_menu();
            }

            ui.separator();

            // Duplicate (objects included)
            let duplicate_label = if selected_count == 1 {
                "Duplicate Page".to_owned()
            } else {
                format!("Duplicate {} Pages", selected_count)
            };
            if ui.button(duplicate_label).clicked() {
                let pages = if selected_count == 1 {
                    vec![index]
                } else {
                    state.selected_pages.clone()
                };
                state.duplicate_pages(&pages);
                ui.close_menu();
            }

            ui.separator();

            // Delete
            let delete_enabled = page_count > selected_count;
            if selected_count == 1 {
                // Show confirmation warning if this is the only page
                let delete_label = if page_count == 1 {
                    "Delete Page (Last Page)"
                } else {
                    "Delete Page"
                };
                if ui
                    .add_enabled(delete_enabled, egui::Button::new(delete_label))
                    .clicked()
                {
                    state.delete_pages(&[index]);
                    ui.close_menu();
                }
            } else {
                if ui
                    .add_enabled(
                        delete_enabled,
                        egui::Button::new(format!("Delete {} Pages", selected_count)),
                    )
                    .clicked()
                {
                    state.delete_selected_pages();
                    ui.close_menu();
                }
            }

            ui.separator();
            if selected_count == 1 && ui.button("Page Properties…").clicked() {
                state.page_properties_requested = Some(index);
                ui.close_menu();
            }

            // Apply Master Page returns once master pages are drawn (todo/13-usability.md 13.F5)
        });

        (dropped_here, drop_target)
    }

    /// Drop the dragged page `from` before position `to` (0..=page count).
    /// Alt-drag drops a copy instead. Both are undoable.
    fn handle_drop(state: &mut AppState, from: usize, to: usize) {
        let count = state.page_count();
        if from >= count || to > count {
            return;
        }
        if state.page_drag.duplicate_mode {
            state.record_undo("Duplicate Page");
            if let Some(doc) = state.document.as_mut() {
                // The copy lands right after the original; then move it to the drop point
                if let Some(copy) = doc.duplicate_page(from) {
                    doc.move_page(copy, to);
                }
            }
            state.current_page = to;
        } else {
            if from == to || from + 1 == to {
                return;
            }
            state.record_undo("Move Page");
            // `to` counts the dragged page, which is removed first
            let adjusted_to = if from < to { to - 1 } else { to };
            if let Some(doc) = state.document.as_mut() {
                doc.move_page(from, adjusted_to);
            }
            if state.current_page == from {
                state.current_page = adjusted_to;
            } else if from < state.current_page && adjusted_to >= state.current_page {
                state.current_page -= 1;
            } else if from > state.current_page && adjusted_to <= state.current_page {
                state.current_page += 1;
            }
        }
        state.selection.clear();
        state.clear_page_selection();
        state.mark_dirty();
    }

    /// Apply a master page to all selected pages.
    fn apply_master_to_selected(
        state: &mut AppState,
        master_id: Option<repub_model::MasterPageId>,
    ) {
        let indices: Vec<usize> = if state.selected_pages.is_empty() {
            vec![state.current_page]
        } else {
            state.selected_pages.clone()
        };

        if let Some(ref mut doc) = state.document {
            for index in indices {
                if let Some(page) = doc.pages_mut().get_mut(index) {
                    page.master_page = master_id;
                }
            }
            state.mark_dirty();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::page_rows;

    #[test]
    fn single_page_view_lists_one_page_per_row() {
        assert_eq!(page_rows(3, false), vec![vec![0], vec![1], vec![2]]);
    }

    #[test]
    fn spread_rows_pair_like_the_canvas() {
        assert_eq!(page_rows(1, true), vec![vec![0]]);
        assert_eq!(page_rows(3, true), vec![vec![0], vec![1, 2]]);
        assert_eq!(page_rows(4, true), vec![vec![0], vec![1, 2], vec![3]]);
        assert_eq!(page_rows(5, true), vec![vec![0], vec![1, 2], vec![3, 4]]);
    }
}
