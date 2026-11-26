//! Serde DTOs for the text content model.

use repub_model::text::{
    Field, Hyperlink, InlineElement, InlineImage, Paragraph, StyledText, TextRun,
};
use serde::{Deserialize, Serialize};

use super::styles::{CharacterStyleDto, ParagraphStyleDto};

/// DTO for [`TextRun`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRunDto {
    /// Text content.
    pub text: String,
    /// Character style.
    pub style: CharacterStyleDto,
}

impl From<TextRun> for TextRunDto {
    fn from(r: TextRun) -> Self {
        Self {
            text: r.text,
            style: r.style.into(),
        }
    }
}

impl From<TextRunDto> for TextRun {
    fn from(r: TextRunDto) -> Self {
        TextRun {
            text: r.text,
            style: r.style.into(),
        }
    }
}

/// DTO for [`Field`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum FieldDto {
    PageNumber,
    PageCount,
    Date { format: String },
    Time { format: String },
    FileName,
    Custom { name: String, default: String },
}

impl From<Field> for FieldDto {
    fn from(f: Field) -> Self {
        match f {
            Field::PageNumber => FieldDto::PageNumber,
            Field::PageCount => FieldDto::PageCount,
            Field::Date { format } => FieldDto::Date { format },
            Field::Time { format } => FieldDto::Time { format },
            Field::FileName => FieldDto::FileName,
            Field::Custom { name, default } => FieldDto::Custom { name, default },
        }
    }
}

impl From<FieldDto> for Field {
    fn from(f: FieldDto) -> Self {
        match f {
            FieldDto::PageNumber => Field::PageNumber,
            FieldDto::PageCount => Field::PageCount,
            FieldDto::Date { format } => Field::Date { format },
            FieldDto::Time { format } => Field::Time { format },
            FieldDto::FileName => Field::FileName,
            FieldDto::Custom { name, default } => Field::Custom { name, default },
        }
    }
}

/// DTO for [`InlineImage`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InlineImageDto {
    /// Image source.
    pub source: String,
    /// Width in points.
    pub width: f64,
    /// Height in points.
    pub height: f64,
    /// Alt text.
    pub alt_text: Option<String>,
}

impl From<InlineImage> for InlineImageDto {
    fn from(i: InlineImage) -> Self {
        Self {
            source: i.source,
            width: i.width,
            height: i.height,
            alt_text: i.alt_text,
        }
    }
}

impl From<InlineImageDto> for InlineImage {
    fn from(i: InlineImageDto) -> Self {
        InlineImage {
            source: i.source,
            width: i.width,
            height: i.height,
            alt_text: i.alt_text,
        }
    }
}

/// DTO for [`Hyperlink`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperlinkDto {
    /// Target URL or reference.
    pub target: String,
    /// Link text.
    pub text: String,
    /// External flag.
    pub external: bool,
    /// Character style.
    pub style: CharacterStyleDto,
}

impl From<Hyperlink> for HyperlinkDto {
    fn from(h: Hyperlink) -> Self {
        Self {
            target: h.target,
            text: h.text,
            external: h.external,
            style: h.style.into(),
        }
    }
}

impl From<HyperlinkDto> for Hyperlink {
    fn from(h: HyperlinkDto) -> Self {
        Hyperlink {
            target: h.target,
            text: h.text,
            external: h.external,
            style: h.style.into(),
        }
    }
}

/// DTO for [`InlineElement`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum InlineElementDto {
    Text(TextRunDto),
    Field(FieldDto),
    Image(InlineImageDto),
    Link(HyperlinkDto),
    LineBreak,
    Tab,
}

impl From<InlineElement> for InlineElementDto {
    fn from(e: InlineElement) -> Self {
        match e {
            InlineElement::Text(r) => InlineElementDto::Text(r.into()),
            InlineElement::Field(f) => InlineElementDto::Field(f.into()),
            InlineElement::Image(i) => InlineElementDto::Image(i.into()),
            InlineElement::Link(l) => InlineElementDto::Link(l.into()),
            InlineElement::LineBreak => InlineElementDto::LineBreak,
            InlineElement::Tab => InlineElementDto::Tab,
        }
    }
}

impl From<InlineElementDto> for InlineElement {
    fn from(e: InlineElementDto) -> Self {
        match e {
            InlineElementDto::Text(r) => InlineElement::Text(r.into()),
            InlineElementDto::Field(f) => InlineElement::Field(f.into()),
            InlineElementDto::Image(i) => InlineElement::Image(i.into()),
            InlineElementDto::Link(l) => InlineElement::Link(l.into()),
            InlineElementDto::LineBreak => InlineElement::LineBreak,
            InlineElementDto::Tab => InlineElement::Tab,
        }
    }
}

/// DTO for [`Paragraph`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphDto {
    /// Inline elements.
    pub elements: Vec<InlineElementDto>,
    /// Paragraph style.
    pub style: ParagraphStyleDto,
}

impl From<Paragraph> for ParagraphDto {
    fn from(p: Paragraph) -> Self {
        Self {
            elements: p.elements.into_iter().map(Into::into).collect(),
            style: p.style.into(),
        }
    }
}

impl From<ParagraphDto> for Paragraph {
    fn from(p: ParagraphDto) -> Self {
        Paragraph {
            elements: p.elements.into_iter().map(Into::into).collect(),
            style: p.style.into(),
        }
    }
}

/// DTO for [`StyledText`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StyledTextDto {
    /// Paragraphs.
    pub paragraphs: Vec<ParagraphDto>,
}

impl From<StyledText> for StyledTextDto {
    fn from(t: StyledText) -> Self {
        Self {
            paragraphs: t.paragraphs.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<StyledTextDto> for StyledText {
    fn from(t: StyledTextDto) -> Self {
        StyledText {
            paragraphs: t.paragraphs.into_iter().map(Into::into).collect(),
        }
    }
}
