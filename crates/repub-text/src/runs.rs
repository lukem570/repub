//! Flattening styled text into shapeable runs.
//!
//! This turns a [`Paragraph`]'s inline elements into a flat sequence of
//! [`ShapeRun`]s: contiguous spans of text with one character style, plus
//! explicit tab and line-break markers. It is purely about content and
//! styling: no fonts, no measurement, no frame or column geometry. The frame
//! layouter in `repub-layout` consumes these runs and places them.

use repub_model::styles::CharacterStyle;
use repub_model::text::{InlineElement, Paragraph, StyledText};

/// A shapeable piece of a paragraph: either a styled text span or a marker.
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeRun {
    /// A contiguous span of text sharing one character style.
    Text {
        /// The text content.
        text: String,
        /// The character style applied to the whole span.
        style: CharacterStyle,
    },
    /// An explicit line break (soft return within a paragraph).
    LineBreak,
    /// A tab character.
    Tab,
}

impl ShapeRun {
    /// The plain text this run contributes (a tab is `\t`, a line break `\n`).
    pub fn plain_text(&self) -> &str {
        match self {
            ShapeRun::Text { text, .. } => text,
            ShapeRun::LineBreak => "\n",
            ShapeRun::Tab => "\t",
        }
    }
}

/// Flatten a paragraph's inline elements into shapeable runs.
///
/// Fields and inline images are rendered as their placeholder text, matching
/// [`Paragraph::plain_text`], so callers that only need text geometry get a
/// consistent view. Hyperlinks contribute their text with the link's style.
pub fn paragraph_runs(paragraph: &Paragraph) -> Vec<ShapeRun> {
    let mut runs = Vec::new();
    for element in &paragraph.elements {
        match element {
            InlineElement::Text(run) => runs.push(ShapeRun::Text {
                text: run.text.clone(),
                style: run.style.clone(),
            }),
            InlineElement::Link(link) => runs.push(ShapeRun::Text {
                text: link.text.clone(),
                style: link.style.clone(),
            }),
            InlineElement::Field(_) => runs.push(ShapeRun::Text {
                text: "[field]".to_string(),
                style: paragraph.style.default_char_style.clone(),
            }),
            InlineElement::Image(_) => runs.push(ShapeRun::Text {
                text: "[image]".to_string(),
                style: paragraph.style.default_char_style.clone(),
            }),
            InlineElement::LineBreak => runs.push(ShapeRun::LineBreak),
            InlineElement::Tab => runs.push(ShapeRun::Tab),
        }
    }
    runs
}

/// Flatten every paragraph of a styled-text block into per-paragraph runs.
pub fn styled_text_runs(text: &StyledText) -> Vec<Vec<ShapeRun>> {
    text.paragraphs.iter().map(paragraph_runs).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_model::text::{Field, TextRun};

    #[test]
    fn plain_paragraph_becomes_one_text_run() {
        let para = Paragraph::plain("Hello");
        let runs = paragraph_runs(&para);
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0].plain_text(), "Hello");
    }

    #[test]
    fn markers_and_fields_are_flattened() {
        let mut para = Paragraph::new();
        para.elements.push(InlineElement::Text(TextRun::new("Page ")));
        para.add_field(Field::page_number());
        para.add_tab();
        para.add_line_break();
        let runs = paragraph_runs(&para);
        assert_eq!(runs.len(), 4);
        assert!(matches!(runs[0], ShapeRun::Text { .. }));
        assert_eq!(runs[1].plain_text(), "[field]");
        assert_eq!(runs[2], ShapeRun::Tab);
        assert_eq!(runs[3], ShapeRun::LineBreak);
    }

    #[test]
    fn styled_text_flattens_per_paragraph() {
        let text = StyledText::plain("One\nTwo\nThree");
        let runs = styled_text_runs(&text);
        assert_eq!(runs.len(), 3);
        assert_eq!(runs[1][0].plain_text(), "Two");
    }
}
