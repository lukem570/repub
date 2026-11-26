//! Insert Hyperlink dialog.

use egui::TextEdit;
use repub_color::Color;
use repub_model::ObjectId;
use repub_model::shape::{Shape, ShapeKind};
use repub_model::styles::{CharacterStyle, UnderlineStyle};
use repub_model::text::Hyperlink;

use crate::state::AppState;
use crate::text_edit::{self, TextPos};
use crate::theme::{colors, fonts, section_header};

/// Link colour used by Publisher for hyperlinks.
const LINK_COLOR: Color = Color::rgb(0.02, 0.35, 0.72);

/// Insert Hyperlink dialog state.
#[derive(Debug, Clone, Default)]
pub struct HyperlinkDialog {
    /// Whether the dialog is open
    pub open: bool,
    /// Text to display
    pub text: String,
    /// Destination address
    pub address: String,
    /// Frame the link is being inserted into (captured on open)
    pub object_id: Option<ObjectId>,
    /// The link goes to a page of this publication (the address is its number)
    pub internal: bool,
    /// The existing link being changed (its start and end), when the caret is in one
    editing: Option<(TextPos, TextPos)>,
}

impl HyperlinkDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog, prefilling the display text from the current text selection.
    pub fn open_for_state(&mut self, state: &AppState) {
        self.open = true;
        self.address.clear();
        self.object_id = None;
        self.internal = false;
        self.editing = None;
        if let Some(edit) = state.text_edit.as_ref() {
            let selected = text_edit::selected_text(state);
            self.text = selected;
            self.object_id = Some(edit.frame);
            // A caret inside a link (with nothing selected) edits that link
            if !edit.has_selection() {
                let found = text_edit::texts_in(state, edit.frame, edit.cell).first().and_then(|t| text_edit::link_around(t, edit.caret));
                if let Some((start, end, link)) = found {
                    self.text = link.text;
                    self.address = link.target;
                    self.internal = !link.external;
                    self.editing = Some((start, end));
                }
            }
        } else {
            self.text.clear();
            self.object_id = state.selection.iter().copied().find(|id| {
                state
                    .current_page()
                    .and_then(|p| p.get_shape(*id))
                    .is_some_and(|o| matches!(o.kind, ShapeKind::TextFrame(_)))
            });
        }
    }

    /// Show the dialog. Returns true when a link was inserted.
    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) -> bool {
        if !self.open {
            return false;
        }
        let mut inserted = false;
        let mut close = false;

        let title = if self.editing.is_some() { "Edit Hyperlink" } else { "Insert Hyperlink" };
        let mut remove = false;
        egui::Window::new(title)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_min_width(380.0);
                section_header(ui, "Link");
                ui.add_space(4.0);

                egui::Grid::new("hyperlink_grid")
                    .num_columns(2)
                    .spacing([8.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(egui::RichText::new("Text to display").font(fonts::body()));
                        ui.add(TextEdit::singleline(&mut self.text).desired_width(240.0));
                        ui.end_row();

                        ui.label(egui::RichText::new("Link to").font(fonts::body()));
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.internal, false, "Web or e-mail address");
                            ui.selectable_value(&mut self.internal, true, "Page in this publication");
                        });
                        ui.end_row();

                        ui.label(egui::RichText::new(if self.internal { "Page number" } else { "Address" }).font(fonts::body()));
                        ui.add(
                            TextEdit::singleline(&mut self.address)
                                .desired_width(240.0)
                                .hint_text(if self.internal { "3" } else { "https://example.com" }),
                        );
                        ui.end_row();
                    });

                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Existing text selections are replaced by the link.")
                        .font(fonts::caption())
                        .color(colors::text_secondary()),
                );

                ui.add_space(12.0);
                ui.separator();
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Cancel").clicked() {
                        close = true;
                    }
                    if self.editing.is_some() && ui.button("Remove Link").clicked() {
                        remove = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add(egui::Button::new("OK").fill(colors::accent())).clicked() {
                            inserted = self.apply(state);
                            close = true;
                        }
                    });
                });
            });

        if remove {
            inserted = self.remove(state);
            close = true;
        }
        if close {
            self.open = false;
        }
        inserted
    }

    /// Turn the link being edited back into plain text.
    fn remove(&mut self, state: &mut AppState) -> bool {
        let (Some((start, end)), Some(edit)) = (self.editing, state.text_edit.clone()) else { return false };
        state.record_undo("Remove Hyperlink");
        let text = self.text.clone();
        let mut after = None;
        text_edit::for_each_text_mut(state, edit.frame, edit.cell, |content| {
            let at = text_edit::delete_range(content, start, end);
            after = Some(text_edit::insert_text(content, at, &text));
        });
        let Some(after) = after else { return false };
        if let Some(edit) = state.text_edit.as_mut() {
            edit.caret = after;
            edit.anchor = after;
        }
        state.mark_dirty();
        true
    }

    /// Insert the hyperlink described by the dialog fields.
    pub fn insert(&mut self, state: &mut AppState) -> bool {
        self.apply(state)
    }

    fn apply(&mut self, state: &mut AppState) -> bool {
        let address = self.address.trim();
        if address.is_empty() {
            return false;
        }
        let label = if self.text.trim().is_empty() { address } else { self.text.trim() };
        let mut link = if self.internal { Hyperlink::internal(address, label) } else { Hyperlink::external(address, label) };
        link.style = CharacterStyle {
            color: LINK_COLOR,
            underline: UnderlineStyle::Single,
            ..CharacterStyle::default()
        };

        state.record_undo("Insert Hyperlink");

        // Editing: replace the selected range (or the link being edited), then move the caret past the link.
        if let Some(edit) = state.text_edit.clone() {
            let (start, end) = self.editing.unwrap_or_else(|| edit.range());
            let mut after = None;
            text_edit::for_each_text_mut(state, edit.frame, edit.cell, |content| {
                let at = if start != end { text_edit::delete_range(content, start, end) } else { start };
                after = Some(text_edit::insert_link(content, at, link.clone()));
            });
            let Some(after) = after else { return false };
            if let Some(edit) = state.text_edit.as_mut() {
                edit.caret = after;
                edit.anchor = after;
            }
            state.mark_dirty();
            return true;
        }

        // A selected text frame: append the link at the end.
        if let Some(&id) = self.object_id.as_ref() {
            let mut placed = false;
            if let Some(page) = state.current_page_mut() {
                if let Some(Shape { kind: ShapeKind::TextFrame(tf), .. }) = page.get_shape_mut(id) {
                    if tf.content.paragraphs.is_empty() {
                        tf.content.paragraphs.push(repub_model::text::Paragraph::new());
                    }
                    let pos = text_edit::end_pos(&tf.content);
                    text_edit::insert_link(&mut tf.content, pos, link.clone());
                    placed = true;
                }
            }
            if placed {
                state.mark_dirty();
                return true;
            }
        }

        // Nothing selected: create a small frame near the page centre.
        let Some(size) = state.current_page().map(|p| p.size) else { return false };
        let bounds = repub_geometry::Rect::new(
            size.width / 2.0 - 90.0,
            size.height / 2.0 - 12.0,
            180.0,
            24.0,
        );
        let mut shape = Shape::text_frame(bounds);
        let ShapeKind::TextFrame(tf) = &mut shape.kind else { unreachable!() };
        text_edit::insert_link(&mut tf.content, TextPos::new(0, 0), link);
        let id = shape.common.id;
        {
            if let Some(page) = state.current_page_mut() {
                page.add_shape(shape);
            }
        }
        state.selection = vec![id];
        state.mark_dirty();
        true
    }
}
