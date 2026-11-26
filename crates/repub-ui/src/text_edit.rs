//! In-place text editing on the canvas.
//!
//! A text frame enters edit mode by double-clicking it, clicking it with the
//! Text tool, drawing a new frame with the Text tool, or pressing Return while
//! it is the only selected object. While editing, keyboard input goes to the
//! text (typing, Backspace/Delete, Return for a new paragraph, arrows with
//! Shift to select, Option/Alt+arrows by word, Home/End, Cmd/Ctrl+A), and the
//! mouse places the caret or drags a selection. Escape or clicking outside the
//! frame ends editing. The whole session is one undo step.
//!
//! Character offsets: one per character of a text run or link, and one per
//! tab, field, line break or inline image (matching `sdf_text` layout).

use repub_model::ObjectId;
use std::borrow::Cow;

use repub_model::shape::{Shape, ShapeKind, TextFrameData};
use repub_model::text::{InlineElement, Paragraph, StyledText, TextRun};

use crate::sdf_text::{self, FrameLayout};
use crate::state::AppState;

/// The [`TextFrameData`] payload of a text-frame shape, if `shape` is one.
pub(crate) fn text_frame_data(shape: &Shape) -> Option<&TextFrameData> {
    match &shape.kind {
        ShapeKind::TextFrame(tf) => Some(tf),
        _ => None,
    }
}

/// The text content of a text-frame shape (the frames returned by [`frame`] are
/// always text frames, so this is used for their content). Empty otherwise.
fn frame_content(shape: &Shape) -> &StyledText {
    static EMPTY: std::sync::OnceLock<StyledText> = std::sync::OnceLock::new();
    match &shape.kind {
        ShapeKind::TextFrame(tf) => &tf.content,
        _ => EMPTY.get_or_init(StyledText::new),
    }
}

/// A caret position: paragraph index and character offset within it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct TextPos {
    pub para: usize,
    pub offset: usize,
}

impl TextPos {
    pub fn new(para: usize, offset: usize) -> Self {
        Self { para, offset }
    }
}

/// An editing session on one text frame.
#[derive(Debug, Clone)]
pub struct TextEditState {
    /// The text box, or the table whose `cell` is being edited.
    pub frame: ObjectId,
    /// Table cell being edited (row, column), when `frame` is a table.
    pub cell: Option<(usize, usize)>,
    pub caret: TextPos,
    /// Other end of the selection; equal to `caret` when nothing is selected.
    pub anchor: TextPos,
    /// Remembered x (document points) for vertical caret movement.
    pub preferred_x: Option<f32>,
    /// Time the caret last moved, so it stays solid while typing.
    pub blink_epoch: f64,
}

impl TextEditState {
    pub fn has_selection(&self) -> bool {
        self.caret != self.anchor
    }

    /// Selection as an ordered (start, end) pair.
    pub fn range(&self) -> (TextPos, TextPos) {
        if self.caret <= self.anchor { (self.caret, self.anchor) } else { (self.anchor, self.caret) }
    }
}

// ----------------------------------------------------------------------- model edits

pub(crate) fn element_len(e: &InlineElement) -> usize {
    match e {
        InlineElement::Text(run) => run.text.chars().count(),
        InlineElement::Link(link) => link.text.chars().count(),
        _ => 1,
    }
}

/// Number of character offsets in a paragraph.
pub fn para_len(p: &Paragraph) -> usize {
    p.elements.iter().map(element_len).sum()
}

/// The paragraph's characters, one per offset (non-text elements become placeholders).
pub fn para_chars(p: &Paragraph) -> Vec<char> {
    let mut out = Vec::new();
    for e in &p.elements {
        match e {
            InlineElement::Text(run) => out.extend(run.text.chars()),
            InlineElement::Link(link) => out.extend(link.text.chars()),
            InlineElement::Tab => out.push('\t'),
            InlineElement::LineBreak => out.push('\n'),
            _ => out.push('\u{FFFC}'),
        }
    }
    out
}

fn byte_index(s: &str, char_index: usize) -> usize {
    s.char_indices().nth(char_index).map_or(s.len(), |(b, _)| b)
}

/// Insert `text` (no newlines) at `offset`, continuing the style of the text
/// before the caret (or the paragraph's default style).
fn insert_in_para(p: &mut Paragraph, offset: usize, text: &str) {
    if text.is_empty() {
        return;
    }
    let mut start = 0;
    let mut last_style = None;
    for e in p.elements.iter_mut() {
        let len = element_len(e);
        match e {
            InlineElement::Text(run) => {
                if offset >= start && offset <= start + len {
                    let at = byte_index(&run.text, offset - start);
                    run.text.insert_str(at, text);
                    return;
                }
                last_style = Some(run.style.clone());
            }
            InlineElement::Link(link) => {
                if offset > start && offset < start + len {
                    let at = byte_index(&link.text, offset - start);
                    link.text.insert_str(at, text);
                    return;
                }
            }
            _ => {}
        }
        start += len;
        if start > offset {
            break;
        }
    }
    // Between non-text elements, or an empty paragraph: new run at the right index
    let style = last_style.unwrap_or_else(|| p.style.default_char_style.clone());
    let mut index = 0;
    let mut pos = 0;
    while index < p.elements.len() && pos < offset {
        pos += element_len(&p.elements[index]);
        index += 1;
    }
    p.elements.insert(index, InlineElement::Text(TextRun::styled(text, style)));
}

/// Delete character offsets `start..end` from a paragraph.
fn delete_in_para(p: &mut Paragraph, start: usize, end: usize) {
    if start >= end {
        return;
    }
    let mut pos = 0;
    let mut kept = Vec::with_capacity(p.elements.len());
    for mut e in p.elements.drain(..) {
        let len = element_len(&e);
        let (s, t) = (start.max(pos), end.min(pos + len));
        if s < t {
            match &mut e {
                InlineElement::Text(run) => {
                    let (a, b) = (byte_index(&run.text, s - pos), byte_index(&run.text, t - pos));
                    run.text.replace_range(a..b, "");
                }
                InlineElement::Link(link) => {
                    let (a, b) = (byte_index(&link.text, s - pos), byte_index(&link.text, t - pos));
                    link.text.replace_range(a..b, "");
                }
                // Non-text elements inside the range are removed entirely
                _ => {
                    pos += len;
                    continue;
                }
            }
        }
        pos += len;
        let empty = match &e {
            InlineElement::Text(run) => run.text.is_empty(),
            InlineElement::Link(link) => link.text.is_empty(),
            _ => false,
        };
        if !empty {
            kept.push(e);
        }
    }
    p.elements = kept;
}

/// Ensure the content has at least one paragraph to put the caret in.
fn ensure_paragraph(text: &mut StyledText) {
    if text.paragraphs.is_empty() {
        text.paragraphs.push(Paragraph::new());
    }
}

/// Delete the text between two positions (any order).
pub fn delete_range(text: &mut StyledText, a: TextPos, b: TextPos) -> TextPos {
    let (start, end) = if a <= b { (a, b) } else { (b, a) };
    if start == end || text.paragraphs.is_empty() {
        return start;
    }
    if start.para == end.para {
        delete_in_para(&mut text.paragraphs[start.para], start.offset, end.offset);
        return start;
    }
    // Keep the head of the first paragraph and the tail of the last, then join them
    let first_len = para_len(&text.paragraphs[start.para]);
    delete_in_para(&mut text.paragraphs[start.para], start.offset, first_len);
    delete_in_para(&mut text.paragraphs[end.para], 0, end.offset);
    let tail = text.paragraphs.remove(end.para);
    text.paragraphs.drain(start.para + 1..end.para);
    text.paragraphs[start.para].elements.extend(tail.elements);
    start
}

/// Insert text at `pos`; newlines start new paragraphs and tabs become tab
/// elements. Returns the caret after it.
pub fn insert_text(text: &mut StyledText, pos: TextPos, s: &str) -> TextPos {
    ensure_paragraph(text);
    let mut pos = clamp(text, pos);
    for (i, line) in s.split('\n').enumerate() {
        if i > 0 {
            pos = split_paragraph(text, pos);
        }
        for (j, part) in line.split('\t').enumerate() {
            if j > 0 {
                insert_inline(&mut text.paragraphs[pos.para], pos.offset, InlineElement::Tab);
                pos.offset += 1;
            }
            let part = part.replace('\r', "");
            insert_in_para(&mut text.paragraphs[pos.para], pos.offset, &part);
            pos.offset += part.chars().count();
        }
    }
    pos
}

/// Split a text or link run at `offset` so an element can be inserted there.
pub(crate) fn split_run_at(p: &mut Paragraph, offset: usize) {
    let mut pos = 0;
    for i in 0..p.elements.len() {
        let len = element_len(&p.elements[i]);
        if offset > pos && offset < pos + len {
            let local = offset - pos;
            match &p.elements[i] {
                InlineElement::Text(run) => {
                    let at = byte_index(&run.text, local);
                    let right = run.text[at..].to_string();
                    let style = run.style.clone();
                    if let InlineElement::Text(run) = &mut p.elements[i] {
                        run.text.truncate(at);
                    }
                    p.elements.insert(i + 1, InlineElement::Text(TextRun::styled(&right, style)));
                }
                InlineElement::Link(link) => {
                    let at = byte_index(&link.text, local);
                    let right = link.text[at..].to_string();
                    let mut right_link = link.clone();
                    right_link.text = right;
                    if let InlineElement::Link(link) = &mut p.elements[i] {
                        link.text.truncate(at);
                    }
                    p.elements.insert(i + 1, InlineElement::Link(right_link));
                }
                _ => {}
            }
            return;
        }
        pos += len;
        if pos >= offset {
            return;
        }
    }
}

/// The link around offset `pos`, as (start, end, link).
pub fn link_around(text: &StyledText, pos: TextPos) -> Option<(TextPos, TextPos, repub_model::text::Hyperlink)> {
    let p = text.paragraphs.get(pos.para)?;
    let mut at = 0;
    for e in &p.elements {
        let len = element_len(e);
        if let InlineElement::Link(link) = e {
            if pos.offset >= at && pos.offset <= at + len && len > 0 {
                return Some((TextPos::new(pos.para, at), TextPos::new(pos.para, at + len), link.clone()));
            }
        }
        at += len;
    }
    None
}

/// Insert a hyperlink element at `pos`. Returns the caret after the link text.
pub fn insert_link(text: &mut StyledText, pos: TextPos, link: repub_model::text::Hyperlink) -> TextPos {
    ensure_paragraph(text);
    let pos = clamp(text, pos);
    let p = &mut text.paragraphs[pos.para];
    split_run_at(p, pos.offset);
    let mut index = 0;
    let mut at = 0;
    while index < p.elements.len() && at < pos.offset {
        at += element_len(&p.elements[index]);
        index += 1;
    }
    let len = link.text.chars().count();
    p.elements.insert(index, InlineElement::Link(link));
    TextPos::new(pos.para, pos.offset + len)
}

/// Insert a non-text element (a tab, line break or field) at `offset`, splitting a text run if needed.
fn insert_inline(p: &mut Paragraph, offset: usize, element: InlineElement) {
    let mut pos = 0;
    for i in 0..p.elements.len() {
        let len = element_len(&p.elements[i]);
        if offset <= pos {
            p.elements.insert(i, element);
            return;
        }
        if offset < pos + len {
            if let InlineElement::Text(run) = &mut p.elements[i] {
                let at = byte_index(&run.text, offset - pos);
                let tail = TextRun::styled(&run.text[at..], run.style.clone());
                run.text.truncate(at);
                p.elements.insert(i + 1, element);
                p.elements.insert(i + 2, InlineElement::Text(tail));
                return;
            }
        }
        pos += len;
    }
    p.elements.push(element);
}

/// Split the paragraph at `pos` (Return). The new paragraph keeps the paragraph style.
pub fn split_paragraph(text: &mut StyledText, pos: TextPos) -> TextPos {
    ensure_paragraph(text);
    let pos = clamp(text, pos);
    let mut tail = text.paragraphs[pos.para].clone();
    let len = para_len(&tail);
    delete_in_para(&mut text.paragraphs[pos.para], pos.offset, len);
    delete_in_para(&mut tail, 0, pos.offset);
    // Carry the style at the split point into an empty new paragraph
    if tail.elements.is_empty() {
        if let Some(InlineElement::Text(run)) = text.paragraphs[pos.para].elements.last() {
            tail.style.default_char_style = run.style.clone();
        }
    }
    text.paragraphs.insert(pos.para + 1, tail);
    TextPos::new(pos.para + 1, 0)
}

/// Clamp a position to the content.
pub fn clamp(text: &StyledText, pos: TextPos) -> TextPos {
    if text.paragraphs.is_empty() {
        return TextPos::default();
    }
    let para = pos.para.min(text.paragraphs.len() - 1);
    TextPos::new(para, pos.offset.min(para_len(&text.paragraphs[para])))
}

/// Plain text between two positions; paragraphs joined with '\n'.
pub fn text_between(text: &StyledText, a: TextPos, b: TextPos) -> String {
    let (start, end) = if a <= b { (a, b) } else { (b, a) };
    let mut out = String::new();
    if text.paragraphs.is_empty() {
        return out;
    }
    for para in start.para..=end.para.min(text.paragraphs.len() - 1) {
        let chars = para_chars(&text.paragraphs[para]);
        let from = if para == start.para { start.offset } else { 0 };
        let to = if para == end.para { end.offset.min(chars.len()) } else { chars.len() };
        if from < to {
            out.extend(chars[from..to].iter().map(|&c| if c == '\u{FFFC}' { ' ' } else { c }));
        }
        if para != end.para {
            out.push('\n');
        }
    }
    out
}

/// End of the content.
pub fn end_pos(text: &StyledText) -> TextPos {
    match text.paragraphs.last() {
        Some(p) => TextPos::new(text.paragraphs.len() - 1, para_len(p)),
        None => TextPos::default(),
    }
}

// ----------------------------------------------------------------------- caret movement

/// Previous position (crossing paragraph boundaries).
pub fn prev_pos(text: &StyledText, pos: TextPos) -> TextPos {
    if pos.offset > 0 {
        TextPos::new(pos.para, pos.offset - 1)
    } else if pos.para > 0 {
        TextPos::new(pos.para - 1, para_len(&text.paragraphs[pos.para - 1]))
    } else {
        pos
    }
}

/// Next position (crossing paragraph boundaries).
pub fn next_pos(text: &StyledText, pos: TextPos) -> TextPos {
    let len = text.paragraphs.get(pos.para).map_or(0, para_len);
    if pos.offset < len {
        TextPos::new(pos.para, pos.offset + 1)
    } else if pos.para + 1 < text.paragraphs.len() {
        TextPos::new(pos.para + 1, 0)
    } else {
        pos
    }
}

fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '\''
}

/// Start of the previous word (Option/Alt+Left).
pub fn prev_word(text: &StyledText, pos: TextPos) -> TextPos {
    if pos.offset == 0 {
        return prev_pos(text, pos);
    }
    let chars = para_chars(&text.paragraphs[pos.para]);
    let mut i = pos.offset;
    while i > 0 && !is_word(chars[i - 1]) {
        i -= 1;
    }
    while i > 0 && is_word(chars[i - 1]) {
        i -= 1;
    }
    TextPos::new(pos.para, i)
}

/// End of the next word (Option/Alt+Right).
pub fn next_word(text: &StyledText, pos: TextPos) -> TextPos {
    let chars = text.paragraphs.get(pos.para).map(para_chars).unwrap_or_default();
    if pos.offset >= chars.len() {
        return next_pos(text, pos);
    }
    let mut i = pos.offset;
    while i < chars.len() && !is_word(chars[i]) {
        i += 1;
    }
    while i < chars.len() && is_word(chars[i]) {
        i += 1;
    }
    TextPos::new(pos.para, i)
}

/// Word around a position (double-click).
pub fn word_at(text: &StyledText, pos: TextPos) -> (TextPos, TextPos) {
    let Some(p) = text.paragraphs.get(pos.para) else { return (pos, pos) };
    let chars = para_chars(p);
    let (mut a, mut b) = (pos.offset.min(chars.len()), pos.offset.min(chars.len()));
    let class = |c: char| is_word(c);
    let target = if b < chars.len() { class(chars[b]) } else if a > 0 { class(chars[a - 1]) } else { true };
    while a > 0 && class(chars[a - 1]) == target && chars[a - 1] != '\n' {
        a -= 1;
    }
    while b < chars.len() && class(chars[b]) == target && chars[b] != '\n' {
        b += 1;
    }
    (TextPos::new(pos.para, a), TextPos::new(pos.para, b))
}

// ----------------------------------------------------------------------- layout queries

/// Line index containing a position (the later line when a wrap point is shared).
fn line_of(layout: &FrameLayout, pos: TextPos) -> Option<usize> {
    let mut found = None;
    for (i, line) in layout.lines.iter().enumerate() {
        if line.para == pos.para && pos.offset >= line.start && pos.offset <= line.end {
            found = Some(i);
            if pos.offset < line.end {
                break;
            }
        }
    }
    found
}

/// Caret x and vertical extent for a position, document points.
pub fn caret_geometry(layout: &FrameLayout, pos: TextPos) -> Option<(f32, f32, f32)> {
    let line = &layout.lines[line_of(layout, pos)?];
    let x = line.xs.get(pos.offset - line.start).copied().or(line.xs.last().copied())?;
    Some((x, line.top, line.bottom))
}

/// Nearest caret position to a point (frame-unrotated document coordinates).
pub fn hit_test(layout: &FrameLayout, x: f32, y: f32) -> TextPos {
    let Some(line) = layout
        .lines
        .iter()
        .min_by(|a, b| line_distance(a, y).total_cmp(&line_distance(b, y)))
    else {
        return TextPos::default();
    };
    let index = line
        .xs
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| (*a - x).abs().total_cmp(&(*b - x).abs()))
        .map_or(0, |(i, _)| i);
    TextPos::new(line.para, line.start + index)
}

fn line_distance(line: &sdf_text::CaretLine, y: f32) -> f32 {
    if y < line.top {
        line.top - y
    } else if y > line.bottom {
        y - line.bottom
    } else {
        0.0
    }
}

/// Position one line up or down from `pos`, keeping `x`.
fn vertical(layout: &FrameLayout, pos: TextPos, x: f32, down: bool) -> Option<TextPos> {
    let line = line_of(layout, pos)?;
    let target = if down { line + 1 } else { line.checked_sub(1)? };
    let line = layout.lines.get(target)?;
    let mid = (line.top + line.bottom) / 2.0;
    Some(hit_test(layout, x, mid))
}

/// Selection highlight rectangles `[x0, y0, x1, y1]`, document points.
pub fn selection_rects(layout: &FrameLayout, a: TextPos, b: TextPos) -> Vec<[f32; 4]> {
    let (start, end) = if a <= b { (a, b) } else { (b, a) };
    let mut rects = Vec::new();
    for line in &layout.lines {
        let line_start = TextPos::new(line.para, line.start);
        let line_end = TextPos::new(line.para, line.end);
        if line_end < start || line_start > end || (line_end == start && line.end > line.start) {
            continue;
        }
        let from = if start > line_start { start.offset - line.start } else { 0 };
        let to = if end < line_end { end.offset - line.start } else { line.end - line.start };
        let (x0, mut x1) = (line.xs[from.min(line.xs.len() - 1)], line.xs[to.min(line.xs.len() - 1)]);
        // When the selection continues past this line, show a small block for the
        // line/paragraph break, as text editors do
        if end > line_end {
            x1 += 4.0;
        }
        if x1 > x0 {
            rects.push([x0, line.top, x1, line.bottom]);
        }
    }
    rects
}

// ----------------------------------------------------------------------- frame geometry

/// The frame `id`'s text is laid out in: a text box, or, when `id` is the
/// table being edited, the edited cell as a text box placed on the page.
fn frame(state: &AppState, id: ObjectId) -> Option<Cow<'_, Shape>> {
    let shape = state.current_page()?.get_shape(id)?;
    match &shape.kind {
        ShapeKind::TextFrame(_) => Some(Cow::Borrowed(shape)),
        ShapeKind::Table(_) => {
            let (row, col) = edited_cell(state, id)?;
            crate::table_view::cell_frame(shape, row, col).map(Cow::Owned)
        }
        _ => None,
    }
}

/// The cell of table `id` being edited, if any.
fn edited_cell(state: &AppState, id: ObjectId) -> Option<(usize, usize)> {
    state.text_edit.as_ref().filter(|e| e.frame == id).and_then(|e| e.cell)
}

/// `id`'s text: a text box's, or the edited cell's when `id` is the table being edited.
pub(crate) fn text_ref(state: &AppState, id: ObjectId) -> Option<&StyledText> {
    let cell = edited_cell(state, id);
    match &state.current_page()?.get_shape(id)?.kind {
        ShapeKind::TextFrame(tf) => Some(&tf.content),
        ShapeKind::Table(table) => {
            let (row, col) = cell?;
            table.cell(row as u32, col as u32).map(|c| &c.content)
        }
        _ => None,
    }
}

/// Mutable [`text_ref`].
pub(crate) fn text_mut(state: &mut AppState, id: ObjectId) -> Option<&mut StyledText> {
    let cell = edited_cell(state, id);
    match &mut state.current_page_mut()?.get_shape_mut(id)?.kind {
        ShapeKind::TextFrame(tf) => Some(&mut tf.content),
        ShapeKind::Table(table) => {
            let (row, col) = cell?;
            table.cell_mut(row as u32, col as u32).map(|c| &mut c.content)
        }
        _ => None,
    }
}

/// The texts of object `id`: a text box's, or a table's cell `cell` (every
/// cell when `cell` is `None`).
pub(crate) fn texts_in(state: &AppState, id: ObjectId, cell: Option<(usize, usize)>) -> Vec<&StyledText> {
    match state.current_page().and_then(|p| p.get_shape(id)).map(|s| &s.kind) {
        Some(ShapeKind::TextFrame(tf)) => vec![&tf.content],
        Some(ShapeKind::Table(table)) => match cell {
            Some((row, col)) => table.cell(row as u32, col as u32).map(|c| &c.content).into_iter().collect(),
            None => table.cells.iter().map(|c| &c.content).collect(),
        },
        _ => Vec::new(),
    }
}

/// Run `f` on each of [`texts_in`], mutably.
pub(crate) fn for_each_text_mut(state: &mut AppState, id: ObjectId, cell: Option<(usize, usize)>, mut f: impl FnMut(&mut StyledText)) {
    match state.current_page_mut().and_then(|p| p.get_shape_mut(id)).map(|s| &mut s.kind) {
        Some(ShapeKind::TextFrame(tf)) => f(&mut tf.content),
        Some(ShapeKind::Table(table)) => match cell {
            Some((row, col)) => {
                if let Some(c) = table.cell_mut(row as u32, col as u32) {
                    f(&mut c.content);
                }
            }
            None => table.cells.iter_mut().for_each(|c| f(&mut c.content)),
        },
        _ => {}
    }
}

/// Whether document point (`x`, `y`) is inside the text being edited (the
/// text box, or the edited cell).
pub fn edit_contains(state: &AppState, x: f32, y: f32) -> bool {
    let Some(edit) = &state.text_edit else { return false };
    frame(state, edit.frame).is_some_and(|tf| frame_contains(&tf, x, y))
}

/// Rotate a document point into the frame's unrotated coordinates.
pub fn to_frame_space(tf: &Shape, x: f32, y: f32) -> (f32, f32) {
    let b = tf.common.bounds;
    let (cx, cy) = ((b.origin.x + b.size.width / 2.0) as f32, (b.origin.y + b.size.height / 2.0) as f32);
    let (sin, cos) = (-(tf.common.rotation as f32)).to_radians().sin_cos();
    let (dx, dy) = (x - cx, y - cy);
    (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
}

/// Rotate an unrotated frame-space point into document coordinates.
pub fn from_frame_space(tf: &Shape, x: f32, y: f32) -> (f32, f32) {
    let b = tf.common.bounds;
    let (cx, cy) = ((b.origin.x + b.size.width / 2.0) as f32, (b.origin.y + b.size.height / 2.0) as f32);
    let (sin, cos) = (tf.common.rotation as f32).to_radians().sin_cos();
    let (dx, dy) = (x - cx, y - cy);
    (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
}

/// Whether a document point lies inside the (possibly rotated) frame.
pub fn frame_contains(tf: &Shape, x: f32, y: f32) -> bool {
    let (fx, fy) = to_frame_space(tf, x, y);
    let b = tf.common.bounds;
    fx >= b.origin.x as f32 && fx <= (b.origin.x + b.size.width) as f32 && fy >= b.origin.y as f32 && fy <= (b.origin.y + b.size.height) as f32
}

// ----------------------------------------------------------------------- session

/// Whether `id` is a text frame on the current page.
pub fn is_text_frame(state: &AppState, id: ObjectId) -> bool {
    frame(state, id).is_some()
}

/// Start editing `id` with the caret at `caret` (or the end of the text).
pub fn begin(state: &mut AppState, id: ObjectId, caret: Option<TextPos>, now: f64) {
    // A later box of linked text boxes edits the story in the first box: the
    // caret (given within this box) moves to the same place in the story
    if let Some(page) = state.current_page() {
        let head = crate::text_chain::head(page, id);
        if head != id {
            let start = crate::text_chain::layout_chains(page).get(&id).map_or(TextPos::new(0, 0), |c| c.start);
            let caret = Some(caret.map_or(start, |c| crate::text_chain::to_story(start, c)));
            return begin(state, head, caret, now);
        }
    }
    if state.text_edit.as_ref().is_some_and(|e| e.frame == id) {
        return;
    }
    end(state);
    let Some(tf) = frame(state, id) else { return };
    let caret = clamp(frame_content(&tf), caret.unwrap_or_else(|| end_pos(frame_content(&tf))));
    state.select(id, false);
    state.begin_edit();
    state.text_edit = Some(TextEditState { frame: id, cell: None, caret, anchor: caret, preferred_x: None, blink_epoch: now });
}

/// Start editing cell (`row`, `col`) of table `id` with the caret at `caret`
/// (or the end of its text). Within one table, moving between cells continues
/// the same editing session (one undo step).
pub fn begin_cell(state: &mut AppState, id: ObjectId, row: usize, col: usize, caret: Option<TextPos>, now: f64) {
    let same_table = state.text_edit.as_ref().is_some_and(|e| e.frame == id && e.cell.is_some());
    if !same_table {
        end(state);
        if !matches!(state.current_page().and_then(|p| p.get_shape(id)).map(|s| &s.kind), Some(ShapeKind::Table(_))) {
            return;
        }
        state.select(id, false);
        state.begin_edit();
    }
    let session = TextEditState { frame: id, cell: Some((row, col)), caret: TextPos::default(), anchor: TextPos::default(), preferred_x: None, blink_epoch: now };
    state.text_edit = Some(session);
    let Some(text) = text_ref(state, id) else {
        // No such cell
        if !same_table {
            state.text_edit = None;
            state.end_edit("Typing");
        }
        return;
    };
    let caret = clamp(text, caret.unwrap_or_else(|| end_pos(text)));
    if let Some(edit) = state.text_edit.as_mut() {
        edit.caret = caret;
        edit.anchor = caret;
    }
}

/// Start editing the cell of table `id` under document point (`x`, `y`), with
/// the caret at the point.
pub fn begin_cell_at_point(state: &mut AppState, id: ObjectId, x: f32, y: f32, now: f64) -> bool {
    let Some(shape) = state.current_page().and_then(|p| p.get_shape(id)) else { return false };
    if !matches!(shape.kind, ShapeKind::Table(_)) { return false }
    // TODO(port): assumes table_view::cell_at / cell_frame take the table `Shape`.
    let Some((row, col)) = crate::table_view::cell_at(shape, repub_geometry::Point::new(x as f64, y as f64)) else { return false };
    let caret = crate::table_view::cell_frame(shape, row, col).map(|tf| {
        let (fx, fy) = to_frame_space(&tf, x, y);
        hit_test(&sdf_text::layout_frame(&tf), fx, fy)
    });
    begin_cell(state, id, row, col, caret, now);
    true
}

/// Tab / Shift+Tab in a table: edit the next (or previous) cell, its text
/// selected. Tab in the last cell adds a row, as in Publisher.
pub fn next_cell(state: &mut AppState, forward: bool, now: f64) {
    let Some(TextEditState { frame: id, cell: Some((row, col)), .. }) = state.text_edit.clone() else { return };
    let neighbour = {
        let Some(shape) = state.current_page().and_then(|p| p.get_shape(id)) else { return };
        if !matches!(shape.kind, ShapeKind::Table(_)) { return }
        crate::table_view::neighbour_cell(shape, row, col, forward)
    };
    let target = match neighbour {
        Some(rc) => rc,
        None if forward => {
            if let Some(shape) = state.current_page_mut().and_then(|p| p.get_shape_mut(id)) {
                if let ShapeKind::Table(t) = &mut shape.kind {
                    let before = t.total_height().max(1.0);
                    t.add_row();
                    let factor = t.total_height() / before;
                    shape.common.bounds.size.height *= factor;
                }
            }
            state.mark_dirty();
            (row + 1, 0)
        }
        None => return,
    };
    begin_cell(state, id, target.0, target.1, None, now);
    // The cell's text selected, so typing replaces it
    if let Some(edit) = state.text_edit.as_mut() {
        edit.anchor = TextPos::default();
    }
}

/// Start editing with the caret at a document point inside the frame.
pub fn begin_at_point(state: &mut AppState, id: ObjectId, x: f32, y: f32, now: f64) {
    // Linked boxes: the part of the story this box shows
    let shown = state.current_page().and_then(|p| crate::text_chain::layout_chains(p).remove(&id)).map(|c| c.frame);
    let caret = shown.map(Cow::Owned).or_else(|| frame(state, id)).map(|tf| {
        let (fx, fy) = to_frame_space(&tf, x, y);
        hit_test(&sdf_text::layout_frame(&tf), fx, fy)
    });
    begin(state, id, caret, now);
}

/// Stop editing; the session becomes one undo step. The frame stays selected.
pub fn end(state: &mut AppState) {
    if state.text_edit.take().is_some() {
        state.end_edit("Typing");
    }
}

/// Run `f` on the edited frame's text and session.
fn with_text<R>(state: &mut AppState, f: impl FnOnce(&mut StyledText, &mut TextEditState) -> R) -> Option<R> {
    let mut edit = state.text_edit.clone()?;
    let text = text_mut(state, edit.frame)?;
    let result = f(text, &mut edit);
    state.text_edit = Some(edit);
    state.mark_dirty();
    Some(result)
}

/// Replace the selection (if any) with `s`.
pub fn type_text(state: &mut AppState, s: &str, now: f64) {
    if s.is_empty() {
        return;
    }
    with_text(state, |text, edit| {
        let at = delete_range(text, edit.caret, edit.anchor);
        let caret = insert_text(text, at, s);
        set_caret(edit, caret, false, now);
    });
}

/// Undo (or redo) while editing: the typing so far becomes one undo step, it is
/// undone, and editing resumes in the same frame if it still exists.
pub fn undo(state: &mut AppState, redo: bool, now: f64) {
    let Some(edit) = state.text_edit.clone() else { return };
    end(state);
    if redo { state.redo() } else { state.undo() };
    match edit.cell {
        Some((row, col)) => begin_cell(state, edit.frame, row, col, Some(edit.caret), now),
        None if is_text_frame(state, edit.frame) => begin(state, edit.frame, Some(edit.caret), now),
        None => {}
    }
}

/// Return: split the paragraph.
pub fn newline(state: &mut AppState, now: f64) {
    with_text(state, |text, edit| {
        let at = delete_range(text, edit.caret, edit.anchor);
        let caret = split_paragraph(text, at);
        set_caret(edit, caret, false, now);
    });
}

/// Shift+Return: a line break inside the paragraph.
pub fn line_break(state: &mut AppState, now: f64) {
    insert_element(state, InlineElement::LineBreak, now);
}

/// Put a field (page number, date, ...) at the caret, replacing the selection.
pub fn insert_field(state: &mut AppState, field: repub_model::text::Field, now: f64) {
    insert_element(state, InlineElement::Field(field), now);
}

fn insert_element(state: &mut AppState, element: InlineElement, now: f64) {
    with_text(state, |text, edit| {
        let at = delete_range(text, edit.caret, edit.anchor);
        ensure_paragraph(text);
        let at = clamp(text, at);
        insert_inline(&mut text.paragraphs[at.para], at.offset, element);
        set_caret(edit, TextPos::new(at.para, at.offset + 1), false, now);
    });
}

/// Backspace: delete the selection or the character before the caret (`word`: the previous word).
pub fn backspace(state: &mut AppState, word: bool, now: f64) {
    with_text(state, |text, edit| {
        let (from, to) = if edit.has_selection() {
            (edit.caret, edit.anchor)
        } else {
            let to = clamp(text, edit.caret);
            (if word { prev_word(text, to) } else { prev_pos(text, to) }, to)
        };
        let caret = delete_range(text, from, to);
        set_caret(edit, caret, false, now);
    });
}

/// Delete: delete the selection or the character after the caret (`word`: the next word).
pub fn delete_forward(state: &mut AppState, word: bool, now: f64) {
    with_text(state, |text, edit| {
        let (from, to) = if edit.has_selection() {
            (edit.caret, edit.anchor)
        } else {
            let from = clamp(text, edit.caret);
            (from, if word { next_word(text, from) } else { next_pos(text, from) })
        };
        let caret = delete_range(text, from, to);
        set_caret(edit, caret, false, now);
    });
}

/// Selected text (empty if nothing is selected).
pub fn selected_text(state: &AppState) -> String {
    let Some(edit) = &state.text_edit else { return String::new() };
    frame(state, edit.frame).map_or_else(String::new, |tf| text_between(frame_content(&tf), edit.caret, edit.anchor))
}

/// Select all text in the frame.
pub fn select_all(state: &mut AppState, now: f64) {
    let Some(end) = state.text_edit.as_ref().and_then(|e| frame(state, e.frame)).map(|tf| end_pos(frame_content(&tf))) else { return };
    if let Some(edit) = state.text_edit.as_mut() {
        edit.anchor = TextPos::default();
        edit.caret = end;
        edit.blink_epoch = now;
    }
}

fn set_caret(edit: &mut TextEditState, caret: TextPos, extend: bool, now: f64) {
    edit.caret = caret;
    if !extend {
        edit.anchor = caret;
    }
    edit.blink_epoch = now;
}

/// Caret movement keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Motion {
    Left,
    Right,
    WordLeft,
    WordRight,
    Up,
    Down,
    LineStart,
    LineEnd,
    DocStart,
    DocEnd,
}

/// Move the caret; `extend` (Shift) grows the selection instead of collapsing it.
pub fn move_caret(state: &mut AppState, motion: Motion, extend: bool, now: f64) {
    let Some(edit) = state.text_edit.clone() else { return };
    let Some(tf) = frame(state, edit.frame) else { return };
    let text = frame_content(&tf);
    let layout = sdf_text::layout_frame(&tf);
    let caret = clamp(text, edit.caret);
    let (start, end) = edit.range();
    let mut preferred_x = None;

    let target = match motion {
        // Without Shift, Left/Right collapse an existing selection to its edge
        Motion::Left if !extend && edit.has_selection() => start,
        Motion::Right if !extend && edit.has_selection() => end,
        Motion::Left => prev_pos(text, caret),
        Motion::Right => next_pos(text, caret),
        Motion::WordLeft => prev_word(text, caret),
        Motion::WordRight => next_word(text, caret),
        Motion::Up | Motion::Down => {
            let x = edit.preferred_x.or_else(|| caret_geometry(&layout, caret).map(|g| g.0)).unwrap_or(0.0);
            preferred_x = Some(x);
            let down = motion == Motion::Down;
            vertical(&layout, caret, x, down).unwrap_or(if down { end_pos(text) } else { TextPos::default() })
        }
        Motion::LineStart => line_of(&layout, caret).map_or(caret, |i| TextPos::new(caret.para, layout.lines[i].start)),
        Motion::LineEnd => line_of(&layout, caret).map_or(caret, |i| {
            let line = &layout.lines[i];
            // Stop before a trailing space at a soft wrap so the caret stays on this line
            let chars = para_chars(&text.paragraphs[caret.para]);
            let wrapped_space = line.end > line.start && line.end < chars.len() && chars[line.end - 1] == ' ';
            TextPos::new(caret.para, if wrapped_space { line.end - 1 } else { line.end })
        }),
        Motion::DocStart => TextPos::default(),
        Motion::DocEnd => end_pos(text),
    };

    if let Some(edit) = state.text_edit.as_mut() {
        set_caret(edit, target, extend, now);
        edit.preferred_x = preferred_x;
    }
}

/// Mouse press inside the edited frame: place the caret (`extend`: Shift-click extends).
pub fn click_at(state: &mut AppState, x: f32, y: f32, extend: bool, now: f64) {
    let Some(edit) = state.text_edit.clone() else { return };
    let Some(tf) = frame(state, edit.frame) else { return };
    let (fx, fy) = to_frame_space(&tf, x, y);
    let pos = hit_test(&sdf_text::layout_frame(&tf), fx, fy);
    if let Some(edit) = state.text_edit.as_mut() {
        set_caret(edit, pos, extend, now);
        edit.preferred_x = None;
    }
}

/// Mouse drag inside the edited frame: extend the selection to the pointer.
pub fn drag_to(state: &mut AppState, x: f32, y: f32, now: f64) {
    click_at(state, x, y, true, now);
}

/// Double-click: select the word under the pointer.
pub fn select_word_at(state: &mut AppState, x: f32, y: f32, now: f64) {
    let Some(edit) = state.text_edit.clone() else { return };
    let Some(tf) = frame(state, edit.frame) else { return };
    let (fx, fy) = to_frame_space(&tf, x, y);
    let pos = hit_test(&sdf_text::layout_frame(&tf), fx, fy);
    let (a, b) = word_at(frame_content(&tf), pos);
    if let Some(edit) = state.text_edit.as_mut() {
        edit.anchor = a;
        edit.caret = b;
        edit.blink_epoch = now;
    }
}

/// Edited frame, its layout, and the session, for drawing caret and selection.
pub fn draw_info(state: &AppState) -> Option<(Cow<'_, Shape>, FrameLayout, &TextEditState)> {
    let edit = state.text_edit.as_ref()?;
    let tf = frame(state, edit.frame)?;
    let layout = sdf_text::layout_frame(&tf);
    Some((tf, layout, edit))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn para(s: &str) -> Paragraph {
        Paragraph::plain(s)
    }

    fn text(paras: &[&str]) -> StyledText {
        StyledText { paragraphs: paras.iter().map(|s| para(s)).collect() }
    }

    /// A text-frame [`Shape`] at `rect` with the given content.
    fn text_frame(rect: repub_geometry::Rect, content: StyledText) -> Shape {
        let mut shape = Shape::text_frame(rect);
        if let ShapeKind::TextFrame(tf) = &mut shape.kind {
            tf.content = content;
        }
        shape
    }

    fn plain(t: &StyledText) -> Vec<String> {
        t.paragraphs.iter().map(|p| para_chars(p).into_iter().collect()).collect()
    }

    #[test]
    fn a_caret_in_or_beside_a_link_finds_it() {
        use repub_model::text::Hyperlink;
        let mut p = Paragraph::new();
        p.elements.push(InlineElement::Text(TextRun::new("ab")));
        p.elements.push(InlineElement::Link(Hyperlink::external("https://example.com", "cd")));
        p.elements.push(InlineElement::Text(TextRun::new("ef")));
        let text = StyledText { paragraphs: vec![p] };
        for offset in [2, 3, 4] {
            let (start, end, link) = link_around(&text, TextPos::new(0, offset)).expect("inside or at the edges");
            assert_eq!((start.offset, end.offset), (2, 4));
            assert_eq!(link.target, "https://example.com");
        }
        assert!(link_around(&text, TextPos::new(0, 1)).is_none());
        assert!(link_around(&text, TextPos::new(0, 5)).is_none());
    }

    #[test]
    fn line_breaks_and_fields_split_a_run_and_count_as_one_character() {
        let mut p = para("abcd");
        insert_inline(&mut p, 2, InlineElement::LineBreak);
        insert_inline(&mut p, 3, InlineElement::Field(repub_model::text::Field::page_count()));
        assert_eq!(para_len(&p), 6);
        let kinds: Vec<&str> = p
            .elements
            .iter()
            .map(|e| match e {
                InlineElement::Text(_) => "text",
                InlineElement::LineBreak => "break",
                InlineElement::Field(_) => "field",
                _ => "other",
            })
            .collect();
        assert_eq!(kinds, ["text", "break", "field", "text"]);
    }

    #[test]
    fn insert_into_empty_creates_paragraph() {
        let mut t = StyledText { paragraphs: vec![] };
        let caret = insert_text(&mut t, TextPos::default(), "Hi");
        assert_eq!(plain(&t), ["Hi"]);
        assert_eq!(caret, TextPos::new(0, 2));
    }

    #[test]
    fn insert_in_middle_and_multiline() {
        let mut t = text(&["Helo"]);
        let c = insert_text(&mut t, TextPos::new(0, 3), "l");
        assert_eq!(plain(&t), ["Hello"]);
        assert_eq!(c, TextPos::new(0, 4));
        let c = insert_text(&mut t, TextPos::new(0, 5), " a\nb\nc");
        assert_eq!(plain(&t), ["Hello a", "b", "c"]);
        assert_eq!(c, TextPos::new(2, 1));
    }

    #[test]
    fn delete_across_paragraphs_joins_them() {
        let mut t = text(&["one", "two", "three"]);
        let c = delete_range(&mut t, TextPos::new(0, 2), TextPos::new(2, 2));
        assert_eq!(plain(&t), ["onree"]);
        assert_eq!(c, TextPos::new(0, 2));
    }

    #[test]
    fn split_and_rejoin() {
        let mut t = text(&["abcd"]);
        let c = split_paragraph(&mut t, TextPos::new(0, 2));
        assert_eq!(plain(&t), ["ab", "cd"]);
        assert_eq!(c, TextPos::new(1, 0));
        // Backspace at start of the second paragraph joins them
        let from = prev_pos(&t, c);
        delete_range(&mut t, from, c);
        assert_eq!(plain(&t), ["abcd"]);
    }

    #[test]
    fn style_continues_from_previous_run() {
        let mut p = Paragraph::new();
        let mut bold = p.style.default_char_style.clone();
        bold.font.weight = repub_model::styles::FontWeight::Bold;
        p.elements.push(InlineElement::Text(TextRun::styled("Bold", bold.clone())));
        let mut t = StyledText { paragraphs: vec![p] };
        insert_text(&mut t, TextPos::new(0, 4), "er");
        match &t.paragraphs[0].elements[0] {
            InlineElement::Text(run) => {
                assert_eq!(run.text, "Bolder");
                assert_eq!(run.style.font.weight, repub_model::styles::FontWeight::Bold);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn unicode_is_handled_per_character() {
        let mut t = text(&["café"]);
        let c = insert_text(&mut t, TextPos::new(0, 4), "!");
        assert_eq!(plain(&t), ["café!"]);
        delete_range(&mut t, TextPos::new(0, 3), TextPos::new(0, 4));
        assert_eq!(plain(&t), ["caf!"]);
        assert_eq!(c.offset, 5);
    }

    #[test]
    fn word_motion_and_word_selection() {
        let t = text(&["hello brave  world"]);
        assert_eq!(next_word(&t, TextPos::new(0, 0)), TextPos::new(0, 5));
        assert_eq!(next_word(&t, TextPos::new(0, 5)), TextPos::new(0, 11));
        assert_eq!(prev_word(&t, TextPos::new(0, 18)), TextPos::new(0, 13));
        assert_eq!(word_at(&t, TextPos::new(0, 7)), (TextPos::new(0, 6), TextPos::new(0, 11)));
    }

    #[test]
    fn text_between_spans_paragraphs() {
        let t = text(&["one", "two"]);
        assert_eq!(text_between(&t, TextPos::new(0, 1), TextPos::new(1, 2)), "ne\ntw");
    }

    #[test]
    fn layout_offsets_round_trip_through_hit_testing() {
        let tf = text_frame(repub_geometry::Rect::new(0.0, 0.0, 300.0, 200.0), text(&["The quick brown fox", "jumps"]));
        let layout = sdf_text::layout_frame(&tf);
        for (para, len) in [(0usize, 19usize), (1, 5)] {
            for offset in 0..=len {
                let pos = TextPos::new(para, offset);
                let (x, top, bottom) = caret_geometry(&layout, pos).expect("caret exists");
                assert_eq!(hit_test(&layout, x, (top + bottom) / 2.0), pos, "offset {offset} of paragraph {para}");
            }
        }
    }

    #[test]
    fn wrapped_lines_cover_every_offset() {
        let tf = text_frame(repub_geometry::Rect::new(0.0, 0.0, 60.0, 400.0), text(&["alpha beta gamma delta epsilon"]));
        let layout = sdf_text::layout_frame(&tf);
        assert!(layout.lines.len() > 2, "narrow frame wraps");
        for offset in 0..=30 {
            assert!(caret_geometry(&layout, TextPos::new(0, offset)).is_some(), "offset {offset}");
        }
    }

    #[test]
    fn rotated_frame_point_mapping_round_trips() {
        let mut tf = Shape::text_frame(repub_geometry::Rect::new(100.0, 100.0, 200.0, 50.0));
        tf.common.rotation = 30.0;
        let (x, y) = from_frame_space(&tf, 120.0, 110.0);
        let (fx, fy) = to_frame_space(&tf, x, y);
        assert!((fx - 120.0).abs() < 1e-3 && (fy - 110.0).abs() < 1e-3);
        assert!(frame_contains(&tf, x, y));
    }
}
