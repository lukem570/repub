//! Character and paragraph formatting of text boxes.
//!
//! Formatting acts on what the user expects:
//! - while editing text: the selected text, or with no selection the word at
//!   the caret (as in Publisher and Word); paragraph formatting acts on the
//!   paragraphs the selection touches;
//! - otherwise: the whole of every selected text box, including each
//!   paragraph's default style so new typing matches.
//!
//! Toggles (bold, italic, underline) turn the style off only when the whole
//! target already has it, else on, so mixed text becomes uniform.
//! Every change is one undo step, and editing continues with the same selection.

use repub_color::Color;
use repub_model::ObjectId;
use repub_model::shape::ShapeKind;
use repub_model::styles::{BaselineShift, CharacterStyle, FontStretch, FontStyle, FontWeight, LineSpacing, ListStyle, NumberFormat, ParagraphStyle, StrikethroughStyle, TextAlignment, UnderlineStyle};
use repub_model::text::{InlineElement, StyledText};

use crate::state::AppState;
use crate::text_edit::{self, element_len, para_len, split_run_at, TextPos};

/// A style that toggles on and off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    Bold,
    Italic,
    Underline,
    Superscript,
    Subscript,
}

impl Toggle {
    fn label(self) -> &'static str {
        match self {
            Toggle::Bold => "Bold",
            Toggle::Italic => "Italic",
            Toggle::Underline => "Underline",
            Toggle::Superscript => "Superscript",
            Toggle::Subscript => "Subscript",
        }
    }

    fn is_on(self, style: &CharacterStyle) -> bool {
        match self {
            Toggle::Bold => style.font.weight == FontWeight::Bold,
            Toggle::Italic => style.font.style == FontStyle::Italic,
            Toggle::Underline => style.underline != UnderlineStyle::None,
            Toggle::Superscript => style.baseline_shift == BaselineShift::Superscript,
            Toggle::Subscript => style.baseline_shift == BaselineShift::Subscript,
        }
    }

    fn set(self, style: &mut CharacterStyle, on: bool) {
        match self {
            Toggle::Bold => style.font.weight = if on { FontWeight::Bold } else { FontWeight::Normal },
            Toggle::Italic => style.font.style = if on { FontStyle::Italic } else { FontStyle::Normal },
            Toggle::Underline => style.underline = if on { UnderlineStyle::Single } else { UnderlineStyle::None },
            Toggle::Superscript => style.baseline_shift = if on { BaselineShift::Superscript } else { BaselineShift::Normal },
            Toggle::Subscript => style.baseline_shift = if on { BaselineShift::Subscript } else { BaselineShift::Normal },
        }
    }
}

/// A character formatting change.
#[derive(Debug, Clone, PartialEq)]
pub enum CharChange {
    Set(Toggle, bool),
    Size(f64),
    /// One step up (`true`) or down the standard size list.
    StepSize(bool),
    Color(Color),
    /// Font family name
    Family(String),
    /// Several character properties at once (the Font dialog)
    Patch(CharPatch),
}

/// Character properties to set; `None` leaves a property as it is.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CharPatch {
    pub family: Option<String>,
    pub weight: Option<FontWeight>,
    pub style: Option<FontStyle>,
    pub stretch: Option<FontStretch>,
    pub size: Option<f64>,
    pub color: Option<Color>,
    pub background: Option<Option<Color>>,
    pub underline: Option<UnderlineStyle>,
    pub underline_color: Option<Option<Color>>,
    pub strikethrough: Option<StrikethroughStyle>,
    pub baseline: Option<BaselineShift>,
    pub letter_spacing: Option<f64>,
    pub all_caps: Option<bool>,
    pub small_caps: Option<bool>,
}

impl CharPatch {
    /// Whether the patch changes nothing.
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    fn apply(&self, style: &mut CharacterStyle) {
        if let Some(v) = &self.family {
            style.font.family = v.clone();
        }
        if let Some(v) = self.weight {
            style.font.weight = v;
        }
        if let Some(v) = self.style {
            style.font.style = v;
        }
        if let Some(v) = self.stretch {
            style.font.stretch = v;
        }
        if let Some(v) = self.size {
            style.font_size = v.clamp(1.0, 1638.0);
        }
        if let Some(v) = self.color {
            style.color = v;
        }
        if let Some(v) = self.background {
            style.background_color = v;
        }
        if let Some(v) = self.underline {
            style.underline = v;
        }
        if let Some(v) = self.underline_color {
            style.underline_color = v;
        }
        if let Some(v) = self.strikethrough {
            style.strikethrough = v;
        }
        if let Some(v) = self.baseline {
            style.baseline_shift = v;
        }
        if let Some(v) = self.letter_spacing {
            style.letter_spacing = v;
        }
        if let Some(v) = self.all_caps {
            style.all_caps = v;
        }
        if let Some(v) = self.small_caps {
            style.small_caps = v;
        }
    }
}

/// Paragraph properties to set; `None` leaves a property as it is.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ParaPatch {
    pub alignment: Option<TextAlignment>,
    pub line_spacing: Option<LineSpacing>,
    pub space_before: Option<f64>,
    pub space_after: Option<f64>,
    pub first_line_indent: Option<f64>,
    pub left_indent: Option<f64>,
    pub right_indent: Option<f64>,
    pub list_style: Option<ListStyle>,
    pub keep_with_next: Option<bool>,
    pub keep_together: Option<bool>,
}

impl ParaPatch {
    pub fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    fn apply(&self, style: &mut ParagraphStyle) {
        if let Some(v) = self.alignment {
            style.alignment = v;
        }
        if let Some(v) = self.line_spacing {
            style.line_spacing = v;
        }
        if let Some(v) = self.space_before {
            style.space_before = v;
        }
        if let Some(v) = self.space_after {
            style.space_after = v;
        }
        if let Some(v) = self.first_line_indent {
            style.first_line_indent = v;
        }
        if let Some(v) = self.left_indent {
            style.left_indent = v;
        }
        if let Some(v) = self.right_indent {
            style.right_indent = v;
        }
        if let Some(v) = &self.list_style {
            style.list_style = v.clone();
        }
        if let Some(v) = self.keep_with_next {
            style.keep_with_next = v;
        }
        if let Some(v) = self.keep_together {
            style.keep_together = v;
        }
    }
}

/// Publisher's font size list, used by Grow/Shrink Font.
pub const FONT_SIZES: [f64; 16] = [8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 36.0, 48.0, 72.0];

fn step_size(size: f64, up: bool) -> f64 {
    if up {
        FONT_SIZES.iter().copied().find(|&s| s > size + 0.01).unwrap_or((size + 10.0).min(1638.0))
    } else {
        FONT_SIZES.iter().rev().copied().find(|&s| s < size - 0.01).unwrap_or((size - 1.0).max(1.0))
    }
}

impl CharChange {
    fn apply(&self, style: &mut CharacterStyle) {
        match self {
            CharChange::Set(toggle, on) => toggle.set(style, *on),
            CharChange::Size(size) => style.font_size = size.clamp(1.0, 1638.0),
            CharChange::StepSize(up) => style.font_size = step_size(style.font_size, *up),
            CharChange::Color(color) => style.color = *color,
            CharChange::Family(family) => style.font.family = family.clone(),
            CharChange::Patch(patch) => patch.apply(style),
        }
    }

    fn label(&self) -> &'static str {
        match self {
            CharChange::Set(toggle, _) => toggle.label(),
            CharChange::Size(_) | CharChange::StepSize(_) => "Font Size",
            CharChange::Color(_) => "Font Color",
            CharChange::Family(_) | CharChange::Patch(_) => "Font",
        }
    }
}

/// What formatting applies to.
enum Target {
    /// Part of the text being edited (start <= end): a text box, or a cell
    /// of a table.
    Range { frame: ObjectId, cell: Option<(usize, usize)>, start: TextPos, end: TextPos },
    /// Whole text boxes and tables (every cell).
    Frames(Vec<ObjectId>),
}

fn target(state: &AppState) -> Option<Target> {
    if let Some(edit) = &state.text_edit {
        let (start, end) = (edit.caret.min(edit.anchor), edit.caret.max(edit.anchor));
        if start != end {
            return Some(Target::Range { frame: edit.frame, cell: edit.cell, start, end });
        }
        // No selection: the word at the caret
        let text = text_of(state, edit.frame)?;
        let (a, b) = text_edit::word_at(text, edit.caret);
        let word = text_edit::text_between(text, a, b);
        if !word.trim().is_empty() {
            return Some(Target::Range { frame: edit.frame, cell: edit.cell, start: a, end: b });
        }
        return None;
    }
    let frames: Vec<ObjectId> = state
        .selected_objects()
        .iter()
        .filter(|o| matches!(o.kind, ShapeKind::TextFrame(_) | ShapeKind::Table(_)))
        .map(|o| o.id())
        .collect();
    (!frames.is_empty()).then_some(Target::Frames(frames))
}

fn text_of(state: &AppState, id: ObjectId) -> Option<&StyledText> {
    // A text box's text, or the table cell being edited
    text_edit::text_ref(state, id)
}

/// Offsets `start..end` of paragraph `para` covered by `[a, b)`.
fn para_span(text: &StyledText, para: usize, a: TextPos, b: TextPos) -> (usize, usize) {
    let len = text.paragraphs.get(para).map_or(0, para_len);
    let start = if para == a.para { a.offset.min(len) } else { 0 };
    let end = if para == b.para { b.offset.min(len) } else { len };
    (start, end)
}

/// Run `f` on the style of every character in `[a, b)` (whole text when `None`),
/// splitting runs at the ends so nothing outside changes.
fn style_range(text: &mut StyledText, range: Option<(TextPos, TextPos)>, f: &mut dyn FnMut(&mut CharacterStyle)) {
    let Some((a, b)) = range else {
        for p in &mut text.paragraphs {
            f(&mut p.style.default_char_style);
            for e in &mut p.elements {
                match e {
                    InlineElement::Text(run) => f(&mut run.style),
                    InlineElement::Link(link) => f(&mut link.style),
                    _ => {}
                }
            }
        }
        return;
    };
    for para in a.para..=b.para.min(text.paragraphs.len().saturating_sub(1)) {
        let (start, end) = para_span(text, para, a, b);
        if start >= end {
            continue;
        }
        let p = &mut text.paragraphs[para];
        split_run_at(p, start);
        split_run_at(p, end);
        let mut pos = 0;
        for e in &mut p.elements {
            let len = element_len(e);
            if pos >= start && pos + len <= end {
                match e {
                    InlineElement::Text(run) => f(&mut run.style),
                    InlineElement::Link(link) => f(&mut link.style),
                    _ => {}
                }
            }
            pos += len;
        }
    }
}

/// Styles of the characters in `[a, b)` (whole text when `None`), read-only.
fn styles_in(text: &StyledText, range: Option<(TextPos, TextPos)>) -> Vec<CharacterStyle> {
    let mut out = Vec::new();
    for (index, p) in text.paragraphs.iter().enumerate() {
        let (start, end) = match range {
            Some((a, b)) if index < a.para || index > b.para => continue,
            Some((a, b)) => para_span(text, index, a, b),
            None => (0, para_len(p)),
        };
        let mut pos = 0;
        for e in &p.elements {
            let len = element_len(e);
            if pos < end && pos + len > start {
                match e {
                    InlineElement::Text(run) if !run.text.is_empty() => out.push(run.style.clone()),
                    InlineElement::Link(link) => out.push(link.style.clone()),
                    _ => {}
                }
            }
            pos += len;
        }
        // An empty paragraph's look comes from its default style
        if out.is_empty() && range.is_none() && para_len(p) == 0 {
            out.push(p.style.default_char_style.clone());
        }
    }
    out
}

/// Change the text in the target as one undo step. While editing, the session
/// is closed and reopened around the change with the same selection, so the
/// typing before it stays its own undo step.
fn edit_target(state: &mut AppState, label: &str, coalesce: bool, f: impl Fn(&mut StyledText, Option<(TextPos, TextPos)>)) -> bool {
    let record = |state: &mut AppState| if coalesce { state.record_undo_coalesced(label) } else { state.record_undo(label) };
    let Some(target) = target(state) else { return false };
    match target {
        Target::Range { frame, cell, start, end } => {
            let Some(edit) = state.text_edit.clone() else { return false };
            text_edit::end(state);
            record(state);
            text_edit::for_each_text_mut(state, frame, cell, |text| f(text, Some((start, end))));
            state.begin_edit();
            state.text_edit = Some(edit);
        }
        Target::Frames(ids) => {
            record(state);
            for id in ids {
                text_edit::for_each_text_mut(state, id, None, |text| f(text, None));
            }
        }
    }
    state.mark_dirty();
    true
}

/// Apply a character change to the target. Returns false when there is no target.
pub fn apply(state: &mut AppState, change: CharChange) -> bool {
    let label = change.label();
    edit_target(state, label, false, |text, range| style_range(text, range, &mut |s| change.apply(s)))
}

/// Like [`apply`], for continuous edits (dragging a size, picking a colour):
/// repeated changes within a second are one undo step.
pub fn apply_continuous(state: &mut AppState, change: CharChange) -> bool {
    let label = change.label();
    edit_target(state, label, true, |text, range| style_range(text, range, &mut |s| change.apply(s)))
}

/// Toggle bold/italic/underline: off if the whole target has it, else on.
pub fn toggle(state: &mut AppState, toggle: Toggle) -> bool {
    let on = !is_on(state, toggle);
    apply(state, CharChange::Set(toggle, on))
}

/// Whether the whole target has `toggle` (false with no target).
pub fn is_on(state: &AppState, toggle: Toggle) -> bool {
    let styles = target_styles(state);
    !styles.is_empty() && styles.iter().all(|s| toggle.is_on(s))
}

/// Character styles across the target (for showing current values).
pub fn target_styles(state: &AppState) -> Vec<CharacterStyle> {
    match target(state) {
        Some(Target::Range { frame, cell, start, end }) => text_edit::texts_in(state, frame, cell).into_iter().flat_map(|t| styles_in(t, Some((start, end)))).collect(),
        Some(Target::Frames(ids)) => ids.iter().flat_map(|&id| text_edit::texts_in(state, id, None)).flat_map(|t| styles_in(t, None)).collect(),
        None => {
            // Caret between words: the style that typing would continue
            let edit = state.text_edit.as_ref();
            edit.and_then(|e| text_of(state, e.frame).map(|t| (t, e.caret)))
                .map(|(t, caret)| {
                    let before = TextPos::new(caret.para, caret.offset.saturating_sub(1));
                    styles_in(t, Some((before, TextPos::new(caret.para, caret.offset.max(1)))))
                })
                .unwrap_or_default()
        }
    }
}

/// The font family shared by the whole target, if it is uniform.
pub fn current_family(state: &AppState) -> Option<String> {
    let styles = target_styles(state);
    let first = styles.first()?.font.family.clone();
    styles.iter().all(|s| s.font.family == first).then_some(first)
}

/// The font size shared by the whole target, if it is uniform.
pub fn current_size(state: &AppState) -> Option<f64> {
    let styles = target_styles(state);
    let first = styles.first()?.font_size;
    styles.iter().all(|s| (s.font_size - first).abs() < 0.01).then_some(first)
}

/// Paragraphs that paragraph formatting acts on: `(object, cell, paragraph
/// range)`. A whole table is every paragraph of every cell.
type ParagraphTarget = (ObjectId, Option<(usize, usize)>, std::ops::RangeInclusive<usize>);

fn target_paragraphs(state: &AppState) -> Vec<ParagraphTarget> {
    if let Some(edit) = &state.text_edit {
        let (a, b) = (edit.caret.min(edit.anchor), edit.caret.max(edit.anchor));
        return vec![(edit.frame, edit.cell, a.para..=b.para)];
    }
    state
        .selected_objects()
        .iter()
        .filter_map(|o| match &o.kind {
            ShapeKind::TextFrame(tf) => Some((o.common.id, None, 0..=tf.content.paragraphs.len().saturating_sub(1))),
            ShapeKind::Table(_) => Some((o.common.id, None, 0..=usize::MAX)),
            _ => None,
        })
        .collect()
}

/// The paragraphs of `paragraphs` whose index is in `range`.
fn in_range<'a, T: 'a>(paragraphs: impl Iterator<Item = T> + 'a, range: &'a std::ops::RangeInclusive<usize>) -> impl Iterator<Item = T> + 'a {
    paragraphs.enumerate().filter(move |(i, _)| range.contains(i)).map(|(_, p)| p)
}

/// Styles of the target paragraphs, in order.
fn target_paragraph_styles(state: &AppState) -> Vec<&ParagraphStyle> {
    let mut out = Vec::new();
    for (id, cell, paras) in target_paragraphs(state) {
        for text in text_edit::texts_in(state, id, cell) {
            out.extend(text.paragraphs.iter().enumerate().filter(|(i, _)| paras.contains(i)).map(|(_, p)| &p.style));
        }
    }
    out
}

/// Change the style of the target paragraphs as one undo step. Returns false
/// when there is nothing to change.
fn edit_paragraphs(state: &mut AppState, label: &str, f: impl Fn(&mut ParagraphStyle)) -> bool {
    let targets = target_paragraphs(state);
    if targets.is_empty() {
        return false;
    }
    let edit = state.text_edit.clone();
    if edit.is_some() {
        text_edit::end(state);
    }
    state.record_undo(label);
    for (id, cell, paras) in targets {
        text_edit::for_each_text_mut(state, id, cell, |text| {
            for p in in_range(text.paragraphs.iter_mut(), &paras) {
                f(&mut p.style);
            }
        });
    }
    if let Some(edit) = edit {
        state.begin_edit();
        state.text_edit = Some(edit);
    }
    state.mark_dirty();
    true
}

/// Set several paragraph properties on the target paragraphs (the Paragraph dialog).
pub fn apply_paragraph_patch(state: &mut AppState, patch: &ParaPatch) -> bool {
    if patch.is_empty() {
        return false;
    }
    edit_paragraphs(state, "Paragraph", |p| patch.apply(p))
}

/// The style of the first target paragraph and whether the targets disagree on any property
/// (`mixed`), for showing the current values.
pub fn paragraph_snapshot(state: &AppState) -> Option<(ParagraphStyle, bool)> {
    let styles = target_paragraph_styles(state);
    let first = (*styles.first()?).clone();
    let mixed = styles.iter().any(|s| {
        let mut a = (*s).clone();
        let mut b = first.clone();
        a.default_char_style = CharacterStyle::default();
        b.default_char_style = CharacterStyle::default();
        a != b
    });
    Some((first, mixed))
}

/// The first character style of the target and whether the target mixes styles.
pub fn character_snapshot(state: &AppState) -> Option<(CharacterStyle, bool)> {
    let styles = target_styles(state);
    let first = styles.first()?.clone();
    let mixed = styles.iter().any(|s| *s != first);
    Some((first, mixed))
}

/// Align the target paragraphs. Returns false when there is nothing to align.
pub fn set_alignment(state: &mut AppState, alignment: TextAlignment) -> bool {
    edit_paragraphs(state, "Text Alignment", |p| p.alignment = alignment)
}

/// Kind of list for the Bullets / Numbering buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    Bullets,
    Numbering,
}

fn list_kind(style: &ListStyle) -> Option<ListKind> {
    match style {
        ListStyle::None => None,
        ListStyle::Bullet { .. } => Some(ListKind::Bullets),
        ListStyle::Numbered { .. } => Some(ListKind::Numbering),
    }
}

/// The list kind shared by all target paragraphs, if they agree.
pub fn current_list(state: &AppState) -> Option<ListKind> {
    let mut kinds = target_paragraph_styles(state).into_iter().map(|p| list_kind(&p.list_style));
    let first = kinds.next()?;
    kinds.all(|k| k == first).then_some(first).flatten()
}

/// Bullets/Numbering button: off when every target paragraph already has that
/// kind of list, else on (with a ¼" hanging indent).
pub fn toggle_list(state: &mut AppState, kind: ListKind) -> bool {
    let on = current_list(state) != Some(kind);
    let style = match (on, kind) {
        (false, _) => ListStyle::None,
        (true, ListKind::Bullets) => ListStyle::Bullet { character: '•', indent: 18.0 },
        (true, ListKind::Numbering) => ListStyle::Numbered { format: NumberFormat::Arabic, start: 1, indent: 18.0 },
    };
    let label = match kind {
        ListKind::Bullets => "Bullets",
        ListKind::Numbering => "Numbering",
    };
    edit_paragraphs(state, label, |p| p.list_style = style.clone())
}

/// Indent step for Increase/Decrease Indent: a quarter inch.
pub const INDENT_STEP: f64 = 18.0;

/// Move the target paragraphs' left indent one step in or out (never negative).
pub fn change_indent(state: &mut AppState, increase: bool) -> bool {
    edit_paragraphs(state, if increase { "Increase Indent" } else { "Decrease Indent" }, |p| {
        p.left_indent = if increase {
            (p.left_indent / INDENT_STEP).floor() * INDENT_STEP + INDENT_STEP
        } else {
            ((p.left_indent / INDENT_STEP).ceil() * INDENT_STEP - INDENT_STEP).max(0.0)
        };
    })
}

/// Set line spacing as a multiple of single spacing.
pub fn set_line_spacing(state: &mut AppState, multiple: f64) -> bool {
    let spacing = match multiple {
        m if (m - 1.0).abs() < 1e-6 => LineSpacing::Single,
        m if (m - 1.5).abs() < 1e-6 => LineSpacing::OneAndHalf,
        m if (m - 2.0).abs() < 1e-6 => LineSpacing::Double,
        m => LineSpacing::Multiple(m),
    };
    edit_paragraphs(state, "Line Spacing", |p| p.line_spacing = spacing.clone())
}

/// Line spacing of the target paragraphs as a multiple, if they agree and it is one.
pub fn current_line_spacing(state: &AppState) -> Option<f64> {
    let mut found: Option<f64> = None;
    for p in target_paragraph_styles(state) {
        {
            let m = match p.line_spacing {
                LineSpacing::Single => 1.0,
                LineSpacing::OneAndHalf => 1.5,
                LineSpacing::Double => 2.0,
                LineSpacing::Multiple(m) => m,
                _ => return None,
            };
            match found {
                None => found = Some(m),
                Some(f) if (f - m).abs() > 1e-6 => return None,
                _ => {}
            }
        }
    }
    found
}

/// The alignment of the target paragraphs, if they agree.
pub fn current_alignment(state: &AppState) -> Option<TextAlignment> {
    let mut found = None;
    for p in target_paragraph_styles(state) {
        {
            match found {
                None => found = Some(p.alignment),
                Some(a) if a != p.alignment => return None,
                _ => {}
            }
        }
    }
    found
}

/// Whether formatting has anything to act on (text box selected or being edited).
pub fn has_target(state: &AppState) -> bool {
    state.text_edit.is_some() || !target_paragraphs(state).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_model::text::{Paragraph, TextRun};

    fn bold_of(text: &StyledText) -> Vec<(String, bool)> {
        text.paragraphs[0]
            .elements
            .iter()
            .filter_map(|e| match e {
                InlineElement::Text(r) if !r.text.is_empty() => Some((r.text.clone(), r.style.font.weight == FontWeight::Bold)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn range_styling_splits_runs_and_leaves_the_rest() {
        let mut text = StyledText { paragraphs: vec![Paragraph::plain("Hello brave world")] };
        style_range(&mut text, Some((TextPos::new(0, 6), TextPos::new(0, 11))), &mut |s| Toggle::Bold.set(s, true));
        assert_eq!(
            bold_of(&text),
            vec![("Hello ".into(), false), ("brave".into(), true), (" world".into(), false)]
        );
    }

    #[test]
    fn range_across_paragraphs() {
        let mut text = StyledText { paragraphs: vec![Paragraph::plain("One two"), Paragraph::plain("Three four")] };
        style_range(&mut text, Some((TextPos::new(0, 4), TextPos::new(1, 5))), &mut |s| s.font_size = 30.0);
        let sizes = |p: &Paragraph| p.elements.iter().filter_map(|e| match e {
            InlineElement::Text(r) if !r.text.is_empty() => Some((r.text.clone(), r.style.font_size)),
            _ => None,
        }).collect::<Vec<_>>();
        let default = CharacterStyle::default().font_size;
        assert_eq!(sizes(&text.paragraphs[0]), vec![("One ".into(), default), ("two".into(), 30.0)]);
        assert_eq!(sizes(&text.paragraphs[1]), vec![("Three".into(), 30.0), (" four".into(), default)]);
    }

    #[test]
    fn styles_in_reads_only_the_range() {
        let mut p = Paragraph::new();
        let mut bold = CharacterStyle::default();
        bold.font.weight = FontWeight::Bold;
        p.elements.push(InlineElement::Text(TextRun::new("plain ")));
        p.elements.push(InlineElement::Text(TextRun::styled("bold", bold)));
        let text = StyledText { paragraphs: vec![p] };
        let styles = styles_in(&text, Some((TextPos::new(0, 6), TextPos::new(0, 10))));
        assert!(styles.iter().all(|s| s.font.weight == FontWeight::Bold));
        assert_eq!(styles_in(&text, None).len(), 2);
    }

    #[test]
    fn size_steps_follow_the_list() {
        assert_eq!(step_size(11.0, true), 12.0);
        assert_eq!(step_size(12.0, false), 11.0);
        assert_eq!(step_size(13.0, true), 14.0);
        assert_eq!(step_size(72.0, true), 82.0);
        assert_eq!(step_size(8.0, false), 7.0);
    }
}
