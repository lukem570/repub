//! Serde DTOs for text-wrap settings.

use repub_model::styles::{TextWrap, TextWrapSide, TextWrapStyle};
use serde::{Deserialize, Serialize};

/// DTO for [`TextWrapStyle`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum TextWrapStyleDto {
    None,
    Square,
    Tight,
    Through,
    TopAndBottom,
}

impl From<TextWrapStyle> for TextWrapStyleDto {
    fn from(s: TextWrapStyle) -> Self {
        match s {
            TextWrapStyle::None => TextWrapStyleDto::None,
            TextWrapStyle::Square => TextWrapStyleDto::Square,
            TextWrapStyle::Tight => TextWrapStyleDto::Tight,
            TextWrapStyle::Through => TextWrapStyleDto::Through,
            TextWrapStyle::TopAndBottom => TextWrapStyleDto::TopAndBottom,
        }
    }
}

impl From<TextWrapStyleDto> for TextWrapStyle {
    fn from(s: TextWrapStyleDto) -> Self {
        match s {
            TextWrapStyleDto::None => TextWrapStyle::None,
            TextWrapStyleDto::Square => TextWrapStyle::Square,
            TextWrapStyleDto::Tight => TextWrapStyle::Tight,
            TextWrapStyleDto::Through => TextWrapStyle::Through,
            TextWrapStyleDto::TopAndBottom => TextWrapStyle::TopAndBottom,
        }
    }
}

/// DTO for [`TextWrapSide`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum TextWrapSideDto {
    Both,
    Left,
    Right,
    Largest,
}

impl From<TextWrapSide> for TextWrapSideDto {
    fn from(s: TextWrapSide) -> Self {
        match s {
            TextWrapSide::Both => TextWrapSideDto::Both,
            TextWrapSide::Left => TextWrapSideDto::Left,
            TextWrapSide::Right => TextWrapSideDto::Right,
            TextWrapSide::Largest => TextWrapSideDto::Largest,
        }
    }
}

impl From<TextWrapSideDto> for TextWrapSide {
    fn from(s: TextWrapSideDto) -> Self {
        match s {
            TextWrapSideDto::Both => TextWrapSide::Both,
            TextWrapSideDto::Left => TextWrapSide::Left,
            TextWrapSideDto::Right => TextWrapSide::Right,
            TextWrapSideDto::Largest => TextWrapSide::Largest,
        }
    }
}

/// DTO for [`TextWrap`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TextWrapDto {
    /// Wrap style.
    pub style: TextWrapStyleDto,
    /// Wrap side.
    pub side: TextWrapSideDto,
    /// Top margin.
    pub margin_top: f64,
    /// Bottom margin.
    pub margin_bottom: f64,
    /// Left margin.
    pub margin_left: f64,
    /// Right margin.
    pub margin_right: f64,
}

impl From<TextWrap> for TextWrapDto {
    fn from(w: TextWrap) -> Self {
        Self {
            style: w.style.into(),
            side: w.side.into(),
            margin_top: w.margin_top,
            margin_bottom: w.margin_bottom,
            margin_left: w.margin_left,
            margin_right: w.margin_right,
        }
    }
}

impl From<TextWrapDto> for TextWrap {
    fn from(w: TextWrapDto) -> Self {
        TextWrap {
            style: w.style.into(),
            side: w.side.into(),
            margin_top: w.margin_top,
            margin_bottom: w.margin_bottom,
            margin_left: w.margin_left,
            margin_right: w.margin_right,
        }
    }
}
