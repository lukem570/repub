//! Character and paragraph formatting of styled text.
//!
//! These are the framework-free building blocks: toggling character styles,
//! applying size/colour/family changes to a range, reading the styles across a
//! range, and changing paragraph styles. Resolving what the user's selection
//! "targets" (the word at the caret, the selected text boxes, ...) is an
//! application concern that drives these functions.
//!
//! Toggles (bold, italic, underline) turn the style off only when the whole
//! target already has it, else on, so mixed text becomes uniform.

use repub_color::Color;
use repub_model::styles::{
    BaselineShift, CharacterStyle, FontStyle, FontWeight, LineSpacing, ListStyle, NumberFormat, ParagraphStyle, TextAlignment,
    UnderlineStyle,
};
use repub_model::text::{InlineElement, StyledText};

use crate::text_edit::{element_len, para_len, split_run_at, TextPos};

/// A style that toggles on and off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    /// Bold weight.
    Bold,
    /// Italic style.
    Italic,
    /// Single underline.
    Underline,
    /// Superscript baseline.
    Superscript,
    /// Subscript baseline.
    Subscript,
}

impl Toggle {
    /// Label used for undo descriptions.
    pub fn label(self) -> &'static str {
        match self {
            Toggle::Bold => "Bold",
            Toggle::Italic => "Italic",
            Toggle::Underline => "Underline",
            Toggle::Superscript => "Superscript",
            Toggle::Subscript => "Subscript",
        }
    }

    /// Whether `style` currently has this toggle on.
    pub fn is_on(self, style: &CharacterStyle) -> bool {
        match self {
            Toggle::Bold => style.font.weight == FontWeight::Bold,
            Toggle::Italic => style.font.style == FontStyle::Italic,
            Toggle::Underline => style.underline != UnderlineStyle::None,
            Toggle::Superscript => style.baseline_shift == BaselineShift::Superscript,
            Toggle::Subscript => style.baseline_shift == BaselineShift::Subscript,
        }
    }

    /// Set this toggle on or off in `style`.
    pub fn set(self, style: &mut CharacterStyle, on: bool) {
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
    /// Toggle a style on or off.
    Set(Toggle, bool),
    /// Set the font size in points.
    Size(f64),
    /// One step up (`true`) or down the standard size list.
    StepSize(bool),
    /// Set the text colour.
    Color(Color),
    /// Set the font family name.
    Family(String),
}

/// Publisher's font size list, used by Grow/Shrink Font.
pub const FONT_SIZES: [f64; 16] = [
    8.0, 9.0, 10.0, 11.0, 12.0, 14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0, 36.0, 48.0, 72.0,
];

/// The next font size up (`up`) or down the standard list from `size`.
pub fn step_size(size: f64, up: bool) -> f64 {
    if up {
        FONT_SIZES.iter().copied().find(|&s| s > size + 0.01).unwrap_or((size + 10.0).min(1638.0))
    } else {
        FONT_SIZES.iter().rev().copied().find(|&s| s < size - 0.01).unwrap_or((size - 1.0).max(1.0))
    }
}

impl CharChange {
    /// Apply this change to a single character style.
    pub fn apply(&self, style: &mut CharacterStyle) {
        match self {
            CharChange::Set(toggle, on) => toggle.set(style, *on),
            CharChange::Size(size) => style.font_size = size.clamp(1.0, 1638.0),
            CharChange::StepSize(up) => style.font_size = step_size(style.font_size, *up),
            CharChange::Color(color) => style.color = *color,
            CharChange::Family(family) => style.font.family = family.clone(),
        }
    }

    /// Label used for undo descriptions.
    pub fn label(&self) -> &'static str {
        match self {
            CharChange::Set(toggle, _) => toggle.label(),
            CharChange::Size(_) | CharChange::StepSize(_) => "Font Size",
            CharChange::Color(_) => "Font Color",
            CharChange::Family(_) => "Font",
        }
    }
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
pub fn style_range(text: &mut StyledText, range: Option<(TextPos, TextPos)>, f: &mut dyn FnMut(&mut CharacterStyle)) {
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

/// Apply a character change over a range (whole text when `None`).
pub fn apply_char_change(text: &mut StyledText, range: Option<(TextPos, TextPos)>, change: &CharChange) {
    style_range(text, range, &mut |s| change.apply(s));
}

/// Styles of the characters in `[a, b)` (whole text when `None`), read-only.
pub fn styles_in(text: &StyledText, range: Option<(TextPos, TextPos)>) -> Vec<CharacterStyle> {
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

/// Whether every style in `styles` has `toggle` on (false when empty).
pub fn all_on(styles: &[CharacterStyle], toggle: Toggle) -> bool {
    !styles.is_empty() && styles.iter().all(|s| toggle.is_on(s))
}

/// Run `f` on the style of every paragraph whose index is in `range`.
pub fn style_paragraphs(text: &mut StyledText, range: std::ops::RangeInclusive<usize>, f: &mut dyn FnMut(&mut ParagraphStyle)) {
    for (i, p) in text.paragraphs.iter_mut().enumerate() {
        if range.contains(&i) {
            f(&mut p.style);
        }
    }
}

/// Kind of list for the Bullets / Numbering buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListKind {
    /// Bulleted list.
    Bullets,
    /// Numbered list.
    Numbering,
}

/// The list kind of a paragraph style, if any.
pub fn list_kind(style: &ListStyle) -> Option<ListKind> {
    match style {
        ListStyle::None => None,
        ListStyle::Bullet { .. } => Some(ListKind::Bullets),
        ListStyle::Numbered { .. } => Some(ListKind::Numbering),
    }
}

/// The list style for turning a list `kind` on (`on`), with a ¼" hanging indent.
pub fn list_style_for(kind: ListKind, on: bool) -> ListStyle {
    match (on, kind) {
        (false, _) => ListStyle::None,
        (true, ListKind::Bullets) => ListStyle::Bullet { character: '•', indent: 18.0 },
        (true, ListKind::Numbering) => ListStyle::Numbered { format: NumberFormat::Arabic, start: 1, indent: 18.0 },
    }
}

/// Indent step for Increase/Decrease Indent: a quarter inch.
pub const INDENT_STEP: f64 = 18.0;

/// The left indent moved one step in (`increase`) or out (never negative).
pub fn step_indent(left_indent: f64, increase: bool) -> f64 {
    if increase {
        (left_indent / INDENT_STEP).floor() * INDENT_STEP + INDENT_STEP
    } else {
        ((left_indent / INDENT_STEP).ceil() * INDENT_STEP - INDENT_STEP).max(0.0)
    }
}

/// Line spacing for a multiple of single spacing.
pub fn line_spacing_for(multiple: f64) -> LineSpacing {
    match multiple {
        m if (m - 1.0).abs() < 1e-6 => LineSpacing::Single,
        m if (m - 1.5).abs() < 1e-6 => LineSpacing::OneAndHalf,
        m if (m - 2.0).abs() < 1e-6 => LineSpacing::Double,
        m => LineSpacing::Multiple(m),
    }
}

/// A line spacing as a multiple of single spacing, if it is one.
pub fn line_spacing_multiple(spacing: &LineSpacing) -> Option<f64> {
    match spacing {
        LineSpacing::Single => Some(1.0),
        LineSpacing::OneAndHalf => Some(1.5),
        LineSpacing::Double => Some(2.0),
        LineSpacing::Multiple(m) => Some(*m),
        _ => None,
    }
}

/// The alignment shared by `styles`, if they agree.
pub fn shared_alignment<'a>(styles: impl IntoIterator<Item = &'a ParagraphStyle>) -> Option<TextAlignment> {
    let mut found = None;
    for p in styles {
        match found {
            None => found = Some(p.alignment),
            Some(a) if a != p.alignment => return None,
            _ => {}
        }
    }
    found
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
        let sizes = |p: &Paragraph| {
            p.elements
                .iter()
                .filter_map(|e| match e {
                    InlineElement::Text(r) if !r.text.is_empty() => Some((r.text.clone(), r.style.font_size)),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
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

    #[test]
    fn paragraph_styling_and_helpers() {
        let mut text = StyledText { paragraphs: vec![Paragraph::plain("a"), Paragraph::plain("b")] };
        style_paragraphs(&mut text, 0..=0, &mut |p| p.alignment = TextAlignment::Center);
        assert_eq!(text.paragraphs[0].style.alignment, TextAlignment::Center);
        assert_eq!(text.paragraphs[1].style.alignment, TextAlignment::Left);
        assert_eq!(step_indent(0.0, true), INDENT_STEP);
        assert_eq!(step_indent(INDENT_STEP, false), 0.0);
        assert_eq!(line_spacing_multiple(&line_spacing_for(1.5)), Some(1.5));
    }
}
