//! Text frame layout.
//!
//! This module handles laying out text within frames, including:
//! - Line breaking and text wrapping
//! - Multi-column layout
//! - Vertical alignment
//! - Overflow detection
//! - Tab stops
//! - Text truncation with ellipsis
//!
//! The frame is modeled as a [`Shape`] whose [`ShapeKind::TextFrame`] carries a
//! [`TextFrameData`]. The frame's placement comes from `shape.common.bounds`;
//! its content, inset, columns, vertical alignment and overflow behavior come
//! from the [`TextFrameData`].

use repub_geometry::Rect;
use repub_model::shape::{ColumnSpec, Shape, ShapeKind, TextFrameData};
use repub_model::styles::{CharacterStyle, ParagraphStyle, TextAlignment};
use repub_model::text::{Paragraph, StyledText};

use repub_fonts::FontProvider;
use repub_text::{LineBreaker, COLUMN_BREAK_CHAR};

/// Alignment for tab stops.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TabAlignment {
    /// Text aligns to the left of the tab stop
    #[default]
    Left,
    /// Text is centered on the tab stop
    Center,
    /// Text aligns to the right of the tab stop
    Right,
    /// Text aligns on a decimal point
    Decimal,
}

/// A tab stop position and alignment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabStop {
    /// Position from left edge in points
    pub position: f64,
    /// Alignment at this tab stop
    pub alignment: TabAlignment,
    /// Character to align on for decimal tabs (default '.')
    pub decimal_char: char,
}

impl TabStop {
    /// Create a new left-aligned tab stop.
    pub fn left(position: f64) -> Self {
        Self {
            position,
            alignment: TabAlignment::Left,
            decimal_char: '.',
        }
    }

    /// Create a new center-aligned tab stop.
    pub fn center(position: f64) -> Self {
        Self {
            position,
            alignment: TabAlignment::Center,
            decimal_char: '.',
        }
    }

    /// Create a new right-aligned tab stop.
    pub fn right(position: f64) -> Self {
        Self {
            position,
            alignment: TabAlignment::Right,
            decimal_char: '.',
        }
    }

    /// Create a new decimal-aligned tab stop.
    pub fn decimal(position: f64) -> Self {
        Self {
            position,
            alignment: TabAlignment::Decimal,
            decimal_char: '.',
        }
    }

    /// Create a decimal tab stop with a custom alignment character.
    pub fn decimal_with_char(position: f64, decimal_char: char) -> Self {
        Self {
            position,
            alignment: TabAlignment::Decimal,
            decimal_char,
        }
    }
}

impl Default for TabStop {
    fn default() -> Self {
        Self::left(72.0) // Default 1 inch
    }
}

/// Vertical alignment for text within a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlignment {
    /// Align to the top of the frame
    #[default]
    Top,
    /// Center vertically in the frame
    Center,
    /// Align to the bottom of the frame
    Bottom,
}

impl From<repub_model::shape::VerticalAlignment> for VerticalAlignment {
    fn from(align: repub_model::shape::VerticalAlignment) -> Self {
        match align {
            repub_model::shape::VerticalAlignment::Top => VerticalAlignment::Top,
            repub_model::shape::VerticalAlignment::Center => VerticalAlignment::Center,
            repub_model::shape::VerticalAlignment::Bottom => VerticalAlignment::Bottom,
        }
    }
}

/// Options for text truncation when overflow occurs.
#[derive(Debug, Clone, PartialEq)]
pub struct TruncationOptions {
    /// Whether truncation is enabled
    pub enabled: bool,
    /// The ellipsis string to use (default "...")
    pub ellipsis: String,
    /// Where to truncate (end, middle, start)
    pub position: TruncationPosition,
}

impl Default for TruncationOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            ellipsis: "…".to_string(), // Unicode ellipsis
            position: TruncationPosition::End,
        }
    }
}

impl TruncationOptions {
    /// Create truncation options with ellipsis at the end.
    pub fn ellipsis_end() -> Self {
        Self {
            enabled: true,
            ellipsis: "…".to_string(),
            position: TruncationPosition::End,
        }
    }

    /// Create truncation options with custom ellipsis.
    pub fn with_ellipsis(ellipsis: &str) -> Self {
        Self {
            enabled: true,
            ellipsis: ellipsis.to_string(),
            position: TruncationPosition::End,
        }
    }
}

/// Position where truncation occurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TruncationPosition {
    /// Truncate at the end (most common)
    #[default]
    End,
    /// Truncate in the middle
    Middle,
    /// Truncate at the start
    Start,
}

/// A laid out glyph with position.
#[derive(Debug, Clone)]
pub struct LayoutGlyph {
    /// The character
    pub char: char,
    /// Position relative to line origin
    pub x: f64,
    /// Advance width
    pub advance: f64,
    /// Font size
    pub font_size: f64,
    /// Index into the original text
    pub text_index: usize,
}

/// A run of text with consistent styling.
#[derive(Debug, Clone)]
pub struct LayoutRun {
    /// Text content
    pub text: String,
    /// Start position relative to line
    pub x: f64,
    /// Width of this run
    pub width: f64,
    /// Character style
    pub style: CharacterStyle,
    /// Individual glyphs
    pub glyphs: Vec<LayoutGlyph>,
}

impl LayoutRun {
    /// Create a new layout run.
    pub fn new(text: String, x: f64, width: f64, style: CharacterStyle) -> Self {
        Self {
            text,
            x,
            width,
            style,
            glyphs: Vec::new(),
        }
    }
}

/// A laid out line of text.
#[derive(Debug, Clone)]
pub struct LayoutLine {
    /// Y position of baseline relative to frame
    pub baseline_y: f64,
    /// Line width
    pub width: f64,
    /// Line height
    pub height: f64,
    /// Ascent from baseline
    pub ascent: f64,
    /// Descent from baseline
    pub descent: f64,
    /// Runs of text on this line
    pub runs: Vec<LayoutRun>,
    /// Paragraph index this line belongs to
    pub paragraph_index: usize,
    /// Whether this line ends with a hard break
    pub hard_break: bool,
    /// Whether this is the last line of a paragraph
    pub is_last_line: bool,
    /// Tab stops for this line
    pub tab_stops: Vec<TabStop>,
    /// Whether this line was truncated
    pub truncated: bool,
    /// Frame index for linked frame layouts (0 = first frame)
    pub frame_index: usize,
    /// Column index within the frame
    pub column_index: usize,
}

impl LayoutLine {
    /// Create a new empty line.
    pub fn new(baseline_y: f64, paragraph_index: usize) -> Self {
        Self {
            baseline_y,
            width: 0.0,
            height: 0.0,
            ascent: 0.0,
            descent: 0.0,
            runs: Vec::new(),
            paragraph_index,
            hard_break: false,
            is_last_line: false,
            tab_stops: Vec::new(),
            truncated: false,
            frame_index: 0,
            column_index: 0,
        }
    }

    /// Set tab stops for this line.
    pub fn with_tab_stops(mut self, tab_stops: Vec<TabStop>) -> Self {
        self.tab_stops = tab_stops;
        self
    }

    /// Count the number of word boundaries (spaces between words).
    pub fn word_gap_count(&self) -> usize {
        let mut count = 0;
        for run in &self.runs {
            let text = run.text.trim();
            count += text.chars().filter(|c| c.is_whitespace()).count();
        }
        count
    }

    /// Add a run to this line.
    pub fn add_run(&mut self, run: LayoutRun) {
        self.width += run.width;
        self.runs.push(run);
    }

    /// Total number of characters on this line.
    pub fn char_count(&self) -> usize {
        self.runs.iter().map(|r| r.text.chars().count()).sum()
    }
}

/// A laid out paragraph.
#[derive(Debug, Clone)]
pub struct LayoutParagraph {
    /// Lines in this paragraph
    pub lines: Vec<LayoutLine>,
    /// Total height
    pub height: f64,
    /// Paragraph style
    pub style: ParagraphStyle,
}

impl LayoutParagraph {
    /// Create a new layout paragraph.
    pub fn new(style: ParagraphStyle) -> Self {
        Self {
            lines: Vec::new(),
            height: 0.0,
            style,
        }
    }

    /// Add a line to this paragraph.
    pub fn add_line(&mut self, line: LayoutLine) {
        self.height += line.height;
        self.lines.push(line);
    }
}

/// A laid out column.
#[derive(Debug, Clone)]
pub struct LayoutColumn {
    /// Column bounds
    pub bounds: Rect,
    /// Lines in this column
    pub lines: Vec<LayoutLine>,
}

impl LayoutColumn {
    /// Create a new layout column.
    pub fn new(bounds: Rect) -> Self {
        Self {
            bounds,
            lines: Vec::new(),
        }
    }

    /// Total height used by content.
    pub fn content_height(&self) -> f64 {
        self.lines
            .last()
            .map(|l| l.baseline_y + l.descent)
            .unwrap_or(0.0)
    }

    /// Remaining height available.
    pub fn remaining_height(&self) -> f64 {
        self.bounds.height() - self.content_height()
    }
}

/// Complete layout for a text frame.
#[derive(Debug, Clone)]
pub struct TextFrameLayout {
    /// Frame bounds
    pub bounds: Rect,
    /// Content bounds (inside insets)
    pub content_bounds: Rect,
    /// Columns
    pub columns: Vec<LayoutColumn>,
    /// Paragraphs
    pub paragraphs: Vec<LayoutParagraph>,
    /// Whether there is overflow
    pub has_overflow: bool,
    /// Index of first character that overflowed (if any)
    pub overflow_start: Option<usize>,
    /// Vertical alignment applied
    pub vertical_alignment: VerticalAlignment,
    /// Whether truncation was applied
    pub was_truncated: bool,
    /// Frame index in a linked frame chain (0 = first frame)
    pub frame_index: usize,
    /// Whether this frame has any content
    pub has_content: bool,
}

impl TextFrameLayout {
    /// Create a new text frame layout.
    pub fn new(bounds: Rect, content_bounds: Rect) -> Self {
        Self {
            bounds,
            content_bounds,
            columns: Vec::new(),
            paragraphs: Vec::new(),
            has_overflow: false,
            overflow_start: None,
            vertical_alignment: VerticalAlignment::Top,
            was_truncated: false,
            frame_index: 0,
            has_content: false,
        }
    }

    /// Calculate total content height across all columns.
    pub fn content_height(&self) -> f64 {
        self.columns
            .iter()
            .map(|c| c.content_height())
            .fold(0.0, f64::max)
    }

    /// Total number of lines across all columns.
    pub fn line_count(&self) -> usize {
        self.columns.iter().map(|c| c.lines.len()).sum()
    }

    /// Get all lines in order.
    pub fn all_lines(&self) -> impl Iterator<Item = &LayoutLine> {
        self.columns.iter().flat_map(|c| c.lines.iter())
    }

    /// The caret rectangle for a position given as `(paragraph, offset)`, where
    /// `offset` is a character offset within that paragraph. Coordinates are in
    /// page points (the same space as [`TextFrameLayout::bounds`]). Returns
    /// `None` if the position has no laid-out glyphs (e.g. an empty frame).
    ///
    /// The caret is a thin vertical rectangle spanning the line's ascent to
    /// descent at the x of the glyph at `offset` (or the line end).
    pub fn caret_rect(&self, paragraph: usize, offset: usize) -> Option<Rect> {
        // Walk this paragraph's lines in order, counting characters, to find
        // the line and x for `offset`.
        let mut seen = 0usize;
        let mut last: Option<(&LayoutColumn, &LayoutLine)> = None;
        for col in &self.columns {
            for line in &col.lines {
                if line.paragraph_index != paragraph {
                    continue;
                }
                let line_chars = line_char_count(line);
                last = Some((col, line));
                if offset <= seen + line_chars {
                    let x = line_x_at(line, offset - seen);
                    return Some(caret_from_line(col, line, x));
                }
                seen += line_chars;
            }
        }
        // Offset past the end: caret at the end of the last line.
        last.map(|(col, line)| caret_from_line(col, line, line_x_at(line, line_char_count(line))))
    }

    /// The text position `(paragraph, offset)` nearest to page point `(x, y)`.
    /// Picks the closest line by vertical distance, then the character gap on
    /// that line nearest the click. Returns `None` for an empty layout.
    pub fn hit_test(&self, x: f64, y: f64) -> Option<(usize, usize)> {
        // Find the closest line across all columns.
        let mut best: Option<(&LayoutColumn, &LayoutLine, f64)> = None;
        for col in &self.columns {
            for line in &col.lines {
                let top = col.bounds.origin.y + line.baseline_y - line.ascent;
                let bottom = col.bounds.origin.y + line.baseline_y + line.descent;
                let dist = if y < top {
                    top - y
                } else if y > bottom {
                    y - bottom
                } else {
                    0.0
                };
                if best.as_ref().map(|(_, _, d)| dist < *d).unwrap_or(true) {
                    best = Some((col, line, dist));
                }
            }
        }
        let (col, line, _) = best?;
        // Nearest character gap on the line.
        let line_x = x - col.bounds.origin.x;
        let line_chars = line_char_count(line);
        let mut best_gap = (f64::INFINITY, 0usize);
        for gap in 0..=line_chars {
            let gx = line_x_at(line, gap);
            let d = (line_x - gx).abs();
            if d < best_gap.0 {
                best_gap = (d, gap);
            }
        }
        let offset_in_line = best_gap.1;
        // Convert the in-line offset to a paragraph offset by summing earlier
        // lines of the same paragraph.
        let mut seen = 0usize;
        for c in &self.columns {
            for l in &c.lines {
                if l.paragraph_index != line.paragraph_index {
                    continue;
                }
                if std::ptr::eq(l, line) {
                    return Some((line.paragraph_index, seen + offset_in_line));
                }
                seen += line_char_count(l);
            }
        }
        Some((line.paragraph_index, offset_in_line))
    }
}

/// Total character count of a line's runs.
fn line_char_count(line: &LayoutLine) -> usize {
    line.runs.iter().map(|r| r.text.chars().count()).sum()
}

/// X of the character gap at `index` within a line (relative to the line
/// origin). Interpolates within each run by character fraction, since per-glyph
/// positions are not always populated.
fn line_x_at(line: &LayoutLine, index: usize) -> f64 {
    let mut seen = 0usize;
    for run in &line.runs {
        let n = run.text.chars().count();
        if index <= seen + n {
            let local = index - seen;
            // Prefer exact glyph positions when present.
            if !run.glyphs.is_empty() && local <= run.glyphs.len() {
                return if local < run.glyphs.len() {
                    run.x + run.glyphs[local].x
                } else {
                    let g = &run.glyphs[run.glyphs.len() - 1];
                    run.x + g.x + g.advance
                };
            }
            let frac = if n == 0 { 0.0 } else { local as f64 / n as f64 };
            return run.x + run.width * frac;
        }
        seen += n;
    }
    // Past the end: after the last run.
    line.runs.last().map(|r| r.x + r.width).unwrap_or(0.0)
}

/// Build a caret rect for a line at line-relative x.
fn caret_from_line(col: &LayoutColumn, line: &LayoutLine, x: f64) -> Rect {
    let page_x = col.bounds.origin.x + x;
    let top = col.bounds.origin.y + line.baseline_y - line.ascent;
    let height = (line.ascent + line.descent).max(1.0);
    Rect::new(page_x, top, 1.0, height)
}

/// Characters that can hang outside the left margin.
pub const LEFT_HANG_PUNCTUATION: &[char] =
    &['"', '\u{201C}', '\u{2018}', '\'', '\u{00AB}', '\u{2039}'];

/// Characters that can hang outside the right margin.
pub const RIGHT_HANG_PUNCTUATION: &[char] = &[
    '.', ',', '-', '\u{2013}', '\u{2014}', '"', '\u{201D}', '\u{2019}', '\'', '\u{00BB}',
    '\u{203A}', ':', ';',
];

/// Configuration for hanging punctuation.
#[derive(Debug, Clone, Copy, Default)]
pub struct HangingPunctuation {
    /// Enable hanging punctuation on the left margin
    pub left: bool,
    /// Enable hanging punctuation on the right margin
    pub right: bool,
    /// How much of the punctuation width to hang (0.0-1.0)
    pub hang_ratio: f64,
}

impl HangingPunctuation {
    /// Create hanging punctuation config with both sides enabled.
    pub fn enabled() -> Self {
        Self {
            left: true,
            right: true,
            hang_ratio: 1.0,
        }
    }

    /// Create with partial hanging (e.g., 0.5 for half-hang).
    pub fn partial(ratio: f64) -> Self {
        Self {
            left: true,
            right: true,
            hang_ratio: ratio.clamp(0.0, 1.0),
        }
    }
}

/// Options for column balancing.
#[derive(Debug, Clone, Copy, Default)]
pub struct ColumnBalanceOptions {
    /// Whether to balance columns
    pub enabled: bool,
    /// Maximum iterations for balancing algorithm
    pub max_iterations: usize,
    /// Tolerance for height difference (in points)
    pub tolerance: f64,
}

impl ColumnBalanceOptions {
    /// Create enabled column balancing with default settings.
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            max_iterations: 10,
            tolerance: 2.0,
        }
    }
}

/// Text layouter for frames.
pub struct TextLayouter<'a> {
    font_provider: &'a dyn FontProvider,
    line_breaker: LineBreaker,
    hanging_punctuation: HangingPunctuation,
    column_balance: ColumnBalanceOptions,
}

impl<'a> TextLayouter<'a> {
    /// Create a new text layouter.
    pub fn new(font_provider: &'a dyn FontProvider) -> Self {
        Self {
            font_provider,
            line_breaker: LineBreaker::new(),
            hanging_punctuation: HangingPunctuation::default(),
            column_balance: ColumnBalanceOptions::default(),
        }
    }

    /// Enable hanging punctuation.
    pub fn with_hanging_punctuation(mut self, config: HangingPunctuation) -> Self {
        self.hanging_punctuation = config;
        self
    }

    /// Enable column balancing.
    pub fn with_column_balance(mut self, options: ColumnBalanceOptions) -> Self {
        self.column_balance = options;
        self
    }

    /// Layout a text-frame shape, if its kind is a text frame.
    ///
    /// This is the convenient entry point: it reads the bounds from the shape
    /// and the content from the [`TextFrameData`] payload.
    pub fn layout_shape(&self, shape: &Shape) -> Option<TextFrameLayout> {
        match &shape.kind {
            ShapeKind::TextFrame(data) => Some(self.layout_frame(shape, data)),
            _ => None,
        }
    }

    /// Layout a text frame given its shape (for bounds) and its data.
    pub fn layout_frame(&self, shape: &Shape, data: &TextFrameData) -> TextFrameLayout {
        let bounds = shape.common.bounds;
        let inset = data.inset;
        let content_bounds = Rect::new(
            bounds.origin.x + inset,
            bounds.origin.y + inset,
            bounds.width() - inset * 2.0,
            bounds.height() - inset * 2.0,
        );

        let mut layout = TextFrameLayout::new(bounds, content_bounds);
        layout.vertical_alignment = data.vertical_align.into();

        // Create columns
        let column_bounds = self.compute_column_bounds(&data.columns, &content_bounds);
        for col_bounds in column_bounds {
            layout.columns.push(LayoutColumn::new(col_bounds));
        }

        if layout.columns.is_empty() {
            layout.columns.push(LayoutColumn::new(content_bounds));
        }

        // Layout paragraphs
        self.layout_paragraphs(&data.content, &mut layout);

        // Apply column balancing if enabled and multiple columns
        if self.column_balance.enabled && layout.columns.len() > 1 && !layout.has_overflow {
            self.balance_columns(&mut layout);
        }

        // Apply hanging punctuation if enabled
        if self.hanging_punctuation.left || self.hanging_punctuation.right {
            self.apply_hanging_punctuation(&mut layout);
        }

        layout
    }

    /// Compute column bounds.
    fn compute_column_bounds(&self, spec: &ColumnSpec, content: &Rect) -> Vec<Rect> {
        if spec.count <= 1 {
            return vec![*content];
        }

        if spec.equal_width {
            let total_gap = (spec.count as f64 - 1.0) * spec.gap;
            let col_width = (content.width() - total_gap) / spec.count as f64;

            (0..spec.count)
                .map(|i| {
                    let x = content.origin.x + (col_width + spec.gap) * i as f64;
                    Rect::new(x, content.origin.y, col_width, content.height())
                })
                .collect()
        } else {
            // Custom widths
            let total_gap = (spec.widths.len() as f64 - 1.0) * spec.gap;
            let total_width: f64 = spec.widths.iter().sum();
            let scale = (content.width() - total_gap) / total_width;

            let mut x = content.origin.x;
            spec.widths
                .iter()
                .map(|w| {
                    let col_width = w * scale;
                    let rect = Rect::new(x, content.origin.y, col_width, content.height());
                    x += col_width + spec.gap;
                    rect
                })
                .collect()
        }
    }

    /// Layout all paragraphs into columns.
    fn layout_paragraphs(&self, content: &StyledText, layout: &mut TextFrameLayout) {
        let mut current_col = 0;
        let mut y = 0.0;
        let mut char_index = 0;

        for (para_idx, paragraph) in content.paragraphs.iter().enumerate() {
            let (para_layout, column_break) = self.layout_paragraph_with_breaks(
                paragraph,
                para_idx,
                layout,
                &mut current_col,
                &mut y,
            );

            // Check if we need to flow to next column due to overflow
            while y > layout.columns[current_col].bounds.height() {
                current_col += 1;
                if current_col >= layout.columns.len() {
                    // Overflow
                    layout.has_overflow = true;
                    layout.overflow_start = Some(char_index);
                    return;
                }
                y = 0.0;
            }

            // Handle explicit column break
            if column_break {
                current_col += 1;
                if current_col >= layout.columns.len() {
                    layout.has_overflow = true;
                    layout.overflow_start = Some(char_index);
                    return;
                }
                y = 0.0;
            }

            char_index += paragraph.plain_text().chars().count();
            layout.paragraphs.push(para_layout);
        }
    }

    /// Layout a single paragraph, returning whether a column break was encountered.
    fn layout_paragraph_with_breaks(
        &self,
        paragraph: &Paragraph,
        para_idx: usize,
        layout: &mut TextFrameLayout,
        col_idx: &mut usize,
        y: &mut f64,
    ) -> (LayoutParagraph, bool) {
        let style = paragraph.style.clone();
        let mut para_layout = LayoutParagraph::new(style.clone());
        let mut encountered_column_break = false;

        // Add space before
        *y += style.space_before;

        // Layout each line
        let text = paragraph.plain_text();
        let mut remaining = text.as_str();
        let mut first_line = true;

        while !remaining.is_empty() {
            // Check for column break at start of remaining text
            if remaining.starts_with(COLUMN_BREAK_CHAR) {
                // Move to next column
                *col_idx += 1;
                if *col_idx >= layout.columns.len() {
                    encountered_column_break = true;
                    break;
                }
                *y = 0.0;
                remaining = &remaining[COLUMN_BREAK_CHAR.len_utf8()..];
                continue;
            }

            // Get column width for current column
            let col_width = layout.columns[*col_idx].bounds.width();

            let indent = if first_line {
                style.first_line_indent
            } else {
                style.left_indent
            };

            let available_width = col_width - indent - style.right_indent;
            let (mut line, consumed, has_column_break) =
                self.layout_line_with_breaks(remaining, available_width, para_idx, *y, indent);

            // Track column index for cross-frame flow
            line.column_index = *col_idx;
            line.frame_index = layout.frame_index;

            *y += line.height;
            layout.has_content = true;
            para_layout.add_line(line.clone());
            layout.columns[*col_idx].lines.push(line);

            remaining = &remaining[consumed..];
            first_line = false;

            // Handle column break within line
            if has_column_break {
                *col_idx += 1;
                if *col_idx >= layout.columns.len() {
                    encountered_column_break = true;
                    break;
                }
                *y = 0.0;
            }
        }

        // Mark the last line of this paragraph
        if let Some(last_line) = para_layout.lines.last_mut() {
            last_line.is_last_line = true;
        }
        // Also mark it in the column's lines
        if let Some(col) = layout.columns.get_mut(*col_idx) {
            if let Some(last_line) = col.lines.last_mut() {
                if last_line.paragraph_index == para_idx {
                    last_line.is_last_line = true;
                }
            }
        }

        // Add space after
        *y += style.space_after;
        para_layout.height = *y;

        (para_layout, encountered_column_break)
    }

    /// Layout a single paragraph (backward compatibility).
    #[allow(dead_code)]
    fn layout_paragraph(
        &self,
        paragraph: &Paragraph,
        para_idx: usize,
        layout: &mut TextFrameLayout,
        col_idx: usize,
        y: &mut f64,
    ) -> LayoutParagraph {
        let mut col = col_idx;
        let (para_layout, _) =
            self.layout_paragraph_with_breaks(paragraph, para_idx, layout, &mut col, y);
        para_layout
    }

    /// Layout a single line of text, returning (line, consumed bytes, has column break).
    fn layout_line_with_breaks(
        &self,
        text: &str,
        available_width: f64,
        para_idx: usize,
        y: f64,
        indent: f64,
    ) -> (LayoutLine, usize, bool) {
        let style = CharacterStyle::default();

        // Check for column break in text
        let column_break_pos = text.find(COLUMN_BREAK_CHAR);
        let text_before_break = match column_break_pos {
            Some(pos) => &text[..pos],
            None => text,
        };

        // Find the break point
        let measure = |s: &str| {
            let font = self.font_provider.default_font();
            self.font_provider.measure_text(s, &font, style.font_size)
        };

        let break_point =
            self.line_breaker
                .find_break_for_width(text_before_break, available_width, measure);

        let (line_text, has_column_break) = match break_point {
            Some(bp) => (&text[..bp], false),
            None => {
                // No line break needed, check if we stopped at column break
                match column_break_pos {
                    Some(pos) => (&text[..pos], true),
                    None => (text, false),
                }
            }
        };

        // Create the line
        let font = self.font_provider.default_font();
        let metrics = self.font_provider.get_metrics(&font, style.font_size);

        let mut line = LayoutLine::new(y + metrics.ascent, para_idx);
        line.ascent = metrics.ascent;
        line.descent = metrics.descent;
        line.height = metrics.line_height;
        line.hard_break = line_text.ends_with('\n');

        // Create run for the line (strip column break char if present)
        let display_text = line_text.replace(COLUMN_BREAK_CHAR, "");
        let width = self
            .font_provider
            .measure_text(display_text.trim_end(), &font, style.font_size);
        let run = LayoutRun::new(display_text, indent, width, style);
        line.add_run(run);

        let consumed = if has_column_break {
            line_text.len() + COLUMN_BREAK_CHAR.len_utf8()
        } else {
            match break_point {
                Some(bp) => bp,
                None => text.len(),
            }
        };

        (line, consumed, has_column_break)
    }

    /// Layout a single line of text (backward compatibility).
    #[allow(dead_code)]
    fn layout_line(
        &self,
        text: &str,
        available_width: f64,
        para_idx: usize,
        y: f64,
        indent: f64,
    ) -> (LayoutLine, usize) {
        let (line, consumed, _) =
            self.layout_line_with_breaks(text, available_width, para_idx, y, indent);
        (line, consumed)
    }

    /// Balance columns by redistributing lines to achieve similar heights.
    fn balance_columns(&self, layout: &mut TextFrameLayout) {
        if layout.columns.len() < 2 {
            return;
        }

        // Collect all lines from all columns
        let all_lines: Vec<LayoutLine> =
            layout.columns.iter().flat_map(|c| c.lines.clone()).collect();

        if all_lines.is_empty() {
            return;
        }

        // Calculate total height of all lines
        let total_height: f64 = all_lines.iter().map(|l| l.height).sum();
        let target_height = total_height / layout.columns.len() as f64;

        // Clear all columns
        for col in &mut layout.columns {
            col.lines.clear();
        }

        // Redistribute lines
        let mut current_col = 0;
        let mut current_height = 0.0;
        let mut current_y = 0.0;

        for mut line in all_lines {
            // Check if we should move to next column
            if current_col < layout.columns.len() - 1 {
                let would_be_height = current_height + line.height;
                // Move to next column if we've exceeded target and there are more columns
                if current_height > 0.0
                    && would_be_height > target_height + self.column_balance.tolerance
                {
                    current_col += 1;
                    current_height = 0.0;
                    current_y = 0.0;
                }
            }

            // Update line position for new column
            line.baseline_y = current_y + line.ascent;
            current_y += line.height;
            current_height += line.height;

            layout.columns[current_col].lines.push(line);
        }
    }

    /// Apply hanging punctuation to laid out lines.
    fn apply_hanging_punctuation(&self, layout: &mut TextFrameLayout) {
        for column in &mut layout.columns {
            for line in &mut column.lines {
                for run in &mut line.runs {
                    if run.text.is_empty() {
                        continue;
                    }

                    // Check for left-hanging punctuation
                    if self.hanging_punctuation.left {
                        if let Some(first_char) = run.text.chars().next() {
                            if LEFT_HANG_PUNCTUATION.contains(&first_char) {
                                let hang_width = self.measure_char(first_char);
                                let offset = hang_width * self.hanging_punctuation.hang_ratio;
                                run.x -= offset;
                            }
                        }
                    }

                    // Check for right-hanging punctuation
                    if self.hanging_punctuation.right {
                        if let Some(last_char) = run.text.chars().last() {
                            if RIGHT_HANG_PUNCTUATION.contains(&last_char) {
                                // Right-hanging punctuation extends past the margin.
                                // The run width stays the same, but the punctuation
                                // is considered to extend past the margin.
                                let _hang_width = self.measure_char(last_char);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Measure a single character's width.
    fn measure_char(&self, c: char) -> f64 {
        let style = CharacterStyle::default();
        let font = self.font_provider.default_font();
        let s = c.to_string();
        self.font_provider.measure_text(&s, &font, style.font_size)
    }

    /// Apply alignment to lines.
    pub fn apply_alignment(&self, layout: &mut TextFrameLayout, alignment: TextAlignment) {
        for column in &mut layout.columns {
            let col_width = column.bounds.width();

            for line in &mut column.lines {
                match alignment {
                    TextAlignment::Left => {
                        // No adjustment needed for left alignment
                    }
                    TextAlignment::Center => {
                        let offset = (col_width - line.width) / 2.0;
                        for run in &mut line.runs {
                            run.x += offset;
                        }
                    }
                    TextAlignment::Right => {
                        let offset = col_width - line.width;
                        for run in &mut line.runs {
                            run.x += offset;
                        }
                    }
                    TextAlignment::Justify => {
                        // Don't justify the last line of a paragraph or lines with hard breaks
                        if line.is_last_line || line.hard_break {
                            // Left-align the last line
                            continue;
                        }

                        self.apply_justify_spacing(line, col_width);
                    }
                };
            }
        }
    }

    /// Apply justified spacing to a line by distributing extra space between words.
    fn apply_justify_spacing(&self, line: &mut LayoutLine, available_width: f64) {
        // Count word gaps (spaces between words)
        let gap_count = self.count_word_gaps(line);
        if gap_count == 0 {
            return; // Can't justify a single word
        }

        // Calculate extra space needed
        let extra_space = available_width - line.width;
        if extra_space <= 0.0 {
            return; // Line is already at or beyond width
        }

        // Space to add at each gap
        let space_per_gap = extra_space / gap_count as f64;

        // Apply extra spacing by adjusting run positions and glyph positions
        let mut accumulated_offset = 0.0;

        for run in &mut line.runs {
            // Offset the run start position
            run.x += accumulated_offset;

            // Count and adjust gaps within this run
            let mut new_glyphs = Vec::new();
            let mut glyph_offset = 0.0;
            let mut prev_was_space = false;

            for glyph in &run.glyphs {
                let is_space = glyph.char.is_whitespace();
                if is_space && !prev_was_space {
                    // Add extra space after word boundary
                    glyph_offset += space_per_gap;
                    accumulated_offset += space_per_gap;
                }

                let mut new_glyph = glyph.clone();
                new_glyph.x += glyph_offset;
                new_glyphs.push(new_glyph);
                prev_was_space = is_space;
            }

            run.glyphs = new_glyphs;
            run.width += glyph_offset;
        }

        // Update line width
        line.width = available_width;
    }

    /// Count word gaps (transitions from non-space to space) in a line.
    fn count_word_gaps(&self, line: &LayoutLine) -> usize {
        let mut count = 0;
        let mut in_word = false;

        for run in &line.runs {
            for c in run.text.chars() {
                if c.is_whitespace() {
                    if in_word {
                        count += 1;
                        in_word = false;
                    }
                } else {
                    in_word = true;
                }
            }
        }

        count
    }

    /// Apply vertical alignment to the layout.
    pub fn apply_vertical_alignment(
        &self,
        layout: &mut TextFrameLayout,
        alignment: VerticalAlignment,
    ) {
        layout.vertical_alignment = alignment;

        if matches!(alignment, VerticalAlignment::Top) {
            return; // No adjustment needed for top alignment
        }

        for column in &mut layout.columns {
            let content_height = column.content_height();
            let available_height = column.bounds.height();

            if content_height >= available_height {
                continue; // No room to adjust
            }

            let offset = match alignment {
                VerticalAlignment::Top => 0.0,
                VerticalAlignment::Center => (available_height - content_height) / 2.0,
                VerticalAlignment::Bottom => available_height - content_height,
            };

            // Offset all line baselines
            for line in &mut column.lines {
                line.baseline_y += offset;
            }
        }
    }

    /// Process tab characters in text and position content according to tab stops.
    pub fn process_tabs<F>(
        &self,
        text: &str,
        tab_stops: &[TabStop],
        current_x: f64,
        measure: F,
    ) -> Vec<(String, f64)>
    where
        F: Fn(&str) -> f64,
    {
        let mut result = Vec::new();
        let mut x = current_x;
        let default_tab_width = 36.0; // Default 0.5 inch tab stops

        for segment in text.split('\t') {
            if !result.is_empty() {
                // Find the next tab stop position
                let tab_pos = self.find_next_tab_stop(x, tab_stops, default_tab_width);
                x = tab_pos;
            }

            if !segment.is_empty() {
                let segment_width = measure(segment);
                result.push((segment.to_string(), x));
                x += segment_width;
            }
        }

        result
    }

    /// Find the next tab stop position from the current x position.
    fn find_next_tab_stop(&self, current_x: f64, tab_stops: &[TabStop], default_width: f64) -> f64 {
        // Find the first tab stop that's past our current position
        for tab in tab_stops {
            if tab.position > current_x {
                return tab.position;
            }
        }

        // No explicit tab stop found, use default tab width
        ((current_x / default_width).floor() + 1.0) * default_width
    }

    /// Apply tab alignment to positioned text segments.
    pub fn apply_tab_alignment<F>(
        &self,
        segment: &str,
        segment_x: f64,
        tab_stop: &TabStop,
        measure: F,
    ) -> f64
    where
        F: Fn(&str) -> f64,
    {
        let segment_width = measure(segment);

        match tab_stop.alignment {
            TabAlignment::Left => segment_x,
            TabAlignment::Center => segment_x - segment_width / 2.0,
            TabAlignment::Right => segment_x - segment_width,
            TabAlignment::Decimal => {
                // Find the decimal character position
                if let Some(decimal_pos) = segment.find(tab_stop.decimal_char) {
                    let before_decimal = &segment[..decimal_pos];
                    let before_width = measure(before_decimal);
                    segment_x - before_width
                } else {
                    // No decimal found, treat as right-aligned
                    segment_x - segment_width
                }
            }
        }
    }

    /// Truncate text with ellipsis when overflow occurs.
    pub fn truncate_with_ellipsis<F>(
        &self,
        text: &str,
        available_width: f64,
        options: &TruncationOptions,
        measure: F,
    ) -> (String, bool)
    where
        F: Fn(&str) -> f64,
    {
        if !options.enabled {
            return (text.to_string(), false);
        }

        let text_width = measure(text);
        if text_width <= available_width {
            return (text.to_string(), false);
        }

        let ellipsis_width = measure(&options.ellipsis);
        let target_width = available_width - ellipsis_width;

        if target_width <= 0.0 {
            // Not even room for ellipsis
            return (options.ellipsis.clone(), true);
        }

        match options.position {
            TruncationPosition::End => {
                let truncated = self.truncate_to_width_end(text, target_width, &measure);
                (format!("{}{}", truncated, options.ellipsis), true)
            }
            TruncationPosition::Start => {
                let truncated = self.truncate_to_width_start(text, target_width, &measure);
                (format!("{}{}", options.ellipsis, truncated), true)
            }
            TruncationPosition::Middle => {
                let half_width = target_width / 2.0;
                let start = self.truncate_to_width_end(text, half_width, &measure);
                let end = self.truncate_to_width_start(text, half_width, &measure);
                (format!("{}{}{}", start, options.ellipsis, end), true)
            }
        }
    }

    /// Truncate text from the end to fit within a width.
    fn truncate_to_width_end<F>(&self, text: &str, max_width: f64, measure: &F) -> String
    where
        F: Fn(&str) -> f64,
    {
        let mut result = String::new();
        let mut current_width = 0.0;

        for c in text.chars() {
            let char_str: String = c.to_string();
            let char_width = measure(&char_str);

            if current_width + char_width > max_width {
                break;
            }

            result.push(c);
            current_width += char_width;
        }

        // Try to break at a word boundary if possible
        if let Some(last_space) = result.rfind(char::is_whitespace) {
            if last_space > result.len() / 2 {
                result.truncate(last_space);
            }
        }

        result.trim_end().to_string()
    }

    /// Truncate text from the start to fit within a width.
    fn truncate_to_width_start<F>(&self, text: &str, max_width: f64, measure: &F) -> String
    where
        F: Fn(&str) -> f64,
    {
        let chars: Vec<char> = text.chars().collect();
        let mut result = String::new();
        let mut current_width = 0.0;

        for c in chars.iter().rev() {
            let char_str: String = c.to_string();
            let char_width = measure(&char_str);

            if current_width + char_width > max_width {
                break;
            }

            result.insert(0, *c);
            current_width += char_width;
        }

        result.trim_start().to_string()
    }

    /// Apply truncation to a layout when overflow is detected.
    pub fn apply_truncation<F>(
        &self,
        layout: &mut TextFrameLayout,
        options: &TruncationOptions,
        measure: F,
    ) where
        F: Fn(&str) -> f64,
    {
        if !options.enabled || !layout.has_overflow {
            return;
        }

        // Find the last column that has content
        for column in layout.columns.iter_mut().rev() {
            if column.lines.is_empty() {
                continue;
            }

            // Get the last line in this column
            if let Some(last_line) = column.lines.last_mut() {
                let col_width = column.bounds.width();

                // Truncate the last run
                if let Some(last_run) = last_line.runs.last_mut() {
                    let (truncated, was_truncated) = self.truncate_with_ellipsis(
                        &last_run.text,
                        col_width - last_run.x,
                        options,
                        &measure,
                    );

                    if was_truncated {
                        last_run.text = truncated.clone();
                        last_run.width = measure(&truncated);
                        last_line.truncated = true;
                        layout.was_truncated = true;
                        layout.has_overflow = false; // Overflow handled by truncation
                    }
                }
            }

            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fontdue::Font;
    use repub_fonts::FontProvider;
    use std::sync::Arc;

    struct MockFontProvider;
    impl FontProvider for MockFontProvider {
        fn get_font(&self, _: &str, _: u16, _: bool) -> Option<Arc<Font>> {
            None
        }
        fn default_font(&self) -> Arc<Font> {
            Arc::new(
                Font::from_bytes(
                    include_bytes!("../../../assets/fonts/fallback.ttf").to_vec(),
                    fontdue::FontSettings::default(),
                )
                .unwrap(),
            )
        }
    }

    #[test]
    fn test_layout_line_creation() {
        let line = LayoutLine::new(10.0, 0);
        assert_eq!(line.baseline_y, 10.0);
        assert_eq!(line.paragraph_index, 0);
        assert_eq!(line.char_count(), 0);
    }

    #[test]
    fn test_layout_run() {
        let run = LayoutRun::new("Hello".to_string(), 0.0, 50.0, CharacterStyle::default());
        assert_eq!(run.text, "Hello");
        assert!((run.width - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_column_bounds_single() {
        let layouter = TextLayouter::new(&MockFontProvider);

        let content = Rect::new(0.0, 0.0, 300.0, 400.0);
        let bounds = layouter.compute_column_bounds(&ColumnSpec::single(), &content);

        assert_eq!(bounds.len(), 1);
        assert_eq!(bounds[0], content);
    }

    #[test]
    fn test_tab_stop_creation() {
        let left = TabStop::left(72.0);
        assert_eq!(left.position, 72.0);
        assert_eq!(left.alignment, TabAlignment::Left);

        let center = TabStop::center(144.0);
        assert_eq!(center.position, 144.0);
        assert_eq!(center.alignment, TabAlignment::Center);

        let right = TabStop::right(216.0);
        assert_eq!(right.position, 216.0);
        assert_eq!(right.alignment, TabAlignment::Right);

        let decimal = TabStop::decimal(288.0);
        assert_eq!(decimal.position, 288.0);
        assert_eq!(decimal.alignment, TabAlignment::Decimal);
        assert_eq!(decimal.decimal_char, '.');
    }

    #[test]
    fn test_tab_stop_with_custom_decimal_char() {
        let tab = TabStop::decimal_with_char(100.0, ',');
        assert_eq!(tab.decimal_char, ',');
        assert_eq!(tab.alignment, TabAlignment::Decimal);
    }

    #[test]
    fn test_process_tabs() {
        let layouter = TextLayouter::new(&MockFontProvider);
        let measure = |s: &str| s.chars().count() as f64 * 10.0;
        let tab_stops = vec![TabStop::left(100.0), TabStop::left(200.0)];

        let segments = layouter.process_tabs("Hello\tWorld", &tab_stops, 0.0, measure);

        assert_eq!(segments.len(), 2);
        assert_eq!(segments[0].0, "Hello");
        assert_eq!(segments[0].1, 0.0);
        assert_eq!(segments[1].0, "World");
        assert_eq!(segments[1].1, 100.0);
    }

    #[test]
    fn test_find_next_tab_stop() {
        let layouter = TextLayouter::new(&MockFontProvider);
        let tab_stops = vec![TabStop::left(72.0), TabStop::left(144.0), TabStop::left(216.0)];

        assert_eq!(layouter.find_next_tab_stop(0.0, &tab_stops, 36.0), 72.0);
        assert_eq!(layouter.find_next_tab_stop(73.0, &tab_stops, 36.0), 144.0);
        assert_eq!(layouter.find_next_tab_stop(220.0, &tab_stops, 36.0), 252.0);
    }

    #[test]
    fn test_tab_alignment() {
        let layouter = TextLayouter::new(&MockFontProvider);
        let measure = |s: &str| s.chars().count() as f64 * 10.0;

        let left_tab = TabStop::left(100.0);
        assert_eq!(
            layouter.apply_tab_alignment("Test", 100.0, &left_tab, &measure),
            100.0
        );

        let right_tab = TabStop::right(100.0);
        assert_eq!(
            layouter.apply_tab_alignment("Test", 100.0, &right_tab, &measure),
            60.0
        );

        let center_tab = TabStop::center(100.0);
        assert_eq!(
            layouter.apply_tab_alignment("Test", 100.0, &center_tab, &measure),
            80.0
        );

        let decimal_tab = TabStop::decimal(100.0);
        assert_eq!(
            layouter.apply_tab_alignment("12.34", 100.0, &decimal_tab, &measure),
            80.0
        );
    }

    #[test]
    fn test_vertical_alignment_enum() {
        assert_eq!(VerticalAlignment::default(), VerticalAlignment::Top);
    }

    #[test]
    fn test_apply_vertical_alignment_top() {
        let layouter = TextLayouter::new(&MockFontProvider);
        let bounds = Rect::new(0.0, 0.0, 200.0, 400.0);
        let mut layout = TextFrameLayout::new(bounds, bounds);

        let mut column = LayoutColumn::new(bounds);
        let mut line = LayoutLine::new(20.0, 0);
        line.height = 20.0;
        line.descent = 4.0;
        column.lines.push(line);
        layout.columns.push(column);

        layouter.apply_vertical_alignment(&mut layout, VerticalAlignment::Top);

        assert_eq!(layout.columns[0].lines[0].baseline_y, 20.0);
    }

    #[test]
    fn test_apply_vertical_alignment_center() {
        let layouter = TextLayouter::new(&MockFontProvider);
        let bounds = Rect::new(0.0, 0.0, 200.0, 400.0);
        let mut layout = TextFrameLayout::new(bounds, bounds);

        let mut column = LayoutColumn::new(bounds);
        let mut line = LayoutLine::new(20.0, 0);
        line.height = 20.0;
        line.descent = 4.0;
        column.lines.push(line);
        layout.columns.push(column);

        layouter.apply_vertical_alignment(&mut layout, VerticalAlignment::Center);

        assert!((layout.columns[0].lines[0].baseline_y - 208.0).abs() < 0.01);
    }

    #[test]
    fn test_apply_vertical_alignment_bottom() {
        let layouter = TextLayouter::new(&MockFontProvider);
        let bounds = Rect::new(0.0, 0.0, 200.0, 400.0);
        let mut layout = TextFrameLayout::new(bounds, bounds);

        let mut column = LayoutColumn::new(bounds);
        let mut line = LayoutLine::new(20.0, 0);
        line.height = 20.0;
        line.descent = 4.0;
        column.lines.push(line);
        layout.columns.push(column);

        layouter.apply_vertical_alignment(&mut layout, VerticalAlignment::Bottom);

        assert!((layout.columns[0].lines[0].baseline_y - 396.0).abs() < 0.01);
    }

    #[test]
    fn test_count_word_gaps() {
        let layouter = TextLayouter::new(&MockFontProvider);

        let mut line = LayoutLine::new(0.0, 0);
        let run = LayoutRun::new(
            "Hello world test".to_string(),
            0.0,
            100.0,
            CharacterStyle::default(),
        );
        line.runs.push(run);

        assert_eq!(layouter.count_word_gaps(&line), 2);
    }

    #[test]
    fn test_justify_does_not_apply_to_last_line() {
        let layouter = TextLayouter::new(&MockFontProvider);
        let bounds = Rect::new(0.0, 0.0, 200.0, 400.0);
        let mut layout = TextFrameLayout::new(bounds, bounds);

        let mut column = LayoutColumn::new(bounds);
        let mut line = LayoutLine::new(20.0, 0);
        line.width = 100.0;
        line.is_last_line = true;
        let run = LayoutRun::new("Last line".to_string(), 0.0, 100.0, CharacterStyle::default());
        line.runs.push(run);
        column.lines.push(line);
        layout.columns.push(column);

        let original_width = layout.columns[0].lines[0].width;

        layouter.apply_alignment(&mut layout, TextAlignment::Justify);

        assert_eq!(layout.columns[0].lines[0].width, original_width);
    }

    #[test]
    fn test_justify_does_not_apply_to_hard_break() {
        let layouter = TextLayouter::new(&MockFontProvider);
        let bounds = Rect::new(0.0, 0.0, 200.0, 400.0);
        let mut layout = TextFrameLayout::new(bounds, bounds);

        let mut column = LayoutColumn::new(bounds);
        let mut line = LayoutLine::new(20.0, 0);
        line.width = 100.0;
        line.hard_break = true;
        let run = LayoutRun::new(
            "Line with hard break\n".to_string(),
            0.0,
            100.0,
            CharacterStyle::default(),
        );
        line.runs.push(run);
        column.lines.push(line);
        layout.columns.push(column);

        let original_width = layout.columns[0].lines[0].width;

        layouter.apply_alignment(&mut layout, TextAlignment::Justify);

        assert_eq!(layout.columns[0].lines[0].width, original_width);
    }

    #[test]
    fn test_truncation_options_default() {
        let options = TruncationOptions::default();
        assert!(!options.enabled);
        assert_eq!(options.ellipsis, "…");
        assert_eq!(options.position, TruncationPosition::End);
    }

    #[test]
    fn test_truncation_options_ellipsis_end() {
        let options = TruncationOptions::ellipsis_end();
        assert!(options.enabled);
        assert_eq!(options.ellipsis, "…");
        assert_eq!(options.position, TruncationPosition::End);
    }

    #[test]
    fn test_truncation_options_with_ellipsis() {
        let options = TruncationOptions::with_ellipsis("...");
        assert!(options.enabled);
        assert_eq!(options.ellipsis, "...");
    }

    #[test]
    fn test_truncate_with_ellipsis_no_truncation_needed() {
        let layouter = TextLayouter::new(&MockFontProvider);
        let measure = |s: &str| s.chars().count() as f64 * 10.0;
        let options = TruncationOptions::ellipsis_end();

        let (result, truncated) = layouter.truncate_with_ellipsis("Hello", 100.0, &options, measure);

        assert_eq!(result, "Hello");
        assert!(!truncated);
    }

    #[test]
    fn test_truncate_with_ellipsis_end() {
        let layouter = TextLayouter::new(&MockFontProvider);
        let measure = |s: &str| s.chars().count() as f64 * 10.0;
        let options = TruncationOptions::ellipsis_end();

        let (result, truncated) =
            layouter.truncate_with_ellipsis("Hello World", 60.0, &options, measure);

        assert!(truncated);
        assert!(result.ends_with('…'));
        assert!(result.len() < "Hello World".len());
    }

    #[test]
    fn test_truncate_with_ellipsis_start() {
        let layouter = TextLayouter::new(&MockFontProvider);
        let measure = |s: &str| s.chars().count() as f64 * 10.0;
        let mut options = TruncationOptions::ellipsis_end();
        options.position = TruncationPosition::Start;

        let (result, truncated) =
            layouter.truncate_with_ellipsis("Hello World", 60.0, &options, measure);

        assert!(truncated);
        assert!(result.starts_with('…'));
    }

    #[test]
    fn test_truncate_with_ellipsis_middle() {
        let layouter = TextLayouter::new(&MockFontProvider);
        let measure = |s: &str| s.chars().count() as f64 * 10.0;
        let mut options = TruncationOptions::ellipsis_end();
        options.position = TruncationPosition::Middle;

        let (result, truncated) =
            layouter.truncate_with_ellipsis("Hello World Test", 80.0, &options, measure);

        assert!(truncated);
        assert!(result.contains('…'));
        let parts: Vec<&str> = result.split('…').collect();
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn test_truncate_disabled() {
        let layouter = TextLayouter::new(&MockFontProvider);
        let measure = |s: &str| s.chars().count() as f64 * 10.0;
        let options = TruncationOptions::default(); // disabled

        let (result, truncated) =
            layouter.truncate_with_ellipsis("Hello World", 60.0, &options, measure);

        assert_eq!(result, "Hello World");
        assert!(!truncated);
    }

    #[test]
    fn test_line_is_last_line_flag() {
        let mut line = LayoutLine::new(0.0, 0);
        assert!(!line.is_last_line);

        line.is_last_line = true;
        assert!(line.is_last_line);
    }

    #[test]
    fn test_line_truncated_flag() {
        let mut line = LayoutLine::new(0.0, 0);
        assert!(!line.truncated);

        line.truncated = true;
        assert!(line.truncated);
    }

    #[test]
    fn test_layout_frame_content_height() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 400.0);
        let mut layout = TextFrameLayout::new(bounds, bounds);

        let mut column = LayoutColumn::new(bounds);
        let mut line1 = LayoutLine::new(20.0, 0);
        line1.descent = 4.0;
        let mut line2 = LayoutLine::new(44.0, 0);
        line2.descent = 4.0;
        column.lines.push(line1);
        column.lines.push(line2);
        layout.columns.push(column);

        assert!((layout.content_height() - 48.0).abs() < 0.01);
    }

    #[test]
    fn test_column_balance_options_default() {
        let options = ColumnBalanceOptions::default();
        assert!(!options.enabled);
    }

    #[test]
    fn test_column_balance_options_enabled() {
        let options = ColumnBalanceOptions::enabled();
        assert!(options.enabled);
        assert_eq!(options.max_iterations, 10);
        assert!((options.tolerance - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_column_balancing() {
        let layouter = TextLayouter::new(&MockFontProvider)
            .with_column_balance(ColumnBalanceOptions::enabled());

        let bounds = Rect::new(0.0, 0.0, 400.0, 400.0);
        let mut layout = TextFrameLayout::new(bounds, bounds);

        let col1_bounds = Rect::new(0.0, 0.0, 190.0, 400.0);
        let col2_bounds = Rect::new(210.0, 0.0, 190.0, 400.0);

        let mut col1 = LayoutColumn::new(col1_bounds);
        let col2 = LayoutColumn::new(col2_bounds);

        for i in 0..4 {
            let mut line = LayoutLine::new(i as f64 * 20.0, 0);
            line.height = 20.0;
            line.ascent = 16.0;
            line.descent = 4.0;
            col1.lines.push(line);
        }

        layout.columns.push(col1);
        layout.columns.push(col2);

        layouter.balance_columns(&mut layout);

        assert!(layout.columns[0].lines.len() >= 1);
        assert!(layout.columns[1].lines.len() >= 1);
    }

    #[test]
    fn test_hanging_punctuation_default() {
        let config = HangingPunctuation::default();
        assert!(!config.left);
        assert!(!config.right);
    }

    #[test]
    fn test_hanging_punctuation_enabled() {
        let config = HangingPunctuation::enabled();
        assert!(config.left);
        assert!(config.right);
        assert!((config.hang_ratio - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_hanging_punctuation_partial() {
        let config = HangingPunctuation::partial(0.5);
        assert!(config.left);
        assert!(config.right);
        assert!((config.hang_ratio - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_hanging_punctuation_applied() {
        let layouter = TextLayouter::new(&MockFontProvider)
            .with_hanging_punctuation(HangingPunctuation::enabled());

        let bounds = Rect::new(0.0, 0.0, 200.0, 400.0);
        let mut layout = TextFrameLayout::new(bounds, bounds);

        let mut column = LayoutColumn::new(bounds);
        let mut line = LayoutLine::new(20.0, 0);

        let run = LayoutRun::new(
            "\u{201C}Hello World\u{201D}".to_string(),
            10.0,
            100.0,
            CharacterStyle::default(),
        );
        line.runs.push(run);
        column.lines.push(line);
        layout.columns.push(column);

        layouter.apply_hanging_punctuation(&mut layout);

        assert!(layout.columns[0].lines[0].runs[0].x < 10.0);
    }

    #[test]
    fn test_column_break_in_text() {
        use repub_model::shape::Shape;

        let layouter = TextLayouter::new(&MockFontProvider);

        // Create a 2-column text-frame shape with a column break
        let mut shape = Shape::text_frame(Rect::new(0.0, 0.0, 400.0, 400.0));
        let text = format!("Column 1{}Column 2", COLUMN_BREAK_CHAR);
        if let ShapeKind::TextFrame(data) = &mut shape.kind {
            data.columns = ColumnSpec::equal(2, 20.0);
            data.set_text(&text);
        }

        let layout = layouter.layout_shape(&shape).unwrap();

        assert_eq!(layout.columns.len(), 2);
        assert!(!layout.columns[0].lines.is_empty());
        assert!(!layout.columns[1].lines.is_empty());
    }

    #[test]
    fn test_line_frame_index() {
        let mut line = LayoutLine::new(0.0, 0);
        assert_eq!(line.frame_index, 0);
        assert_eq!(line.column_index, 0);

        line.frame_index = 2;
        line.column_index = 1;
        assert_eq!(line.frame_index, 2);
        assert_eq!(line.column_index, 1);
    }

    #[test]
    fn test_layout_frame_index() {
        let bounds = Rect::new(0.0, 0.0, 200.0, 400.0);
        let mut layout = TextFrameLayout::new(bounds, bounds);
        assert_eq!(layout.frame_index, 0);
        assert!(!layout.has_content);

        layout.frame_index = 1;
        layout.has_content = true;
        assert_eq!(layout.frame_index, 1);
        assert!(layout.has_content);
    }

    #[test]
    fn caret_rect_advances_with_offset() {
        use repub_model::shape::Shape;
        let layouter = TextLayouter::new(&MockFontProvider);
        let mut shape = Shape::text_frame(Rect::new(0.0, 0.0, 400.0, 100.0));
        if let ShapeKind::TextFrame(data) = &mut shape.kind {
            data.set_text("Hello");
        }
        let layout = layouter.layout_shape(&shape).unwrap();

        let c0 = layout.caret_rect(0, 0).expect("caret at start");
        let c3 = layout.caret_rect(0, 3).expect("caret at offset 3");
        assert!(c3.origin.x > c0.origin.x, "{} !> {}", c3.origin.x, c0.origin.x);
        assert!((c0.origin.y - c3.origin.y).abs() < 0.01, "same line");
        assert!(c0.height() > 0.0);
    }

    #[test]
    fn hit_test_round_trips_through_caret() {
        use repub_model::shape::Shape;
        let layouter = TextLayouter::new(&MockFontProvider);
        let mut shape = Shape::text_frame(Rect::new(0.0, 0.0, 400.0, 100.0));
        if let ShapeKind::TextFrame(data) = &mut shape.kind {
            data.set_text("Hello world");
        }
        let layout = layouter.layout_shape(&shape).unwrap();

        let c = layout.caret_rect(0, 4).expect("caret");
        let (para, offset) = layout
            .hit_test(c.origin.x, c.origin.y + c.height() / 2.0)
            .expect("hit");
        assert_eq!(para, 0);
        assert!((offset as i64 - 4).abs() <= 1, "offset {offset} near 4");
    }

    #[test]
    fn hit_test_far_left_is_line_start() {
        use repub_model::shape::Shape;
        let layouter = TextLayouter::new(&MockFontProvider);
        let mut shape = Shape::text_frame(Rect::new(0.0, 0.0, 400.0, 100.0));
        if let ShapeKind::TextFrame(data) = &mut shape.kind {
            data.set_text("Hello");
        }
        let layout = layouter.layout_shape(&shape).unwrap();
        let (para, offset) = layout.hit_test(-100.0, 10.0).expect("hit");
        assert_eq!((para, offset), (0, 0));
    }
}
