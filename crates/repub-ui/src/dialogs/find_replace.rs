//! Find and Replace dialog.
//!
//! Searches every text frame in the publication. Matches are tracked as
//! (page, object, paragraph, character offset) so "Find Next" can select the
//! text in place and "Replace" can edit exactly the matched range.

use egui::{Key, TextEdit};
use repub_model::ObjectId;
use repub_model::shape::ShapeKind;

use crate::state::AppState;
use crate::text_edit::{self, TextPos};
use crate::theme::{colors, fonts, section_header};

/// One search hit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub page: usize,
    pub object: ObjectId,
    pub para: usize,
    pub offset: usize,
    pub len: usize,
}

/// Find and Replace dialog state.
#[derive(Debug, Clone, Default)]
pub struct FindReplaceDialog {
    /// Whether the dialog is open
    pub open: bool,
    /// Search query
    pub query: String,
    /// Replacement text
    pub replace_with: String,
    /// Case-sensitive matching
    pub match_case: bool,
    /// Whole-word matching
    pub whole_word: bool,
    /// Current hits (recomputed on Find Next / Replace)
    pub matches: Vec<Match>,
    /// Index of the current hit
    pub current: usize,
    /// Set when the last search found nothing
    pub no_matches: bool,
    /// Showing the replace field and buttons (Replace) or only searching (Find)
    pub replace_mode: bool,
    /// Move keyboard focus to the first field on the next frame
    focus_pending: bool,
}

impl FindReplaceDialog {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the dialog for finding, keeping any previous query.
    pub fn open(&mut self) {
        self.open = true;
        self.replace_mode = false;
        self.focus_pending = true;
    }

    /// Open the dialog with the replace field and buttons.
    pub fn open_replace(&mut self) {
        self.open = true;
        self.replace_mode = true;
        self.focus_pending = true;
    }

    /// Show the dialog. Returns true when a search or replacement was performed.
    pub fn show(&mut self, ctx: &egui::Context, state: &mut AppState) -> bool {
        if !self.open {
            return false;
        }
        let mut did_something = false;
        let mut close = false;
        let now = ctx.input(|i| i.time);

        egui::Window::new(if self.replace_mode { "Replace" } else { "Find" })
            .id(egui::Id::new("find_replace_dialog"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_TOP, [-24.0, 120.0])
            .show(ctx, |ui| {
                ui.set_min_width(340.0);
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Find what").font(fonts::body()));
                    let resp = ui.add(
                        TextEdit::singleline(&mut self.query)
                            .desired_width(200.0)
                            .hint_text("Search text"),
                    );
                    if std::mem::take(&mut self.focus_pending) {
                        resp.request_focus();
                    }
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)) {
                        self.find_next(state, true, now);
                        did_something = true;
                    }
                });

                if self.replace_mode {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Replace with").font(fonts::body()));
                        ui.add(
                            TextEdit::singleline(&mut self.replace_with)
                                .desired_width(200.0)
                                .hint_text("Replacement"),
                        );
                    });
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.match_case, "Match case");
                    ui.checkbox(&mut self.whole_word, "Whole words only");
                });

                if self.no_matches {
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("No matches found.")
                            .font(fonts::caption())
                            .color(colors::text_secondary()),
                    );
                }

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Find Next").clicked() {
                        self.find_next(state, true, now);
                        did_something = true;
                    }
                    if self.replace_mode {
                        if ui.button("Replace").clicked() {
                            self.replace_current(state, now);
                            did_something = true;
                        }
                        if ui.button("Replace All").clicked() {
                            self.replace_all(state, now);
                            did_something = true;
                        }
                    } else if ui.button("Replace…").clicked() {
                        self.replace_mode = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Close").clicked() {
                            close = true;
                        }
                    });
                });
            });

        if close {
            self.open = false;
        }
        did_something
    }

    /// Recompute the hit list from the current query.
    pub fn search(&mut self, state: &AppState) {
        self.recompute(state);
    }

    fn recompute(&mut self, state: &AppState) {
        self.matches.clear();
        self.no_matches = false;
        if self.query.is_empty() {
            return;
        }
        let Some(doc) = state.document.as_ref() else { return };
        for (page_idx, page) in doc.pages().iter().enumerate() {
            for obj in page.layers().iter().flat_map(|l| l.shapes()) {
                let ShapeKind::TextFrame(tf) = &obj.kind else { continue };
                for (para_idx, para) in tf.content.paragraphs.iter().enumerate() {
                    let chars = text_edit::para_chars(para);
                    let mut start = 0;
                    while start < chars.len() {
                        if let Some(len) = match_at(&chars, start, &self.query, self.match_case, self.whole_word) {
                            self.matches.push(Match {
                                page: page_idx,
                                object: obj.common.id,
                                para: para_idx,
                                offset: start,
                                len,
                            });
                            start += len.max(1);
                        } else {
                            start += 1;
                        }
                    }
                }
            }
        }
        self.no_matches = self.matches.is_empty();
        self.current = self.current.min(self.matches.len().saturating_sub(1));
    }

    fn find_next(&mut self, state: &mut AppState, advance: bool, now: f64) {
        self.recompute(state);
        if self.matches.is_empty() {
            return;
        }
        if advance && !self.matches.is_empty() {
            self.current = (self.current + 1) % self.matches.len();
        }
        self.select_match(state, now);
    }

    /// Select the current hit in the document and begin text editing over it.
    pub fn select_current(&mut self, state: &mut AppState, now: f64) {
        self.select_match(state, now);
    }

    fn select_match(&mut self, state: &mut AppState, now: f64) {
        let Some(m) = self.matches.get(self.current).copied() else { return };
        state.current_page = m.page;
        state.selection = vec![m.object];
        state.fit_page_pending = false;
        text_edit::begin(state, m.object, Some(TextPos::new(m.para, m.offset)), now);
        if let Some(edit) = state.text_edit.as_mut() {
            edit.anchor = TextPos::new(m.para, m.offset + m.len);
        }
    }

    /// Replace the currently selected hit.
    pub fn replace_current(&mut self, state: &mut AppState, now: f64) {
        self.replace_current_inner(state, now);
    }

    fn replace_current_inner(&mut self, state: &mut AppState, now: f64) {
        if self.matches.is_empty() {
            self.recompute(state);
        }
        let Some(m) = self.matches.get(self.current).copied() else {
            self.no_matches = true;
            return;
        };
        state.record_undo("Replace Text");
        let replaced = self.replace_in_object(state, m);
        if replaced {
            self.find_next(state, false, now);
        }
    }

    fn replace_all(&mut self, state: &mut AppState, now: f64) {
        self.recompute(state);
        if self.matches.is_empty() {
            return;
        }
        state.record_undo("Replace All");
        // Apply from the end of the document backwards so earlier offsets stay valid.
        let matches = self.matches.clone();
        for m in matches.into_iter().rev() {
            self.replace_in_object(state, m);
        }
        state.mark_dirty();
        self.recompute(state);
        self.current = 0;
        if !self.matches.is_empty() {
            self.select_match(state, now);
        }
    }

    fn replace_in_object(&self, state: &mut AppState, m: Match) -> bool {
        let Some(page) = state.document.as_mut().and_then(|d| d.pages_mut().get_mut(m.page)) else {
            return false;
        };
        let Some(shape) = page.get_shape_mut(m.object) else {
            return false;
        };
        let ShapeKind::TextFrame(tf) = &mut shape.kind else {
            return false;
        };
        if m.para >= tf.content.paragraphs.len() {
            return false;
        }
        let start = TextPos::new(m.para, m.offset);
        let end = TextPos::new(m.para, m.offset + m.len);
        // Edit through the shared helpers so styling of surrounding runs survives.
        let mut text = std::mem::replace(&mut tf.content, repub_model::text::StyledText::new());
        text_edit::delete_range(&mut text, start, end);
        if !self.replace_with.is_empty() {
            text_edit::insert_text(&mut text, start, &self.replace_with);
        }
        tf.content = text;
        state.mark_dirty();
        true
    }
}

/// Match `query` at `start` in `chars`; returns its length when it matches.
fn match_at(chars: &[char], start: usize, query: &str, match_case: bool, whole_word: bool) -> Option<usize> {
    let q: Vec<char> = query.chars().collect();
    if q.is_empty() || start + q.len() > chars.len() {
        return None;
    }
    for (i, qc) in q.iter().enumerate() {
        let c = chars[start + i];
        let ok = if match_case { c == *qc } else { c.eq_ignore_ascii_case(qc) };
        if !ok {
            return None;
        }
    }
    if whole_word {
        if start > 0 && chars[start - 1].is_alphanumeric() {
            return None;
        }
        if let Some(next) = chars.get(start + q.len()) {
            if next.is_alphanumeric() {
                return None;
            }
        }
    }
    Some(q.len())
}
