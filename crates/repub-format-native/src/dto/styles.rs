//! Serde DTOs for model style types.

use repub_model::styles::{
    BaselineShift, Bevel, BevelStyle, BevelTechnique, CharacterStyle, DropShadow, Effects, Fill,
    FontSpec, FontStretch, FontStyle, FontWeight, Glow, InnerGlow, InnerGlowSource, InnerShadow,
    LineCap, LineJoin, LineSpacing, ListStyle, NumberFormat, ParagraphStyle, Reflection, SoftEdges,
    StrikethroughStyle, Stroke, TextAlignment, TexturePattern, TileMode, UnderlineStyle,
};
use serde::{Deserialize, Serialize};

use super::geometry::{ColorDto, LinearGradientDto, PatternDto, RadialGradientDto};

/// DTO for [`FontWeight`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum FontWeightDto {
    Thin,
    ExtraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    Black,
}

impl From<FontWeight> for FontWeightDto {
    fn from(w: FontWeight) -> Self {
        match w {
            FontWeight::Thin => FontWeightDto::Thin,
            FontWeight::ExtraLight => FontWeightDto::ExtraLight,
            FontWeight::Light => FontWeightDto::Light,
            FontWeight::Normal => FontWeightDto::Normal,
            FontWeight::Medium => FontWeightDto::Medium,
            FontWeight::SemiBold => FontWeightDto::SemiBold,
            FontWeight::Bold => FontWeightDto::Bold,
            FontWeight::ExtraBold => FontWeightDto::ExtraBold,
            FontWeight::Black => FontWeightDto::Black,
        }
    }
}

impl From<FontWeightDto> for FontWeight {
    fn from(w: FontWeightDto) -> Self {
        match w {
            FontWeightDto::Thin => FontWeight::Thin,
            FontWeightDto::ExtraLight => FontWeight::ExtraLight,
            FontWeightDto::Light => FontWeight::Light,
            FontWeightDto::Normal => FontWeight::Normal,
            FontWeightDto::Medium => FontWeight::Medium,
            FontWeightDto::SemiBold => FontWeight::SemiBold,
            FontWeightDto::Bold => FontWeight::Bold,
            FontWeightDto::ExtraBold => FontWeight::ExtraBold,
            FontWeightDto::Black => FontWeight::Black,
        }
    }
}

/// DTO for [`FontStyle`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum FontStyleDto {
    Normal,
    Italic,
    Oblique,
}

impl From<FontStyle> for FontStyleDto {
    fn from(s: FontStyle) -> Self {
        match s {
            FontStyle::Normal => FontStyleDto::Normal,
            FontStyle::Italic => FontStyleDto::Italic,
            FontStyle::Oblique => FontStyleDto::Oblique,
        }
    }
}

impl From<FontStyleDto> for FontStyle {
    fn from(s: FontStyleDto) -> Self {
        match s {
            FontStyleDto::Normal => FontStyle::Normal,
            FontStyleDto::Italic => FontStyle::Italic,
            FontStyleDto::Oblique => FontStyle::Oblique,
        }
    }
}

/// DTO for [`FontStretch`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum FontStretchDto {
    UltraCondensed,
    ExtraCondensed,
    Condensed,
    SemiCondensed,
    Normal,
    SemiExpanded,
    Expanded,
    ExtraExpanded,
    UltraExpanded,
}

impl From<FontStretch> for FontStretchDto {
    fn from(s: FontStretch) -> Self {
        match s {
            FontStretch::UltraCondensed => FontStretchDto::UltraCondensed,
            FontStretch::ExtraCondensed => FontStretchDto::ExtraCondensed,
            FontStretch::Condensed => FontStretchDto::Condensed,
            FontStretch::SemiCondensed => FontStretchDto::SemiCondensed,
            FontStretch::Normal => FontStretchDto::Normal,
            FontStretch::SemiExpanded => FontStretchDto::SemiExpanded,
            FontStretch::Expanded => FontStretchDto::Expanded,
            FontStretch::ExtraExpanded => FontStretchDto::ExtraExpanded,
            FontStretch::UltraExpanded => FontStretchDto::UltraExpanded,
        }
    }
}

impl From<FontStretchDto> for FontStretch {
    fn from(s: FontStretchDto) -> Self {
        match s {
            FontStretchDto::UltraCondensed => FontStretch::UltraCondensed,
            FontStretchDto::ExtraCondensed => FontStretch::ExtraCondensed,
            FontStretchDto::Condensed => FontStretch::Condensed,
            FontStretchDto::SemiCondensed => FontStretch::SemiCondensed,
            FontStretchDto::Normal => FontStretch::Normal,
            FontStretchDto::SemiExpanded => FontStretch::SemiExpanded,
            FontStretchDto::Expanded => FontStretch::Expanded,
            FontStretchDto::ExtraExpanded => FontStretch::ExtraExpanded,
            FontStretchDto::UltraExpanded => FontStretch::UltraExpanded,
        }
    }
}

/// DTO for [`FontSpec`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FontSpecDto {
    /// Family name.
    pub family: String,
    /// Weight.
    pub weight: FontWeightDto,
    /// Style.
    pub style: FontStyleDto,
    /// Stretch.
    pub stretch: FontStretchDto,
}

impl From<FontSpec> for FontSpecDto {
    fn from(f: FontSpec) -> Self {
        Self {
            family: f.family,
            weight: f.weight.into(),
            style: f.style.into(),
            stretch: f.stretch.into(),
        }
    }
}

impl From<FontSpecDto> for FontSpec {
    fn from(f: FontSpecDto) -> Self {
        FontSpec {
            family: f.family,
            weight: f.weight.into(),
            style: f.style.into(),
            stretch: f.stretch.into(),
        }
    }
}

/// DTO for [`UnderlineStyle`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum UnderlineStyleDto {
    None,
    Single,
    Double,
    Dotted,
    Dashed,
    Wavy,
}

impl From<UnderlineStyle> for UnderlineStyleDto {
    fn from(u: UnderlineStyle) -> Self {
        match u {
            UnderlineStyle::None => UnderlineStyleDto::None,
            UnderlineStyle::Single => UnderlineStyleDto::Single,
            UnderlineStyle::Double => UnderlineStyleDto::Double,
            UnderlineStyle::Dotted => UnderlineStyleDto::Dotted,
            UnderlineStyle::Dashed => UnderlineStyleDto::Dashed,
            UnderlineStyle::Wavy => UnderlineStyleDto::Wavy,
        }
    }
}

impl From<UnderlineStyleDto> for UnderlineStyle {
    fn from(u: UnderlineStyleDto) -> Self {
        match u {
            UnderlineStyleDto::None => UnderlineStyle::None,
            UnderlineStyleDto::Single => UnderlineStyle::Single,
            UnderlineStyleDto::Double => UnderlineStyle::Double,
            UnderlineStyleDto::Dotted => UnderlineStyle::Dotted,
            UnderlineStyleDto::Dashed => UnderlineStyle::Dashed,
            UnderlineStyleDto::Wavy => UnderlineStyle::Wavy,
        }
    }
}

/// DTO for [`StrikethroughStyle`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum StrikethroughStyleDto {
    None,
    Single,
    Double,
}

impl From<StrikethroughStyle> for StrikethroughStyleDto {
    fn from(s: StrikethroughStyle) -> Self {
        match s {
            StrikethroughStyle::None => StrikethroughStyleDto::None,
            StrikethroughStyle::Single => StrikethroughStyleDto::Single,
            StrikethroughStyle::Double => StrikethroughStyleDto::Double,
        }
    }
}

impl From<StrikethroughStyleDto> for StrikethroughStyle {
    fn from(s: StrikethroughStyleDto) -> Self {
        match s {
            StrikethroughStyleDto::None => StrikethroughStyle::None,
            StrikethroughStyleDto::Single => StrikethroughStyle::Single,
            StrikethroughStyleDto::Double => StrikethroughStyle::Double,
        }
    }
}

/// DTO for [`BaselineShift`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum BaselineShiftDto {
    Normal,
    Superscript,
    Subscript,
}

impl From<BaselineShift> for BaselineShiftDto {
    fn from(b: BaselineShift) -> Self {
        match b {
            BaselineShift::Normal => BaselineShiftDto::Normal,
            BaselineShift::Superscript => BaselineShiftDto::Superscript,
            BaselineShift::Subscript => BaselineShiftDto::Subscript,
        }
    }
}

impl From<BaselineShiftDto> for BaselineShift {
    fn from(b: BaselineShiftDto) -> Self {
        match b {
            BaselineShiftDto::Normal => BaselineShift::Normal,
            BaselineShiftDto::Superscript => BaselineShift::Superscript,
            BaselineShiftDto::Subscript => BaselineShift::Subscript,
        }
    }
}

/// DTO for [`CharacterStyle`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CharacterStyleDto {
    /// Font specification.
    pub font: FontSpecDto,
    /// Font size in points.
    pub font_size: f64,
    /// Text color.
    pub color: ColorDto,
    /// Highlight/background color.
    pub background_color: Option<ColorDto>,
    /// Underline style.
    pub underline: UnderlineStyleDto,
    /// Underline color.
    pub underline_color: Option<ColorDto>,
    /// Strikethrough style.
    pub strikethrough: StrikethroughStyleDto,
    /// Baseline shift.
    pub baseline_shift: BaselineShiftDto,
    /// Letter spacing.
    pub letter_spacing: f64,
    /// All caps.
    pub all_caps: bool,
    /// Small caps.
    pub small_caps: bool,
}

impl From<CharacterStyle> for CharacterStyleDto {
    fn from(c: CharacterStyle) -> Self {
        Self {
            font: c.font.into(),
            font_size: c.font_size,
            color: c.color.into(),
            background_color: c.background_color.map(Into::into),
            underline: c.underline.into(),
            underline_color: c.underline_color.map(Into::into),
            strikethrough: c.strikethrough.into(),
            baseline_shift: c.baseline_shift.into(),
            letter_spacing: c.letter_spacing,
            all_caps: c.all_caps,
            small_caps: c.small_caps,
        }
    }
}

impl From<CharacterStyleDto> for CharacterStyle {
    fn from(c: CharacterStyleDto) -> Self {
        CharacterStyle {
            font: c.font.into(),
            font_size: c.font_size,
            color: c.color.into(),
            background_color: c.background_color.map(Into::into),
            underline: c.underline.into(),
            underline_color: c.underline_color.map(Into::into),
            strikethrough: c.strikethrough.into(),
            baseline_shift: c.baseline_shift.into(),
            letter_spacing: c.letter_spacing,
            all_caps: c.all_caps,
            small_caps: c.small_caps,
        }
    }
}

/// DTO for [`TextAlignment`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum TextAlignmentDto {
    Left,
    Center,
    Right,
    Justify,
}

impl From<TextAlignment> for TextAlignmentDto {
    fn from(a: TextAlignment) -> Self {
        match a {
            TextAlignment::Left => TextAlignmentDto::Left,
            TextAlignment::Center => TextAlignmentDto::Center,
            TextAlignment::Right => TextAlignmentDto::Right,
            TextAlignment::Justify => TextAlignmentDto::Justify,
        }
    }
}

impl From<TextAlignmentDto> for TextAlignment {
    fn from(a: TextAlignmentDto) -> Self {
        match a {
            TextAlignmentDto::Left => TextAlignment::Left,
            TextAlignmentDto::Center => TextAlignment::Center,
            TextAlignmentDto::Right => TextAlignment::Right,
            TextAlignmentDto::Justify => TextAlignment::Justify,
        }
    }
}

/// DTO for [`LineSpacing`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum LineSpacingDto {
    Single,
    OneAndHalf,
    Double,
    Multiple(f64),
    Exact(f64),
    AtLeast(f64),
}

impl From<LineSpacing> for LineSpacingDto {
    fn from(l: LineSpacing) -> Self {
        match l {
            LineSpacing::Single => LineSpacingDto::Single,
            LineSpacing::OneAndHalf => LineSpacingDto::OneAndHalf,
            LineSpacing::Double => LineSpacingDto::Double,
            LineSpacing::Multiple(v) => LineSpacingDto::Multiple(v),
            LineSpacing::Exact(v) => LineSpacingDto::Exact(v),
            LineSpacing::AtLeast(v) => LineSpacingDto::AtLeast(v),
        }
    }
}

impl From<LineSpacingDto> for LineSpacing {
    fn from(l: LineSpacingDto) -> Self {
        match l {
            LineSpacingDto::Single => LineSpacing::Single,
            LineSpacingDto::OneAndHalf => LineSpacing::OneAndHalf,
            LineSpacingDto::Double => LineSpacing::Double,
            LineSpacingDto::Multiple(v) => LineSpacing::Multiple(v),
            LineSpacingDto::Exact(v) => LineSpacing::Exact(v),
            LineSpacingDto::AtLeast(v) => LineSpacing::AtLeast(v),
        }
    }
}

/// DTO for [`NumberFormat`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum NumberFormatDto {
    Arabic,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
}

impl From<NumberFormat> for NumberFormatDto {
    fn from(n: NumberFormat) -> Self {
        match n {
            NumberFormat::Arabic => NumberFormatDto::Arabic,
            NumberFormat::LowerAlpha => NumberFormatDto::LowerAlpha,
            NumberFormat::UpperAlpha => NumberFormatDto::UpperAlpha,
            NumberFormat::LowerRoman => NumberFormatDto::LowerRoman,
            NumberFormat::UpperRoman => NumberFormatDto::UpperRoman,
        }
    }
}

impl From<NumberFormatDto> for NumberFormat {
    fn from(n: NumberFormatDto) -> Self {
        match n {
            NumberFormatDto::Arabic => NumberFormat::Arabic,
            NumberFormatDto::LowerAlpha => NumberFormat::LowerAlpha,
            NumberFormatDto::UpperAlpha => NumberFormat::UpperAlpha,
            NumberFormatDto::LowerRoman => NumberFormat::LowerRoman,
            NumberFormatDto::UpperRoman => NumberFormat::UpperRoman,
        }
    }
}

/// DTO for [`ListStyle`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum ListStyleDto {
    None,
    Bullet {
        character: char,
        indent: f64,
    },
    Numbered {
        format: NumberFormatDto,
        start: u32,
        indent: f64,
    },
}

impl From<ListStyle> for ListStyleDto {
    fn from(l: ListStyle) -> Self {
        match l {
            ListStyle::None => ListStyleDto::None,
            ListStyle::Bullet { character, indent } => ListStyleDto::Bullet { character, indent },
            ListStyle::Numbered {
                format,
                start,
                indent,
            } => ListStyleDto::Numbered {
                format: format.into(),
                start,
                indent,
            },
        }
    }
}

impl From<ListStyleDto> for ListStyle {
    fn from(l: ListStyleDto) -> Self {
        match l {
            ListStyleDto::None => ListStyle::None,
            ListStyleDto::Bullet { character, indent } => ListStyle::Bullet { character, indent },
            ListStyleDto::Numbered {
                format,
                start,
                indent,
            } => ListStyle::Numbered {
                format: format.into(),
                start,
                indent,
            },
        }
    }
}

/// DTO for [`ParagraphStyle`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParagraphStyleDto {
    /// Text alignment.
    pub alignment: TextAlignmentDto,
    /// Line spacing.
    pub line_spacing: LineSpacingDto,
    /// Space before, in points.
    pub space_before: f64,
    /// Space after, in points.
    pub space_after: f64,
    /// First line indent.
    pub first_line_indent: f64,
    /// Left indent.
    pub left_indent: f64,
    /// Right indent.
    pub right_indent: f64,
    /// List style.
    pub list_style: ListStyleDto,
    /// Keep with next flag.
    pub keep_with_next: bool,
    /// Keep together flag.
    pub keep_together: bool,
    /// Default character style for the paragraph.
    pub default_char_style: CharacterStyleDto,
}

impl From<ParagraphStyle> for ParagraphStyleDto {
    fn from(p: ParagraphStyle) -> Self {
        Self {
            alignment: p.alignment.into(),
            line_spacing: p.line_spacing.into(),
            space_before: p.space_before,
            space_after: p.space_after,
            first_line_indent: p.first_line_indent,
            left_indent: p.left_indent,
            right_indent: p.right_indent,
            list_style: p.list_style.into(),
            keep_with_next: p.keep_with_next,
            keep_together: p.keep_together,
            default_char_style: p.default_char_style.into(),
        }
    }
}

impl From<ParagraphStyleDto> for ParagraphStyle {
    fn from(p: ParagraphStyleDto) -> Self {
        ParagraphStyle {
            alignment: p.alignment.into(),
            line_spacing: p.line_spacing.into(),
            space_before: p.space_before,
            space_after: p.space_after,
            first_line_indent: p.first_line_indent,
            left_indent: p.left_indent,
            right_indent: p.right_indent,
            list_style: p.list_style.into(),
            keep_with_next: p.keep_with_next,
            keep_together: p.keep_together,
            default_char_style: p.default_char_style.into(),
        }
    }
}

/// DTO for [`TileMode`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum TileModeDto {
    Tile,
    TileHorizontal,
    TileVertical,
    Stretch,
    Center,
}

impl From<TileMode> for TileModeDto {
    fn from(t: TileMode) -> Self {
        match t {
            TileMode::Tile => TileModeDto::Tile,
            TileMode::TileHorizontal => TileModeDto::TileHorizontal,
            TileMode::TileVertical => TileModeDto::TileVertical,
            TileMode::Stretch => TileModeDto::Stretch,
            TileMode::Center => TileModeDto::Center,
        }
    }
}

impl From<TileModeDto> for TileMode {
    fn from(t: TileModeDto) -> Self {
        match t {
            TileModeDto::Tile => TileMode::Tile,
            TileModeDto::TileHorizontal => TileMode::TileHorizontal,
            TileModeDto::TileVertical => TileMode::TileVertical,
            TileModeDto::Stretch => TileMode::Stretch,
            TileModeDto::Center => TileMode::Center,
        }
    }
}

/// DTO for [`TexturePattern`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TexturePatternDto {
    /// Image source.
    pub source: String,
    /// Tile mode.
    pub tile: TileModeDto,
    /// Scale x.
    pub scale_x: f64,
    /// Scale y.
    pub scale_y: f64,
    /// Offset x.
    pub offset_x: f64,
    /// Offset y.
    pub offset_y: f64,
    /// Rotation in degrees.
    pub rotation: f64,
    /// Transparency.
    pub transparency: f64,
}

impl From<TexturePattern> for TexturePatternDto {
    fn from(t: TexturePattern) -> Self {
        Self {
            source: t.source,
            tile: t.tile.into(),
            scale_x: t.scale_x,
            scale_y: t.scale_y,
            offset_x: t.offset_x,
            offset_y: t.offset_y,
            rotation: t.rotation,
            transparency: t.transparency,
        }
    }
}

impl From<TexturePatternDto> for TexturePattern {
    fn from(t: TexturePatternDto) -> Self {
        TexturePattern {
            source: t.source,
            tile: t.tile.into(),
            scale_x: t.scale_x,
            scale_y: t.scale_y,
            offset_x: t.offset_x,
            offset_y: t.offset_y,
            rotation: t.rotation,
            transparency: t.transparency,
        }
    }
}

/// DTO for [`Fill`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum FillDto {
    None,
    Solid(ColorDto),
    LinearGradient(LinearGradientDto),
    RadialGradient(RadialGradientDto),
    Pattern(PatternDto),
    Texture(TexturePatternDto),
}

impl From<Fill> for FillDto {
    fn from(f: Fill) -> Self {
        match f {
            Fill::None => FillDto::None,
            Fill::Solid(c) => FillDto::Solid(c.into()),
            Fill::LinearGradient(g) => FillDto::LinearGradient(g.into()),
            Fill::RadialGradient(g) => FillDto::RadialGradient(g.into()),
            Fill::Pattern(p) => FillDto::Pattern(p.into()),
            Fill::Texture(t) => FillDto::Texture(t.into()),
        }
    }
}

impl From<FillDto> for Fill {
    fn from(f: FillDto) -> Self {
        match f {
            FillDto::None => Fill::None,
            FillDto::Solid(c) => Fill::Solid(c.into()),
            FillDto::LinearGradient(g) => Fill::LinearGradient(g.into()),
            FillDto::RadialGradient(g) => Fill::RadialGradient(g.into()),
            FillDto::Pattern(p) => Fill::Pattern(p.into()),
            FillDto::Texture(t) => Fill::Texture(t.into()),
        }
    }
}

/// DTO for [`LineCap`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum LineCapDto {
    Butt,
    Round,
    Square,
}

impl From<LineCap> for LineCapDto {
    fn from(c: LineCap) -> Self {
        match c {
            LineCap::Butt => LineCapDto::Butt,
            LineCap::Round => LineCapDto::Round,
            LineCap::Square => LineCapDto::Square,
        }
    }
}

impl From<LineCapDto> for LineCap {
    fn from(c: LineCapDto) -> Self {
        match c {
            LineCapDto::Butt => LineCap::Butt,
            LineCapDto::Round => LineCap::Round,
            LineCapDto::Square => LineCap::Square,
        }
    }
}

/// DTO for [`LineJoin`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum LineJoinDto {
    Miter,
    Round,
    Bevel,
}

impl From<LineJoin> for LineJoinDto {
    fn from(j: LineJoin) -> Self {
        match j {
            LineJoin::Miter => LineJoinDto::Miter,
            LineJoin::Round => LineJoinDto::Round,
            LineJoin::Bevel => LineJoinDto::Bevel,
        }
    }
}

impl From<LineJoinDto> for LineJoin {
    fn from(j: LineJoinDto) -> Self {
        match j {
            LineJoinDto::Miter => LineJoin::Miter,
            LineJoinDto::Round => LineJoin::Round,
            LineJoinDto::Bevel => LineJoin::Bevel,
        }
    }
}

/// DTO for [`Stroke`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrokeDto {
    /// Color.
    pub color: ColorDto,
    /// Width in points.
    pub width: f64,
    /// Line cap.
    pub cap: LineCapDto,
    /// Line join.
    pub join: LineJoinDto,
    /// Miter limit.
    pub miter_limit: f64,
    /// Dash pattern.
    pub dash_pattern: Option<Vec<f64>>,
    /// Dash offset.
    pub dash_offset: f64,
}

impl From<Stroke> for StrokeDto {
    fn from(s: Stroke) -> Self {
        Self {
            color: s.color.into(),
            width: s.width,
            cap: s.cap.into(),
            join: s.join.into(),
            miter_limit: s.miter_limit,
            dash_pattern: s.dash_pattern,
            dash_offset: s.dash_offset,
        }
    }
}

impl From<StrokeDto> for Stroke {
    fn from(s: StrokeDto) -> Self {
        Stroke {
            color: s.color.into(),
            width: s.width,
            cap: s.cap.into(),
            join: s.join.into(),
            miter_limit: s.miter_limit,
            dash_pattern: s.dash_pattern,
            dash_offset: s.dash_offset,
        }
    }
}

/// DTO for [`DropShadow`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DropShadowDto {
    /// Color.
    pub color: ColorDto,
    /// Horizontal offset.
    pub offset_x: f64,
    /// Vertical offset.
    pub offset_y: f64,
    /// Blur radius.
    pub blur_radius: f64,
    /// Spread.
    pub spread: f64,
}

impl From<DropShadow> for DropShadowDto {
    fn from(s: DropShadow) -> Self {
        Self {
            color: s.color.into(),
            offset_x: s.offset_x,
            offset_y: s.offset_y,
            blur_radius: s.blur_radius,
            spread: s.spread,
        }
    }
}

impl From<DropShadowDto> for DropShadow {
    fn from(s: DropShadowDto) -> Self {
        DropShadow {
            color: s.color.into(),
            offset_x: s.offset_x,
            offset_y: s.offset_y,
            blur_radius: s.blur_radius,
            spread: s.spread,
        }
    }
}

/// DTO for [`InnerShadow`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerShadowDto {
    /// Color.
    pub color: ColorDto,
    /// Horizontal offset.
    pub offset_x: f64,
    /// Vertical offset.
    pub offset_y: f64,
    /// Blur radius.
    pub blur_radius: f64,
    /// Choke.
    pub choke: f64,
}

impl From<InnerShadow> for InnerShadowDto {
    fn from(s: InnerShadow) -> Self {
        Self {
            color: s.color.into(),
            offset_x: s.offset_x,
            offset_y: s.offset_y,
            blur_radius: s.blur_radius,
            choke: s.choke,
        }
    }
}

impl From<InnerShadowDto> for InnerShadow {
    fn from(s: InnerShadowDto) -> Self {
        InnerShadow {
            color: s.color.into(),
            offset_x: s.offset_x,
            offset_y: s.offset_y,
            blur_radius: s.blur_radius,
            choke: s.choke,
        }
    }
}

/// DTO for [`Glow`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlowDto {
    /// Color.
    pub color: ColorDto,
    /// Radius.
    pub radius: f64,
    /// Spread.
    pub spread: f64,
}

impl From<Glow> for GlowDto {
    fn from(g: Glow) -> Self {
        Self {
            color: g.color.into(),
            radius: g.radius,
            spread: g.spread,
        }
    }
}

impl From<GlowDto> for Glow {
    fn from(g: GlowDto) -> Self {
        Glow {
            color: g.color.into(),
            radius: g.radius,
            spread: g.spread,
        }
    }
}

/// DTO for [`InnerGlowSource`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum InnerGlowSourceDto {
    Edge,
    Center,
}

impl From<InnerGlowSource> for InnerGlowSourceDto {
    fn from(s: InnerGlowSource) -> Self {
        match s {
            InnerGlowSource::Edge => InnerGlowSourceDto::Edge,
            InnerGlowSource::Center => InnerGlowSourceDto::Center,
        }
    }
}

impl From<InnerGlowSourceDto> for InnerGlowSource {
    fn from(s: InnerGlowSourceDto) -> Self {
        match s {
            InnerGlowSourceDto::Edge => InnerGlowSource::Edge,
            InnerGlowSourceDto::Center => InnerGlowSource::Center,
        }
    }
}

/// DTO for [`InnerGlow`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InnerGlowDto {
    /// Color.
    pub color: ColorDto,
    /// Radius.
    pub radius: f64,
    /// Source.
    pub source: InnerGlowSourceDto,
    /// Choke.
    pub choke: f64,
}

impl From<InnerGlow> for InnerGlowDto {
    fn from(g: InnerGlow) -> Self {
        Self {
            color: g.color.into(),
            radius: g.radius,
            source: g.source.into(),
            choke: g.choke,
        }
    }
}

impl From<InnerGlowDto> for InnerGlow {
    fn from(g: InnerGlowDto) -> Self {
        InnerGlow {
            color: g.color.into(),
            radius: g.radius,
            source: g.source.into(),
            choke: g.choke,
        }
    }
}

/// DTO for [`Reflection`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionDto {
    /// Opacity.
    pub opacity: f64,
    /// Fade length.
    pub fade_length: f64,
    /// Gap.
    pub gap: f64,
    /// Blur.
    pub blur: f64,
    /// Direction in degrees.
    pub direction: f64,
    /// Size.
    pub size: f64,
}

impl From<Reflection> for ReflectionDto {
    fn from(r: Reflection) -> Self {
        Self {
            opacity: r.opacity,
            fade_length: r.fade_length,
            gap: r.gap,
            blur: r.blur,
            direction: r.direction,
            size: r.size,
        }
    }
}

impl From<ReflectionDto> for Reflection {
    fn from(r: ReflectionDto) -> Self {
        Reflection {
            opacity: r.opacity,
            fade_length: r.fade_length,
            gap: r.gap,
            blur: r.blur,
            direction: r.direction,
            size: r.size,
        }
    }
}

/// DTO for [`BevelStyle`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum BevelStyleDto {
    Outer,
    Inner,
    Emboss,
    PillowEmboss,
}

impl From<BevelStyle> for BevelStyleDto {
    fn from(b: BevelStyle) -> Self {
        match b {
            BevelStyle::Outer => BevelStyleDto::Outer,
            BevelStyle::Inner => BevelStyleDto::Inner,
            BevelStyle::Emboss => BevelStyleDto::Emboss,
            BevelStyle::PillowEmboss => BevelStyleDto::PillowEmboss,
        }
    }
}

impl From<BevelStyleDto> for BevelStyle {
    fn from(b: BevelStyleDto) -> Self {
        match b {
            BevelStyleDto::Outer => BevelStyle::Outer,
            BevelStyleDto::Inner => BevelStyle::Inner,
            BevelStyleDto::Emboss => BevelStyle::Emboss,
            BevelStyleDto::PillowEmboss => BevelStyle::PillowEmboss,
        }
    }
}

/// DTO for [`BevelTechnique`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum BevelTechniqueDto {
    Smooth,
    ChiselHard,
    ChiselSoft,
}

impl From<BevelTechnique> for BevelTechniqueDto {
    fn from(b: BevelTechnique) -> Self {
        match b {
            BevelTechnique::Smooth => BevelTechniqueDto::Smooth,
            BevelTechnique::ChiselHard => BevelTechniqueDto::ChiselHard,
            BevelTechnique::ChiselSoft => BevelTechniqueDto::ChiselSoft,
        }
    }
}

impl From<BevelTechniqueDto> for BevelTechnique {
    fn from(b: BevelTechniqueDto) -> Self {
        match b {
            BevelTechniqueDto::Smooth => BevelTechnique::Smooth,
            BevelTechniqueDto::ChiselHard => BevelTechnique::ChiselHard,
            BevelTechniqueDto::ChiselSoft => BevelTechnique::ChiselSoft,
        }
    }
}

/// DTO for [`Bevel`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BevelDto {
    /// Style.
    pub style: BevelStyleDto,
    /// Technique.
    pub technique: BevelTechniqueDto,
    /// Depth.
    pub depth: f64,
    /// Size.
    pub size: f64,
    /// Soften.
    pub soften: f64,
    /// Light angle.
    pub light_angle: f64,
    /// Light altitude.
    pub light_altitude: f64,
    /// Highlight color.
    pub highlight_color: ColorDto,
    /// Highlight opacity.
    pub highlight_opacity: f64,
    /// Shadow color.
    pub shadow_color: ColorDto,
    /// Shadow opacity.
    pub shadow_opacity: f64,
}

impl From<Bevel> for BevelDto {
    fn from(b: Bevel) -> Self {
        Self {
            style: b.style.into(),
            technique: b.technique.into(),
            depth: b.depth,
            size: b.size,
            soften: b.soften,
            light_angle: b.light_angle,
            light_altitude: b.light_altitude,
            highlight_color: b.highlight_color.into(),
            highlight_opacity: b.highlight_opacity,
            shadow_color: b.shadow_color.into(),
            shadow_opacity: b.shadow_opacity,
        }
    }
}

impl From<BevelDto> for Bevel {
    fn from(b: BevelDto) -> Self {
        Bevel {
            style: b.style.into(),
            technique: b.technique.into(),
            depth: b.depth,
            size: b.size,
            soften: b.soften,
            light_angle: b.light_angle,
            light_altitude: b.light_altitude,
            highlight_color: b.highlight_color.into(),
            highlight_opacity: b.highlight_opacity,
            shadow_color: b.shadow_color.into(),
            shadow_opacity: b.shadow_opacity,
        }
    }
}

/// DTO for [`SoftEdges`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SoftEdgesDto {
    /// Feather radius.
    pub radius: f64,
}

impl From<SoftEdges> for SoftEdgesDto {
    fn from(s: SoftEdges) -> Self {
        Self { radius: s.radius }
    }
}

impl From<SoftEdgesDto> for SoftEdges {
    fn from(s: SoftEdgesDto) -> Self {
        SoftEdges { radius: s.radius }
    }
}

/// DTO for [`Effects`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EffectsDto {
    /// Drop shadow.
    pub drop_shadow: Option<DropShadowDto>,
    /// Inner shadow.
    pub inner_shadow: Option<InnerShadowDto>,
    /// Outer glow.
    pub outer_glow: Option<GlowDto>,
    /// Inner glow.
    pub inner_glow: Option<InnerGlowDto>,
    /// Bevel.
    pub bevel: Option<BevelDto>,
    /// Reflection.
    pub reflection: Option<ReflectionDto>,
    /// Soft edges.
    pub soft_edges: Option<SoftEdgesDto>,
}

impl From<Effects> for EffectsDto {
    fn from(e: Effects) -> Self {
        Self {
            drop_shadow: e.drop_shadow.map(Into::into),
            inner_shadow: e.inner_shadow.map(Into::into),
            outer_glow: e.outer_glow.map(Into::into),
            inner_glow: e.inner_glow.map(Into::into),
            bevel: e.bevel.map(Into::into),
            reflection: e.reflection.map(Into::into),
            soft_edges: e.soft_edges.map(Into::into),
        }
    }
}

impl From<EffectsDto> for Effects {
    fn from(e: EffectsDto) -> Self {
        Effects {
            drop_shadow: e.drop_shadow.map(Into::into),
            inner_shadow: e.inner_shadow.map(Into::into),
            outer_glow: e.outer_glow.map(Into::into),
            inner_glow: e.inner_glow.map(Into::into),
            bevel: e.bevel.map(Into::into),
            reflection: e.reflection.map(Into::into),
            soft_edges: e.soft_edges.map(Into::into),
        }
    }
}
