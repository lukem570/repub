//! Text editing engine operating on [`StyledText`].
//!
//! These are the framework-free building blocks of in-place text editing:
//! cursor positions, inserting and deleting text, splitting paragraphs, and
//! word/line navigation. Laying out a frame and mapping screen points to caret
//! positions needs a text layout engine and is a rendering concern, so caret
//! geometry and hit-testing against a laid-out frame live with the renderer.
//!
//! Character offsets: one per character of a text run or link, and one per
//! tab, field, line break or inline image.

use repub_geometry::Rect;
use repub_model::shape::Shape;
use repub_model::text::{Hyperlink, InlineElement, Paragraph, StyledText, TextRun};

/// A caret position: paragraph index and character offset within it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct TextPos {
    /// Paragraph index.
    pub para: usize,
    /// Character offset within the paragraph.
    pub offset: usize,
}

impl TextPos {
    /// Create a new text position.
    pub fn new(para: usize, offset: usize) -> Self {
        Self { para, offset }
    }
}

/// An editing session on one text frame.
#[derive(Debug, Clone)]
pub struct TextEditState {
    /// The text box, or the table whose `cell` is being edited.
    pub frame: repub_model::id::ObjectId,
    /// Table cell being edited (row, column), when `frame` is a table.
    pub cell: Option<(usize, usize)>,
    /// The caret position.
    pub caret: TextPos,
    /// Other end of the selection; equal to `caret` when nothing is selected.
    pub anchor: TextPos,
    /// Remembered x (document points) for vertical caret movement.
    pub preferred_x: Option<f32>,
    /// Time the caret last moved, so it stays solid while typing.
    pub blink_epoch: f64,
}

impl TextEditState {
    /// Whether there is a non-empty selection.
    pub fn has_selection(&self) -> bool {
        self.caret != self.anchor
    }

    /// Selection as an ordered (start, end) pair.
    pub fn range(&self) -> (TextPos, TextPos) {
        if self.caret <= self.anchor {
            (self.caret, self.anchor)
        } else {
            (self.anchor, self.caret)
        }
    }
}

/// The number of character offsets an inline element occupies.
pub fn element_len(e: &InlineElement) -> usize {
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
            InlineElement::Link(link) if offset > start && offset < start + len => {
                let at = byte_index(&link.text, offset - start);
                link.text.insert_str(at, text);
                return;
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

/// Delete the text between two positions (any order). Returns the caret at the
/// start of the deleted range.
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
                insert_tab(&mut text.paragraphs[pos.para], pos.offset);
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
pub fn split_run_at(p: &mut Paragraph, offset: usize) {
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

/// Insert a hyperlink element at `pos`. Returns the caret after the link text.
pub fn insert_link(text: &mut StyledText, pos: TextPos, link: Hyperlink) -> TextPos {
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

/// Insert a tab element at `offset`, splitting a text run if needed.
fn insert_tab(p: &mut Paragraph, offset: usize) {
    let mut pos = 0;
    for i in 0..p.elements.len() {
        let len = element_len(&p.elements[i]);
        if offset <= pos {
            p.elements.insert(i, InlineElement::Tab);
            return;
        }
        if offset < pos + len {
            if let InlineElement::Text(run) = &mut p.elements[i] {
                let at = byte_index(&run.text, offset - pos);
                let tail = TextRun::styled(&run.text[at..], run.style.clone());
                run.text.truncate(at);
                p.elements.insert(i + 1, InlineElement::Tab);
                p.elements.insert(i + 2, InlineElement::Text(tail));
                return;
            }
        }
        pos += len;
    }
    p.elements.push(InlineElement::Tab);
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
    let target = if b < chars.len() {
        class(chars[b])
    } else if a > 0 {
        class(chars[a - 1])
    } else {
        true
    };
    while a > 0 && class(chars[a - 1]) == target && chars[a - 1] != '\n' {
        a -= 1;
    }
    while b < chars.len() && class(chars[b]) == target && chars[b] != '\n' {
        b += 1;
    }
    (TextPos::new(pos.para, a), TextPos::new(pos.para, b))
}

/// Rotate a document point into the text frame's unrotated coordinates.
pub fn to_frame_space(shape: &Shape, x: f32, y: f32) -> (f32, f32) {
    let b = shape.common.bounds;
    let (cx, cy) = ((b.origin.x + b.size.width / 2.0) as f32, (b.origin.y + b.size.height / 2.0) as f32);
    let (sin, cos) = (-(shape.common.rotation as f32)).to_radians().sin_cos();
    let (dx, dy) = (x - cx, y - cy);
    (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
}

/// Rotate an unrotated frame-space point into document coordinates.
pub fn from_frame_space(shape: &Shape, x: f32, y: f32) -> (f32, f32) {
    let b = shape.common.bounds;
    let (cx, cy) = ((b.origin.x + b.size.width / 2.0) as f32, (b.origin.y + b.size.height / 2.0) as f32);
    let (sin, cos) = (shape.common.rotation as f32).to_radians().sin_cos();
    let (dx, dy) = (x - cx, y - cy);
    (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
}

/// Whether a document point lies inside the (possibly rotated) frame bounds.
pub fn frame_contains(shape: &Shape, x: f32, y: f32) -> bool {
    let (fx, fy) = to_frame_space(shape, x, y);
    let b = shape.common.bounds;
    fx >= b.origin.x as f32
        && fx <= (b.origin.x + b.size.width) as f32
        && fy >= b.origin.y as f32
        && fy <= (b.origin.y + b.size.height) as f32
}

/// Whether a rectangle (document points) contains a point.
pub fn rect_contains(r: Rect, x: f64, y: f64) -> bool {
    x >= r.origin.x && x <= r.origin.x + r.size.width && y >= r.origin.y && y <= r.origin.y + r.size.height
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

    fn plain(t: &StyledText) -> Vec<String> {
        t.paragraphs.iter().map(|p| para_chars(p).into_iter().collect()).collect()
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
    fn insert_link_round_trips() {
        let mut t = text(&["ab"]);
        let link = Hyperlink::external("https://example.com", "link");
        let caret = insert_link(&mut t, TextPos::new(0, 1), link);
        assert_eq!(caret, TextPos::new(0, 5));
        assert!(matches!(&t.paragraphs[0].elements[1], InlineElement::Link(_)));
    }

    #[test]
    fn rotated_frame_point_mapping_round_trips() {
        let mut tf = Shape::text_frame(Rect::new(100.0, 100.0, 200.0, 50.0));
        tf.common.rotation = 30.0;
        let (x, y) = from_frame_space(&tf, 120.0, 110.0);
        let (fx, fy) = to_frame_space(&tf, x, y);
        assert!((fx - 120.0).abs() < 1e-3 && (fy - 110.0).abs() < 1e-3);
        assert!(frame_contains(&tf, x, y));
    }
}
