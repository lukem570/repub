//! Text content model with styling.

use crate::styles::{CharacterStyle, ParagraphStyle};

/// A run of text with uniform styling.
#[derive(Debug, Clone, PartialEq)]
pub struct TextRun {
    /// The text content
    pub text: String,
    /// Character-level styling
    pub style: CharacterStyle,
}

impl TextRun {
    /// Create a new text run with default styling.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: CharacterStyle::default(),
        }
    }

    /// Create a text run with specific styling.
    pub fn styled(text: impl Into<String>, style: CharacterStyle) -> Self {
        Self {
            text: text.into(),
            style,
        }
    }

    /// Get the length of the text in characters.
    pub fn len(&self) -> usize {
        self.text.chars().count()
    }

    /// Check if the run is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }
}

/// An inline element that can appear within text.
#[derive(Debug, Clone, PartialEq)]
pub enum InlineElement {
    /// Regular text run
    Text(TextRun),
    /// A field that displays dynamic content
    Field(Field),
    /// An inline image
    Image(InlineImage),
    /// A hyperlink (wraps other inline elements)
    Link(Hyperlink),
    /// A line break
    LineBreak,
    /// A tab character
    Tab,
}

/// A dynamic field that displays computed content.
#[derive(Debug, Clone, PartialEq)]
pub enum Field {
    /// Current page number
    PageNumber,
    /// Total page count
    PageCount,
    /// Current date
    Date {
        /// Date format string
        format: String,
    },
    /// Current time
    Time {
        /// Time format string
        format: String,
    },
    /// Document file name
    FileName,
    /// Custom field with a name
    Custom {
        /// Field name
        name: String,
        /// Default value
        default: String,
    },
}

impl Field {
    /// Create a page number field.
    pub fn page_number() -> Self {
        Field::PageNumber
    }

    /// Create a page count field.
    pub fn page_count() -> Self {
        Field::PageCount
    }

    /// Create a date field with the given format.
    pub fn date(format: impl Into<String>) -> Self {
        Field::Date {
            format: format.into(),
        }
    }

    /// Create a time field with the given format.
    pub fn time(format: impl Into<String>) -> Self {
        Field::Time {
            format: format.into(),
        }
    }
}

/// An image that appears inline with text.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineImage {
    /// Image source (path or embedded data)
    pub source: String,
    /// Width in points
    pub width: f64,
    /// Height in points
    pub height: f64,
    /// Alternative text
    pub alt_text: Option<String>,
}

/// A hyperlink wrapping text content.
#[derive(Debug, Clone, PartialEq)]
pub struct Hyperlink {
    /// Link destination (URL or internal reference)
    pub target: String,
    /// Link text
    pub text: String,
    /// Whether the link is external
    pub external: bool,
    /// Character style for the link
    pub style: CharacterStyle,
}

impl Hyperlink {
    /// Create an external hyperlink.
    pub fn external(url: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            target: url.into(),
            text: text.into(),
            external: true,
            style: CharacterStyle::default(),
        }
    }

    /// Create an internal hyperlink (to a page or anchor).
    pub fn internal(reference: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            target: reference.into(),
            text: text.into(),
            external: false,
            style: CharacterStyle::default(),
        }
    }
}

/// A paragraph of text.
#[derive(Debug, Clone, PartialEq)]
pub struct Paragraph {
    /// Inline elements (text runs, fields, etc.)
    pub elements: Vec<InlineElement>,
    /// Paragraph-level styling
    pub style: ParagraphStyle,
}

impl Paragraph {
    /// Create an empty paragraph.
    pub fn new() -> Self {
        Self {
            elements: Vec::new(),
            style: ParagraphStyle::default(),
        }
    }

    /// Create a paragraph with plain text.
    pub fn plain(text: &str) -> Self {
        Self {
            elements: vec![InlineElement::Text(TextRun::new(text))],
            style: ParagraphStyle::default(),
        }
    }

    /// Create a paragraph with a styled text run.
    pub fn styled(text: &str, style: CharacterStyle) -> Self {
        Self {
            elements: vec![InlineElement::Text(TextRun::styled(text, style))],
            style: ParagraphStyle::default(),
        }
    }

    /// Add a text run to the paragraph.
    pub fn add_text(&mut self, text: &str) {
        self.elements.push(InlineElement::Text(TextRun::new(text)));
    }

    /// Add a styled text run to the paragraph.
    pub fn add_styled_text(&mut self, text: &str, style: CharacterStyle) {
        self.elements
            .push(InlineElement::Text(TextRun::styled(text, style)));
    }

    /// Add a field to the paragraph.
    pub fn add_field(&mut self, field: Field) {
        self.elements.push(InlineElement::Field(field));
    }

    /// Add a line break.
    pub fn add_line_break(&mut self) {
        self.elements.push(InlineElement::LineBreak);
    }

    /// Add a tab.
    pub fn add_tab(&mut self) {
        self.elements.push(InlineElement::Tab);
    }

    /// Get the plain text content of the paragraph.
    pub fn plain_text(&self) -> String {
        let mut result = String::new();
        for element in &self.elements {
            match element {
                InlineElement::Text(run) => result.push_str(&run.text),
                InlineElement::Field(_) => result.push_str("[field]"),
                InlineElement::Image(_) => result.push_str("[image]"),
                InlineElement::Link(link) => result.push_str(&link.text),
                InlineElement::LineBreak => result.push('\n'),
                InlineElement::Tab => result.push('\t'),
            }
        }
        result
    }

    /// Check if the paragraph is empty.
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
            || self
                .elements
                .iter()
                .all(|e| matches!(e, InlineElement::Text(r) if r.is_empty()))
    }

    /// Set the paragraph style.
    pub fn with_style(mut self, style: ParagraphStyle) -> Self {
        self.style = style;
        self
    }
}

impl Default for Paragraph {
    fn default() -> Self {
        Self::new()
    }
}

/// A collection of styled text paragraphs.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StyledText {
    /// The paragraphs in this text block
    pub paragraphs: Vec<Paragraph>,
}

impl StyledText {
    /// Create an empty styled text block.
    pub fn new() -> Self {
        Self {
            paragraphs: Vec::new(),
        }
    }

    /// Create styled text from plain text (splits on newlines).
    pub fn plain(text: &str) -> Self {
        let paragraphs = text.split('\n').map(Paragraph::plain).collect();
        Self { paragraphs }
    }

    /// Create styled text with a single paragraph.
    pub fn single(paragraph: Paragraph) -> Self {
        Self {
            paragraphs: vec![paragraph],
        }
    }

    /// Add a paragraph.
    pub fn add_paragraph(&mut self, paragraph: Paragraph) {
        self.paragraphs.push(paragraph);
    }

    /// Add plain text as a new paragraph.
    pub fn add_plain(&mut self, text: &str) {
        self.paragraphs.push(Paragraph::plain(text));
    }

    /// Get the plain text content of all paragraphs.
    pub fn plain_text(&self) -> String {
        self.paragraphs
            .iter()
            .map(|p| p.plain_text())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Check if the text is empty.
    pub fn is_empty(&self) -> bool {
        self.paragraphs.is_empty() || self.paragraphs.iter().all(|p| p.is_empty())
    }

    /// Get the number of paragraphs.
    pub fn paragraph_count(&self) -> usize {
        self.paragraphs.len()
    }

    /// Get a paragraph by index.
    pub fn paragraph(&self, index: usize) -> Option<&Paragraph> {
        self.paragraphs.get(index)
    }

    /// Get a mutable paragraph by index.
    pub fn paragraph_mut(&mut self, index: usize) -> Option<&mut Paragraph> {
        self.paragraphs.get_mut(index)
    }

    /// Count total characters across all paragraphs.
    pub fn char_count(&self) -> usize {
        self.plain_text().chars().count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_run() {
        let run = TextRun::new("Hello, World!");
        assert_eq!(run.len(), 13);
        assert!(!run.is_empty());
    }

    #[test]
    fn test_paragraph_plain() {
        let para = Paragraph::plain("Test paragraph");
        assert_eq!(para.plain_text(), "Test paragraph");
    }

    #[test]
    fn test_styled_text_from_plain() {
        let text = StyledText::plain("Line 1\nLine 2\nLine 3");
        assert_eq!(text.paragraph_count(), 3);
        assert_eq!(text.plain_text(), "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn test_styled_text_add_paragraph() {
        let mut text = StyledText::new();
        text.add_plain("First");
        text.add_plain("Second");
        assert_eq!(text.paragraph_count(), 2);
    }

    #[test]
    fn test_paragraph_with_field() {
        let mut para = Paragraph::plain("Page ");
        para.add_field(Field::page_number());
        para.add_text(" of ");
        para.add_field(Field::page_count());
        assert_eq!(para.plain_text(), "Page [field] of [field]");
    }
}
