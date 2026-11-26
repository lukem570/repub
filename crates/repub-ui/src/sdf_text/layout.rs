//! Text frame layout for SDF rendering and text editing.
//!
//! Produces glyph and underline quads plus caret geometry, in document points
//! (unrotated, relative to the page). The renderer applies the frame's rotation
//! and zoom; the text editor uses the caret lines for hit-testing and drawing
//! the caret and selection.
//!
//! Character offsets follow the edit model in `repub_edit::text_edit`: one
//! offset per character of a text run or link, and one per tab, field, line
//! break or image.
//!
//! Frame geometry (bounds, rotation, id) lives on the enclosing
//! [`repub_model::shape::Shape`]; the text content and layout settings live on
//! its [`repub_model::shape::TextFrameData`]. The layout functions take the
//! `Shape` and read both.

#![allow(missing_docs)]

use std::cell::Cell;

use repub_geometry::Rect;
use repub_model::shape::{Shape, ShapeKind, TextFrameData, VerticalAlignment};
use repub_model::styles::{
    CharacterStyle, LineSpacing, StrikethroughStyle, TextAlignment, UnderlineStyle,
};
use repub_model::text::{Field, InlineElement, Paragraph};
use repub_model::ObjectId;

use super::atlas::{Atlas, Face, REGULAR};

// Current page number and total page count, used to resolve `Field`s during layout.
thread_local! {
    static PAGE_CONTEXT: Cell<(u32, u32)> = const { Cell::new((1, 1)) };
}

/// Set the page context used when resolving page-number fields in text.
pub fn set_page_context(page_number: u32, page_count: u32) {
    PAGE_CONTEXT.with(|c| c.set((page_number, page_count)));
}

thread_local! {
    static FILE_NAME: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
}

/// Set the document name that file-name fields show.
pub fn set_file_name(name: &str) {
    FILE_NAME.with(|n| {
        if *n.borrow() != name {
            *n.borrow_mut() = name.to_string();
        }
    });
}

/// The local date or time now in `format` (strftime), or in `fallback` when `format` isn't valid.
fn formatted_now(format: &str, fallback: &str) -> String {
    use chrono::format::{Item, StrftimeItems};
    let valid = !format.is_empty() && !StrftimeItems::new(format).any(|i| matches!(i, Item::Error));
    chrono::Local::now()
        .format(if valid { format } else { fallback })
        .to_string()
        .trim()
        .to_string()
}

fn current_page_context() -> (u32, u32) {
    PAGE_CONTEXT.with(|c| c.get())
}

/// Text a field should display in the current page context.
fn field_text(field: &Field) -> String {
    let (page, count) = current_page_context();
    match field {
        Field::PageNumber => page.to_string(),
        Field::PageCount => count.to_string(),
        Field::Date { format } => formatted_now(format, "%B %e, %Y"),
        Field::Time { format } => formatted_now(format, "%l:%M %p"),
        Field::FileName => FILE_NAME.with(|n| n.borrow().clone()),
        Field::Custom { default, .. } => default.clone(),
    }
}

/// Slant applied to italic/oblique text (Selawik has no italic face): x shift per unit of height.
const SYNTHETIC_ITALIC: f32 = 0.2;
/// A tab advances by this many spaces.
const TAB_SPACES: f32 = 4.0;

/// One textured quad in document points.
#[derive(Debug, Clone, Copy)]
pub struct Quad {
    /// `[x0, y0, x1, y1]`, document points, y down.
    pub rect: [f32; 4],
    pub uv: [f32; 4],
    /// Straight-alpha sRGB color, 0..1.
    pub color: [f32; 4],
    /// Baseline y used as the skew pivot (document points).
    pub baseline: f32,
    /// Horizontal shear factor (synthetic italic).
    pub skew: f32,
}

/// Caret geometry for one laid-out line.
#[derive(Debug, Clone)]
pub struct CaretLine {
    /// Paragraph index.
    pub para: usize,
    /// First character offset on the line.
    pub start: usize,
    /// Offset just past the last character on the line.
    pub end: usize,
    /// Caret x for each offset `start..=end` (document points).
    pub xs: Vec<f32>,
    /// Top and bottom of the caret / selection band.
    pub top: f32,
    pub bottom: f32,
    pub baseline: f32,
}

/// Result of laying out a text frame.
#[derive(Debug, Clone, Default)]
pub struct FrameLayout {
    pub quads: Vec<Quad>,
    /// Every line in reading order (at least one per paragraph).
    pub lines: Vec<CaretLine>,
}

/// A laid-out character before line placement.
#[derive(Clone, Copy)]
struct Item {
    c: char,
    /// Character offset within the paragraph.
    offset: usize,
    size: f32,
    /// Atlas face index
    face: usize,
    skew: f32,
    color: [f32; 4],
    underline: UnderlineStyle,
    underline_color: Option<[f32; 4]>,
    strikethrough: StrikethroughStyle,
    /// Highlight behind the character
    background: Option<[f32; 4]>,
    /// Advance in points, including letter spacing and kerning with the next char.
    advance: f32,
    is_space: bool,
    /// Forced line break after this item.
    hard_break: bool,
    /// Draws no glyph (line break, image placeholder).
    invisible: bool,
    /// Raise above the baseline in points (superscript > 0, subscript < 0).
    rise: f32,
}

fn color_of(c: &repub_color::Color) -> [f32; 4] {
    [c.r as f32, c.g as f32, c.b as f32, c.a as f32]
}

fn base_item(atlas: &Atlas, c: char, offset: usize, style: &CharacterStyle) -> Item {
    use repub_model::styles::BaselineShift;
    let size = style.font_size.max(0.5) as f32;
    // Superscript/subscript: smaller, raised or lowered (Publisher's proportions)
    let (size, rise) = match style.baseline_shift {
        BaselineShift::Normal => (size, 0.0),
        BaselineShift::Superscript => (size * 0.65, size * 0.33),
        BaselineShift::Subscript => (size * 0.65, -size * 0.14),
    };
    // The style's installed face (italic slanted synthetically if the face isn't)
    let (face, slant) = atlas.resolve(style);
    Item {
        c,
        offset,
        size,
        rise,
        face,
        skew: if slant { SYNTHETIC_ITALIC } else { 0.0 },
        color: color_of(&style.color),
        underline: style.underline,
        underline_color: style.underline_color.as_ref().map(color_of),
        strikethrough: style.strikethrough,
        background: style
            .background_color
            .as_ref()
            .filter(|c| c.a > 0.0)
            .map(color_of),
        advance: style.letter_spacing as f32,
        is_space: c.is_whitespace(),
        hard_break: false,
        invisible: false,
    }
}

/// A list marker (bullet or number) drawn before a paragraph's first line.
struct Marker {
    text: String,
    /// From the column's left edge, points.
    x: f32,
}

/// Number `n` in a list numbering format, with its punctuation ("3.", "c.", "iv.").
fn list_number(format: repub_model::styles::NumberFormat, n: u32) -> String {
    use repub_model::styles::NumberFormat;
    fn roman(mut n: u32) -> String {
        let table = [
            (1000, "m"),
            (900, "cm"),
            (500, "d"),
            (400, "cd"),
            (100, "c"),
            (90, "xc"),
            (50, "l"),
            (40, "xl"),
            (10, "x"),
            (9, "ix"),
            (5, "v"),
            (4, "iv"),
            (1, "i"),
        ];
        let mut out = String::new();
        for (value, digits) in table {
            while n >= value {
                out.push_str(digits);
                n -= value;
            }
        }
        out
    }
    fn alpha(n: u32) -> String {
        // a..z, aa..zz, ...
        let n = n.max(1) - 1;
        let letter = (b'a' + (n % 26) as u8) as char;
        letter.to_string().repeat((n / 26) as usize + 1)
    }
    let body = match format {
        NumberFormat::Arabic => n.to_string(),
        NumberFormat::LowerAlpha => alpha(n),
        NumberFormat::UpperAlpha => alpha(n).to_uppercase(),
        NumberFormat::LowerRoman => roman(n),
        NumberFormat::UpperRoman => roman(n).to_uppercase(),
    };
    format!("{body}.")
}

/// Indents and marker for a paragraph's list style. A hanging indent (negative
/// first-line indent) holds the marker; otherwise the text moves in by the
/// list indent and the marker sits at the paragraph's left edge. `counter`
/// carries numbering through consecutive numbered paragraphs.
/// Returns (left indent, first-line indent, marker).
fn list_layout(
    ps: &repub_model::styles::ParagraphStyle,
    counter: &mut Option<(repub_model::styles::NumberFormat, u32)>,
) -> (f32, f32, Option<Marker>) {
    use repub_model::styles::ListStyle;
    let (left, first) = (ps.left_indent as f32, ps.first_line_indent as f32);
    let (text, list_indent) = match &ps.list_style {
        ListStyle::None => {
            *counter = None;
            return (left, first, None);
        }
        ListStyle::Bullet { character, indent } => {
            *counter = None;
            (character.to_string(), *indent as f32)
        }
        ListStyle::Numbered {
            format,
            start,
            indent,
        } => {
            let n = match *counter {
                Some((f, n)) if f == *format => n + 1,
                _ => *start,
            };
            *counter = Some((*format, n));
            (list_number(*format, n), *indent as f32)
        }
    };
    if first < 0.0 {
        (
            left,
            0.0,
            Some(Marker {
                text,
                x: left + first,
            }),
        )
    } else {
        let hang = if list_indent > 0.0 { list_indent } else { 18.0 };
        (left + hang, 0.0, Some(Marker { text, x: left }))
    }
}

/// Characters of one paragraph with their resolved style. One item per character offset.
fn paragraph_items(atlas: &Atlas, para: &Paragraph) -> Vec<Item> {
    let mut items: Vec<Item> = Vec::new();
    let default = &para.style.default_char_style;
    let mut offset = 0;
    let push_text =
        |text: &str, style: &CharacterStyle, items: &mut Vec<Item>, offset: &mut usize| {
            for c in text.chars() {
                let upper = c.to_uppercase().next().unwrap_or(c);
                let small = style.small_caps && !style.all_caps && c.is_lowercase();
                let shown = if style.all_caps || small { upper } else { c };
                let mut item = base_item(atlas, shown, *offset, style);
                if small {
                    item.size *= 0.8;
                }
                items.push(item);
                *offset += 1;
            }
        };

    for element in &para.elements {
        match element {
            InlineElement::Text(run) => push_text(&run.text, &run.style, &mut items, &mut offset),
            InlineElement::Link(link) => {
                push_text(&link.text, &link.style, &mut items, &mut offset)
            }
            InlineElement::Field(field) => {
                push_text(&field_text(field), default, &mut items, &mut offset)
            }
            InlineElement::Tab => {
                let style = items
                    .last()
                    .map_or_else(|| base_item(atlas, ' ', 0, default), |i| *i);
                let mut item = base_item(atlas, ' ', offset, default);
                item.size = style.size;
                item.face = style.face;
                item.advance = f32::NAN; // marks a tab; resolved below
                items.push(item);
                offset += 1;
            }
            InlineElement::LineBreak => {
                let mut item = base_item(atlas, '\n', offset, default);
                if let Some(prev) = items.last() {
                    item.size = prev.size;
                    item.face = prev.face;
                }
                item.hard_break = true;
                item.invisible = true;
                items.push(item);
                offset += 1;
            }
            InlineElement::Image(_) => {
                let mut item = base_item(atlas, ' ', offset, default);
                item.invisible = true;
                items.push(item);
                offset += 1;
            }
        }
    }

    // Advances with kerning (only between characters of the same face and size)
    for i in 0..items.len() {
        let face = atlas.face(items[i].face);
        if items[i].c == '\n' {
            items[i].advance = 0.0;
            continue;
        }
        let tab = items[i].advance.is_nan();
        let glyph_advance = face.glyph(items[i].c).map_or(0.5, |g| g.advance);
        let mut advance = if tab {
            glyph_advance * TAB_SPACES
        } else {
            glyph_advance
        };
        if let Some(next) = items.get(i + 1) {
            if !tab && next.face == items[i].face && next.size == items[i].size {
                advance += face.kern(items[i].c, next.c);
            }
        }
        let spacing = if tab { 0.0 } else { items[i].advance };
        items[i].advance = spacing + advance * items[i].size;
    }
    items
}

/// A line: item range plus metrics.
struct Line {
    start: usize,
    end: usize,
    width: f32,
    ascent: f32,
    descent: f32,
    gap: f32,
    /// Ends the paragraph or a hard break (not justified).
    last: bool,
}

/// Greedy word wrap: the end of the line starting at `start` that fits in
/// `avail` points (at least one item, so long words still make progress).
fn break_one(items: &[Item], start: usize, avail: f32) -> usize {
    let mut x = 0.0;
    let mut end = start;
    let mut last_break: Option<usize> = None; // index after a space
    while end < items.len() {
        let item = items[end];
        if !item.is_space && x + item.advance > avail && end > start {
            break;
        }
        x += item.advance;
        end += 1;
        if item.is_space {
            last_break = Some(end);
        }
        if item.hard_break {
            break;
        }
    }
    // Prefer breaking after the last space when the line overflowed mid-word
    if end < items.len() && !items[end - 1].hard_break && !items[end].is_space {
        if let Some(b) = last_break {
            end = b;
        }
    }
    end
}

/// An area of the page text must flow around (text wrap), in document points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Exclusion {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    /// Top and Bottom wrap: no text beside it at all.
    pub full_width: bool,
    pub side: repub_model::styles::TextWrapSide,
}

thread_local! {
    /// Text wrap exclusions per text frame, set by the canvas for the pages it shows.
    static EXCLUSIONS: std::cell::RefCell<std::collections::HashMap<ObjectId, Vec<Exclusion>>> = Default::default();
}

/// Record the areas `frame`'s text wraps around (empty clears them).
pub fn set_exclusions(frame: ObjectId, exclusions: Vec<Exclusion>) {
    EXCLUSIONS.with(|e| {
        let mut e = e.borrow_mut();
        if exclusions.is_empty() {
            e.remove(&frame);
        } else {
            e.insert(frame, exclusions);
        }
    });
}

fn exclusions_for(frame: ObjectId) -> Vec<Exclusion> {
    EXCLUSIONS.with(|e| e.borrow().get(&frame).cloned().unwrap_or_default())
}

/// The free stretch of `[left, right]` for a line occupying `[top, bottom]`.
/// `Err(y)` when the line can't go here and should move down to `y`.
fn free_segment(
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    exclusions: &[Exclusion],
    min_width: f32,
) -> Result<(f32, f32), f32> {
    use repub_model::styles::TextWrapSide;
    let hits: Vec<&Exclusion> = exclusions
        .iter()
        .filter(|e| e.y0 < bottom && e.y1 > top && e.x0 < right && e.x1 > left)
        .collect();
    if hits.is_empty() {
        return Ok((left, right));
    }
    let below = hits.iter().map(|e| e.y1).fold(f32::INFINITY, f32::min);
    if hits.iter().any(|e| e.full_width) {
        return Err(below);
    }
    // Subtract each obstacle from the free intervals
    let mut free = vec![(left, right)];
    for e in &hits {
        free = free
            .into_iter()
            .flat_map(|(a, b)| {
                let mut parts = Vec::new();
                if e.x0 > a {
                    parts.push((a, e.x0.min(b)));
                }
                if e.x1 < b {
                    parts.push((e.x1.max(a), b));
                }
                parts
            })
            .collect();
    }
    free.retain(|(a, b)| b - a >= min_width);
    let pick = if hits.iter().all(|e| e.side == TextWrapSide::Left) {
        free.first().copied()
    } else if hits.iter().all(|e| e.side == TextWrapSide::Right) {
        free.last().copied()
    } else {
        // Both sides / largest side: the widest gap (one stretch per line)
        free.iter()
            .copied()
            .fold(None, |best: Option<(f32, f32)>, f| match best {
                Some(b) if b.1 - b.0 >= f.1 - f.0 => Some(b),
                _ => Some(f),
            })
    };
    pick.ok_or(below)
}

fn measure(
    atlas: &Atlas,
    items: &[Item],
    start: usize,
    end: usize,
    last: bool,
    default_size: f32,
) -> Line {
    let (mut ascent, mut descent, mut gap) = (0.0f32, 0.0f32, 0.0f32);
    for item in &items[start..end] {
        let face: &Face = atlas.face(item.face);
        ascent = ascent.max(face.ascent * item.size + item.rise);
        descent = descent.max(face.descent * item.size - item.rise);
        gap = gap.max(face.line_gap * item.size);
    }
    if start == end {
        let face = atlas.face(REGULAR);
        ascent = face.ascent * default_size;
        descent = face.descent * default_size;
        gap = face.line_gap * default_size;
    }
    // Trailing spaces and the line break don't count toward the width (alignment)
    let mut trimmed = end;
    while trimmed > start && items[trimmed - 1].is_space {
        trimmed -= 1;
    }
    let width = items[start..trimmed].iter().map(|i| i.advance).sum();
    Line {
        start,
        end,
        width,
        ascent,
        descent,
        gap,
        last,
    }
}

fn line_height(line: &Line, spacing: &LineSpacing) -> f32 {
    let single = line.ascent + line.descent + line.gap;
    match *spacing {
        LineSpacing::Single => single,
        LineSpacing::OneAndHalf => single * 1.5,
        LineSpacing::Double => single * 2.0,
        LineSpacing::Multiple(m) => single * m as f32,
        LineSpacing::Exact(pt) => pt as f32,
        LineSpacing::AtLeast(pt) => single.max(pt as f32),
    }
}

/// Lay out a text frame (document points, frame unrotated): columns, and the
/// text fit mode (shrink on overflow / grow / best fit) applied as a uniform
/// scale of the text's sizes.
///
/// The frame's on-page bounds and identity come from `shape.common`; the text
/// content and layout settings from the `TextFrameData` in `shape.kind`. A
/// shape that is not a text frame lays out as empty.
pub fn layout_text_frame(atlas: &Atlas, shape: &Shape) -> FrameLayout {
    let ShapeKind::TextFrame(tf) = &shape.kind else {
        return FrameLayout::default();
    };
    let bounds = shape.common.bounds;
    let id = shape.common.id;
    let scale = fit_scale(atlas, tf, bounds, id);
    if (scale - 1.0).abs() < 1e-4 {
        return layout_scaled(atlas, tf, bounds, id);
    }
    layout_scaled(atlas, &scaled_frame(tf, scale), bounds, id)
}

/// Whether the laid-out text stays inside the frame's text area.
fn fits(layout: &FrameLayout, tf: &TextFrameData, bounds: Rect) -> bool {
    let m = tf.effective_margins();
    let bottom = (bounds.origin.y + bounds.size.height - m.bottom) as f32;
    layout.lines.iter().all(|l| l.bottom <= bottom + 0.5)
}

/// One frame's remembered fit result: the text and bounds it was computed for
/// (so a change recomputes it), its wrap exclusions, and the chosen scale.
type FitCacheEntry = (TextFrameData, Rect, Vec<Exclusion>, f32);

/// Text sizes scale chosen by the frame's fit mode (1.0 = as written).
/// Remembered per frame, since text is laid out every frame.
fn fit_scale(atlas: &Atlas, tf: &TextFrameData, bounds: Rect, id: ObjectId) -> f32 {
    use repub_model::shape::TextFitMode;
    let (low, high) = match tf.text_fit {
        TextFitMode::None => return 1.0,
        TextFitMode::ShrinkOnOverflow => {
            if fits(&layout_scaled(atlas, tf, bounds, id), tf, bounds) {
                return 1.0;
            }
            (0.05, 1.0)
        }
        TextFitMode::GrowToFit => (1.0, 20.0),
        TextFitMode::BestFit => (0.05, 20.0),
    };

    thread_local! {
        static CACHE: std::cell::RefCell<std::collections::HashMap<ObjectId, FitCacheEntry>> = Default::default();
    }
    let exclusions = exclusions_for(id);
    if let Some(scale) = CACHE.with(|c| {
        c.borrow()
            .get(&id)
            .filter(|(cached, b, ex, _)| cached == tf && *b == bounds && *ex == exclusions)
            .map(|(_, _, _, s)| *s)
    }) {
        return scale;
    }
    // Largest scale that fits: binary search to within 1%
    let (mut lo, mut hi) = (low, high);
    if !fits(
        &layout_scaled(atlas, &scaled_frame(tf, lo), bounds, id),
        tf,
        bounds,
    ) {
        hi = lo;
    }
    while hi / lo > 1.01 {
        let mid = (lo * hi).sqrt();
        if fits(
            &layout_scaled(atlas, &scaled_frame(tf, mid), bounds, id),
            tf,
            bounds,
        ) {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    CACHE.with(|c| {
        let mut c = c.borrow_mut();
        if c.len() > 512 {
            c.clear();
        }
        c.insert(id, (tf.clone(), bounds, exclusions, lo));
    });
    lo
}

/// A copy of `tf` with every text size (and size-relative spacing) times `scale`.
fn scaled_frame(tf: &TextFrameData, scale: f32) -> TextFrameData {
    let s = scale as f64;
    let mut out = tf.clone();
    for p in &mut out.content.paragraphs {
        p.style.default_char_style.font_size *= s;
        p.style.space_before *= s;
        p.style.space_after *= s;
        p.style.first_line_indent *= s;
        if let LineSpacing::Exact(pt) | LineSpacing::AtLeast(pt) = &mut p.style.line_spacing {
            *pt *= s;
        }
        for e in &mut p.elements {
            let style = match e {
                InlineElement::Text(run) => &mut run.style,
                InlineElement::Link(link) => &mut link.style,
                _ => continue,
            };
            style.font_size *= s;
            style.letter_spacing *= s;
        }
    }
    out
}

/// Lay out at the sizes as written, flowing through the frame's columns.
fn layout_scaled(atlas: &Atlas, tf: &TextFrameData, bounds: Rect, id: ObjectId) -> FrameLayout {
    let m = tf.effective_margins();
    let left = bounds.origin.x as f32 + m.left as f32;
    let top = bounds.origin.y as f32 + m.top as f32;
    let area_width = (bounds.size.width as f32 - (m.left + m.right) as f32).max(1.0);
    let height = (bounds.size.height as f32 - (m.top + m.bottom) as f32).max(1.0);

    // Columns of equal width; text fills one, then moves to the top of the next
    let columns = tf.columns.count.max(1) as usize;
    let gap = if columns > 1 {
        tf.columns.gap.max(0.0) as f32
    } else {
        0.0
    };
    let width = ((area_width - gap * (columns - 1) as f32) / columns as f32).max(1.0);
    let mut column = 0usize;
    let mut tallest = 0.0f32;

    let exclusions = exclusions_for(id);
    let mut list_counter: Option<(repub_model::styles::NumberFormat, u32)> = None;

    let mut out = FrameLayout::default();
    let mut y = 0.0f32; // top of the current line, relative to `top`

    let default_paragraph = Paragraph::new();
    let paragraphs: Vec<&Paragraph> = if tf.content.paragraphs.is_empty() {
        // An empty frame still has a caret position
        vec![&default_paragraph]
    } else {
        tf.content.paragraphs.iter().collect()
    };

    for (para_index, para) in paragraphs.into_iter().enumerate() {
        let ps = &para.style;
        let items = paragraph_items(atlas, para);
        y += ps.space_before as f32;

        let (indent_left, first_indent, marker) = list_layout(ps, &mut list_counter);
        let default_size = ps.default_char_style.font_size as f32;

        let right_indent = ps.right_indent as f32;
        // An empty paragraph, or one ending in a line break, gets an (empty) last line
        let trailing_empty = items.is_empty() || items.last().is_some_and(|i| i.hard_break);
        let mut emitted_trailing = false;
        let mut start = 0usize;
        let mut index = 0usize;
        let mut estimate = default_size * 1.2;

        while start < items.len() || (trailing_empty && !emitted_trailing) {
            let more_text = start < items.len();
            let first_ind = if index == 0 { first_indent } else { 0.0 };

            // Find where this line goes: the free stretch of the column at this height
            // (text wrap), moving down past obstacles and on to the next column
            let mut guard = 0;
            let (line, seg_left, text_width) = loop {
                guard += 1;
                let col_left = left + column as f32 * (width + gap);
                let min_width = (default_size * 2.0).min(width);
                let (seg_left, seg_right) = match free_segment(
                    col_left,
                    col_left + width,
                    top + y,
                    top + y + estimate,
                    &exclusions,
                    min_width,
                ) {
                    Ok(seg) => seg,
                    Err(next) if guard < 64 => {
                        y = (next - top).max(y + 1.0);
                        continue;
                    }
                    Err(_) => (col_left, col_left + width),
                };
                let text_width = (seg_right - seg_left - indent_left - right_indent).max(1.0);
                let line = if more_text {
                    let end = break_one(&items, start, text_width - first_ind);
                    measure(
                        atlas,
                        &items,
                        start,
                        end,
                        end == items.len() || items[end - 1].hard_break,
                        default_size,
                    )
                } else {
                    measure(
                        atlas,
                        &items,
                        start,
                        start,
                        true,
                        items.last().map_or(default_size, |i| i.size),
                    )
                };
                let lh = line_height(&line, &ps.line_spacing);
                // Full column: carry on at the top of the next one (the last one overflows)
                if column + 1 < columns && y > 0.0 && y + lh > height + 0.5 && guard < 64 {
                    tallest = tallest.max(y);
                    column += 1;
                    y = 0.0;
                    continue;
                }
                // Taller than estimated: check the obstacles against the real height
                if lh > estimate + 0.5 && !exclusions.is_empty() && guard < 64 {
                    estimate = lh;
                    match free_segment(
                        col_left,
                        col_left + width,
                        top + y,
                        top + y + lh,
                        &exclusions,
                        min_width,
                    ) {
                        Ok((a, b)) if (b - a) + 0.5 < seg_right - seg_left => continue,
                        Err(next) => {
                            y = (next - top).max(y + 1.0);
                            continue;
                        }
                        _ => {}
                    }
                }
                break (line, seg_left, text_width);
            };
            let line = &line;
            if !more_text {
                emitted_trailing = true;
            }

            let indent = indent_left + first_ind;
            let avail = text_width - first_ind;
            let slack = (avail - line.width).max(0.0);
            let spaces = items[line.start..line.end]
                .iter()
                .filter(|i| i.is_space && !i.hard_break)
                .count();
            let (mut x, space_extra) = match ps.alignment {
                TextAlignment::Center => (slack / 2.0, 0.0),
                TextAlignment::Right => (slack, 0.0),
                TextAlignment::Justify if !line.last && spaces > 0 => (0.0, slack / spaces as f32),
                _ => (0.0, 0.0),
            };
            let lh = line_height(line, &ps.line_spacing);
            estimate = lh;
            x += seg_left + indent;
            // Extra leading from line spacing goes BELOW the text (after it),
            // not above. The first line starts at the same position regardless
            // of line spacing - only the gap to the NEXT line is affected.
            let line_top = top + y;
            let baseline = line_top + line.ascent;

            // List marker before the first line, in the style of the paragraph's text
            if let (0, Some(marker)) = (index, &marker) {
                let style = items
                    .iter()
                    .find(|i| !i.invisible)
                    .copied()
                    .unwrap_or_else(|| base_item(atlas, ' ', 0, &ps.default_char_style));
                let face = atlas.face(style.face);
                let mut mx = seg_left + marker.x;
                for c in marker.text.chars() {
                    if let Some(g) = face.glyph(c) {
                        if g.has_quad() {
                            let s = style.size;
                            out.quads.push(Quad {
                                rect: [
                                    mx + g.plane[0] * s,
                                    baseline + g.plane[1] * s,
                                    mx + g.plane[2] * s,
                                    baseline + g.plane[3] * s,
                                ],
                                uv: g.uv,
                                color: style.color,
                                baseline,
                                skew: 0.0,
                            });
                        }
                        mx += g.advance * style.size;
                    }
                }
            }

            let first_offset = items.get(line.start).map_or(items.len(), |i| i.offset);
            let mut xs = vec![x];
            for item in &items[line.start..line.end] {
                let face = atlas.face(item.face);
                let solid_uv = [
                    atlas.solid_uv[0],
                    atlas.solid_uv[1],
                    atlas.solid_uv[0],
                    atlas.solid_uv[1],
                ];
                if let (Some(color), false) = (item.background, item.invisible) {
                    let advance = item.advance
                        + if item.is_space && !item.hard_break {
                            space_extra
                        } else {
                            0.0
                        };
                    out.quads.push(Quad {
                        rect: [
                            x,
                            baseline - item.size * 0.85,
                            x + advance,
                            baseline + item.size * 0.25,
                        ],
                        uv: solid_uv,
                        color,
                        baseline,
                        skew: 0.0,
                    });
                }
                if !item.invisible {
                    if let Some(g) = face.glyph(item.c).filter(|g| g.has_quad()) {
                        let s = item.size;
                        let base = baseline - item.rise;
                        out.quads.push(Quad {
                            rect: [
                                x + g.plane[0] * s,
                                base + g.plane[1] * s,
                                x + g.plane[2] * s,
                                base + g.plane[3] * s,
                            ],
                            uv: g.uv,
                            color: item.color,
                            baseline: base,
                            skew: item.skew,
                        });
                    }
                }
                let advance = item.advance
                    + if item.is_space && !item.hard_break {
                        space_extra
                    } else {
                        0.0
                    };
                if !item.invisible {
                    let thickness = (item.size * 0.06).max(0.5);
                    let mut bar = |from: f32, to: f32, y: f32, color: [f32; 4]| {
                        out.quads.push(Quad {
                            rect: [from, y, to, y + thickness],
                            uv: solid_uv,
                            color,
                            baseline,
                            skew: 0.0,
                        });
                    };
                    let color = item.underline_color.unwrap_or(item.color);
                    let uy = baseline + item.size * 0.12;
                    match item.underline {
                        UnderlineStyle::None => {}
                        UnderlineStyle::Single => bar(x, x + advance, uy, color),
                        UnderlineStyle::Double => {
                            bar(x, x + advance, uy, color);
                            bar(x, x + advance, uy + thickness * 2.0, color);
                        }
                        UnderlineStyle::Dotted | UnderlineStyle::Dashed => {
                            let (dash, gap) = if item.underline == UnderlineStyle::Dotted {
                                (thickness, thickness)
                            } else {
                                (thickness * 4.0, thickness * 2.0)
                            };
                            let mut sx = x;
                            while sx < x + advance {
                                bar(sx, (sx + dash).min(x + advance), uy, color);
                                sx += dash + gap;
                            }
                        }
                        UnderlineStyle::Wavy => {
                            let step = (thickness * 2.5).max(1.0);
                            let (mut sx, mut up) = (x, false);
                            while sx < x + advance {
                                bar(
                                    sx,
                                    (sx + step).min(x + advance),
                                    if up { uy - thickness } else { uy + thickness },
                                    color,
                                );
                                sx += step;
                                up = !up;
                            }
                        }
                    }
                    let sy = baseline - item.size * 0.3 - item.rise;
                    match item.strikethrough {
                        StrikethroughStyle::None => {}
                        StrikethroughStyle::Single => bar(x, x + advance, sy, item.color),
                        StrikethroughStyle::Double => {
                            bar(x, x + advance, sy - thickness, item.color);
                            bar(x, x + advance, sy + thickness, item.color);
                        }
                    }
                }
                x += advance;
                xs.push(x);
            }
            // A hard break ends the line before the break character's caret slot
            let mut end_offset = first_offset + (line.end - line.start);
            if line.end > line.start && items[line.end - 1].hard_break {
                end_offset -= 1;
                xs.pop();
            }
            out.lines.push(CaretLine {
                para: para_index,
                start: first_offset,
                end: end_offset,
                xs,
                top: line_top,
                bottom: baseline + line.descent,
                baseline,
            });
            y += lh;
            start = line.end;
            index += 1;
        }
        y += ps.space_after as f32;
    }

    // Vertical alignment within the frame (by the tallest column)
    let y = tallest.max(y);
    let offset = match tf.vertical_align {
        VerticalAlignment::Top => 0.0,
        VerticalAlignment::Center => ((height - y) / 2.0).max(0.0),
        VerticalAlignment::Bottom => (height - y).max(0.0),
    };
    if offset != 0.0 {
        for q in &mut out.quads {
            q.rect[1] += offset;
            q.rect[3] += offset;
            q.baseline += offset;
        }
        for line in &mut out.lines {
            line.top += offset;
            line.bottom += offset;
            line.baseline += offset;
        }
    }
    out
}

#[cfg(test)]
mod frame_tests {
    use super::*;
    use repub_model::shape::{ColumnSpec, Shape, ShapeKind, TextFitMode, TextFrameData};
    use repub_model::text::StyledText;

    /// A text-frame shape with `words` copies of "word" as its content.
    fn frame(w: f64, h: f64, words: usize) -> Shape {
        let text = vec!["word"; words].join(" ");
        text_frame(
            w,
            h,
            StyledText {
                paragraphs: vec![Paragraph::plain(&text)],
            },
        )
    }

    /// A text-frame shape at `(0, 0, w, h)` holding `content`.
    fn text_frame(w: f64, h: f64, content: StyledText) -> Shape {
        let mut data = TextFrameData::new();
        data.content = content;
        Shape::new(Rect::new(0.0, 0.0, w, h), ShapeKind::TextFrame(data))
    }

    /// The `TextFrameData` behind a text-frame shape.
    fn data(shape: &mut Shape) -> &mut TextFrameData {
        match &mut shape.kind {
            ShapeKind::TextFrame(d) => d,
            _ => unreachable!("text frame"),
        }
    }

    /// Bake the default face's glyphs the layout tests draw, as `prepare` does
    /// at runtime, so a test sees quads regardless of which tests ran first.
    fn bake_default_glyphs() {
        let style = CharacterStyle::default();
        let face = {
            let mut atlas = super::super::atlas::write();
            atlas.load(&style);
            atlas.resolve(&style).0
        };
        let chars: Vec<char> = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 .?"
            .chars()
            .collect();
        super::super::atlas::write().bake_many(face, &chars);
    }

    #[test]
    fn single_run_lays_glyphs_at_increasing_x() {
        bake_default_glyphs();
        let atlas = super::super::atlas::get_or_build();
        let shape = text_frame(
            300.0,
            100.0,
            StyledText {
                paragraphs: vec![Paragraph::plain("abcd")],
            },
        );
        let layout = layout_text_frame(&atlas, &shape);
        // One line, with a caret x per character boundary (start..=end), strictly increasing
        assert_eq!(layout.lines.len(), 1);
        let xs = &layout.lines[0].xs;
        assert_eq!(xs.len(), 5, "four characters -> five caret positions");
        for pair in xs.windows(2) {
            assert!(pair[1] > pair[0], "caret x advances: {xs:?}");
        }
        // The glyph quads march left to right too
        let mut last = f32::NEG_INFINITY;
        for q in &layout.quads {
            assert!(q.rect[0] >= last - 0.5, "glyphs laid left to right");
            last = q.rect[0];
        }
    }

    #[test]
    fn date_and_time_fields_survive_a_bad_format() {
        assert!(!formatted_now("%Y", "%B").is_empty());
        // An invalid specifier falls back instead of panicking when displayed
        assert!(!formatted_now("%Q", "%Y").is_empty());
        assert!(!field_text(&Field::time("%l:%M %p")).is_empty());
    }

    #[test]
    fn decorations_add_quads_without_moving_the_text() {
        use repub_model::text::TextRun;
        bake_default_glyphs();
        let atlas = super::super::atlas::get_or_build();
        let quads_for = |style: CharacterStyle| {
            let mut p = Paragraph::new();
            p.elements.push(InlineElement::Text(TextRun {
                text: "abcd".into(),
                style,
            }));
            let shape = text_frame(
                300.0,
                100.0,
                StyledText {
                    paragraphs: vec![p],
                },
            );
            let layout = layout_text_frame(&atlas, &shape);
            (layout.quads.len(), layout.lines[0].xs.clone())
        };
        let (plain, xs) = quads_for(CharacterStyle::default());
        let mut struck = CharacterStyle::default();
        struck.strikethrough = StrikethroughStyle::Double;
        assert_eq!(quads_for(struck).0, plain + 8, "two bars per character");
        let mut double = CharacterStyle::default();
        double.underline = UnderlineStyle::Double;
        assert_eq!(quads_for(double).0, plain + 8);
        let mut marked = CharacterStyle::default();
        marked.background_color = Some(repub_color::Color::from_rgb8(255, 255, 0));
        let (with_bg, marked_xs) = quads_for(marked);
        assert_eq!(with_bg, plain + 4);
        assert_eq!(marked_xs, xs);
        let mut small = CharacterStyle::default();
        small.small_caps = true;
        let mut full = CharacterStyle::default();
        full.all_caps = true;
        assert!(
            quads_for(small).1.last() < quads_for(full).1.last(),
            "small capitals are narrower than full capitals"
        );
    }

    #[test]
    fn superscript_is_smaller_and_raised() {
        use repub_model::styles::BaselineShift;
        use repub_model::text::TextRun;
        bake_default_glyphs();
        let atlas = super::super::atlas::get_or_build();
        let mut sup = CharacterStyle::default();
        sup.baseline_shift = BaselineShift::Superscript;
        let mut p = Paragraph::new();
        p.elements.push(InlineElement::Text(TextRun::new("H")));
        p.elements
            .push(InlineElement::Text(TextRun::styled("H", sup)));
        let shape = text_frame(
            300.0,
            100.0,
            StyledText {
                paragraphs: vec![p],
            },
        );
        let quads = layout_text_frame(&atlas, &shape).quads;
        let (normal, raised) = (quads[0].rect, quads[1].rect);
        assert!(raised[3] - raised[1] < normal[3] - normal[1], "smaller");
        assert!(raised[3] < normal[3] - 1.0, "raised");
    }

    #[test]
    fn list_markers_and_numbering() {
        use repub_model::styles::{ListStyle, NumberFormat};
        bake_default_glyphs();
        let atlas = super::super::atlas::get_or_build();
        let numbered = ListStyle::Numbered {
            format: NumberFormat::Arabic,
            start: 1,
            indent: 18.0,
        };
        let mut paras: Vec<Paragraph> = ["one", "two", "three"]
            .iter()
            .map(|t| Paragraph::plain(t))
            .collect();
        for p in &mut paras {
            p.style.list_style = numbered.clone();
        }
        let shape = text_frame(300.0, 200.0, StyledText { paragraphs: paras });
        let layout = layout_text_frame(&atlas, &shape);
        let left = match &shape.kind {
            ShapeKind::TextFrame(d) => d.effective_margins().left as f32,
            _ => unreachable!(),
        };
        // Text moves in by the list indent; markers sit at the left edge before it
        assert!(
            layout
                .lines
                .iter()
                .all(|l| (l.xs[0] - left - 18.0).abs() < 0.5),
            "text indented"
        );
        let markers = layout
            .quads
            .iter()
            .filter(|q| q.rect[0] < left + 15.0)
            .count();
        assert!(markers >= 3, "a marker per paragraph, got {markers}");
        assert_eq!(list_number(NumberFormat::Arabic, 3), "3.");
        assert_eq!(list_number(NumberFormat::LowerRoman, 4), "iv.");
        assert_eq!(list_number(NumberFormat::UpperAlpha, 28), "BB.");
    }

    #[test]
    fn columns_flow_left_to_right() {
        let atlas = super::super::atlas::get_or_build();
        let mut shape = frame(300.0, 100.0, 200);
        data(&mut shape).columns = ColumnSpec::equal(2, 20.0);
        let layout = layout_text_frame(&atlas, &shape);
        let starts: Vec<f32> = layout.lines.iter().map(|l| l.xs[0]).collect();
        let right = starts.iter().filter(|&&x| x > 150.0).count();
        assert!(right > 0, "some lines in the second column");
        assert!(
            starts.iter().all(|&x| x < 150.0 || x >= 150.0 + 4.0),
            "second column starts after the gap"
        );
        // The first line of column two is back at the top
        let first_right = layout.lines.iter().find(|l| l.xs[0] > 150.0).unwrap();
        assert!((first_right.top - layout.lines[0].top).abs() < 1.0);
        // Lines are narrower than the frame
        assert!(layout
            .lines
            .iter()
            .all(|l| l.xs.last().unwrap() - l.xs[0] <= 140.0 + 1.0));
    }

    #[test]
    fn shrink_on_overflow_fits_the_text_in() {
        let atlas = super::super::atlas::get_or_build();
        let mut shape = frame(200.0, 60.0, 120);
        let (bounds, id) = (shape.common.bounds, shape.common.id);
        assert!(
            !fits(&layout_text_frame(&atlas, &shape), data(&mut shape), bounds),
            "overflows as written"
        );
        data(&mut shape).text_fit = TextFitMode::ShrinkOnOverflow;
        assert!(fits(
            &layout_text_frame(&atlas, &shape),
            data(&mut shape),
            bounds
        ));
        // Short text is left alone
        let mut short = frame(200.0, 60.0, 2);
        data(&mut short).text_fit = TextFitMode::ShrinkOnOverflow;
        let (sb, sid) = (short.common.bounds, short.common.id);
        assert_eq!(fit_scale(&atlas, data(&mut short), sb, sid), 1.0);
    }

    #[test]
    fn best_fit_grows_short_text() {
        let atlas = super::super::atlas::get_or_build();
        let mut shape = frame(300.0, 200.0, 3);
        data(&mut shape).text_fit = TextFitMode::BestFit;
        let (bounds, id) = (shape.common.bounds, shape.common.id);
        let scale = fit_scale(&atlas, data(&mut shape), bounds, id);
        assert!(scale > 2.0, "grew to {scale}");
        assert!(fits(
            &layout_text_frame(&atlas, &shape),
            data(&mut shape),
            bounds
        ));
    }
}
