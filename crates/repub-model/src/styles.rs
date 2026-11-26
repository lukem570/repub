//! Style definitions for text, paragraphs, and shapes.
//!
//! This module consolidates fonts and character/paragraph styling, shape fill
//! and stroke, visual effects, text wrap, texture fills, and the named style
//! sheet into one place.

use std::collections::HashMap;

use repub_color::Color;

use crate::id::StyleId;

// Bitmap fill patterns and gradients come from the foundation color layer.
pub use repub_color::{GradientStop, LinearGradient, Pattern, PatternType, RadialGradient};

/// Font weight specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontWeight {
    /// Thin (100)
    Thin,
    /// Extra Light (200)
    ExtraLight,
    /// Light (300)
    Light,
    /// Normal/Regular (400)
    #[default]
    Normal,
    /// Medium (500)
    Medium,
    /// Semi Bold (600)
    SemiBold,
    /// Bold (700)
    Bold,
    /// Extra Bold (800)
    ExtraBold,
    /// Black (900)
    Black,
}

impl FontWeight {
    /// Get the numeric weight value.
    pub fn weight(&self) -> u16 {
        match self {
            FontWeight::Thin => 100,
            FontWeight::ExtraLight => 200,
            FontWeight::Light => 300,
            FontWeight::Normal => 400,
            FontWeight::Medium => 500,
            FontWeight::SemiBold => 600,
            FontWeight::Bold => 700,
            FontWeight::ExtraBold => 800,
            FontWeight::Black => 900,
        }
    }
}

/// Font style (normal or italic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontStyle {
    /// Normal upright text
    #[default]
    Normal,
    /// Italic text
    Italic,
    /// Oblique text (slanted, but not true italic)
    Oblique,
}

/// Font stretch/width specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FontStretch {
    /// Ultra-condensed
    UltraCondensed,
    /// Extra-condensed
    ExtraCondensed,
    /// Condensed
    Condensed,
    /// Semi-condensed
    SemiCondensed,
    /// Normal width
    #[default]
    Normal,
    /// Semi-expanded
    SemiExpanded,
    /// Expanded
    Expanded,
    /// Extra-expanded
    ExtraExpanded,
    /// Ultra-expanded
    UltraExpanded,
}

/// Complete font specification.
#[derive(Debug, Clone, PartialEq)]
pub struct FontSpec {
    /// Font family name (e.g., "Arial", "Times New Roman")
    pub family: String,
    /// Font weight
    pub weight: FontWeight,
    /// Font style
    pub style: FontStyle,
    /// Font stretch
    pub stretch: FontStretch,
}

impl Default for FontSpec {
    fn default() -> Self {
        Self {
            family: "Arial".to_string(),
            weight: FontWeight::Normal,
            style: FontStyle::Normal,
            stretch: FontStretch::Normal,
        }
    }
}

impl FontSpec {
    /// Create a new font specification with the given family.
    pub fn new(family: impl Into<String>) -> Self {
        Self {
            family: family.into(),
            ..Default::default()
        }
    }

    /// Set the font weight.
    pub fn with_weight(mut self, weight: FontWeight) -> Self {
        self.weight = weight;
        self
    }

    /// Set the font style.
    pub fn with_style(mut self, style: FontStyle) -> Self {
        self.style = style;
        self
    }

    /// Make the font bold.
    pub fn bold(self) -> Self {
        self.with_weight(FontWeight::Bold)
    }

    /// Make the font italic.
    pub fn italic(self) -> Self {
        self.with_style(FontStyle::Italic)
    }
}

/// Underline style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnderlineStyle {
    /// No underline
    #[default]
    None,
    /// Single line
    Single,
    /// Double line
    Double,
    /// Dotted line
    Dotted,
    /// Dashed line
    Dashed,
    /// Wavy line
    Wavy,
}

/// Strikethrough style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StrikethroughStyle {
    /// No strikethrough
    #[default]
    None,
    /// Single line through middle
    Single,
    /// Double line
    Double,
}

/// Text baseline position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BaselineShift {
    /// Normal baseline
    #[default]
    Normal,
    /// Superscript
    Superscript,
    /// Subscript
    Subscript,
}

/// Character-level text styling.
#[derive(Debug, Clone, PartialEq)]
pub struct CharacterStyle {
    /// Font specification
    pub font: FontSpec,
    /// Font size in points
    pub font_size: f64,
    /// Text color
    pub color: Color,
    /// Background/highlight color
    pub background_color: Option<Color>,
    /// Underline style
    pub underline: UnderlineStyle,
    /// Underline color (defaults to text color if None)
    pub underline_color: Option<Color>,
    /// Strikethrough style
    pub strikethrough: StrikethroughStyle,
    /// Baseline shift (superscript/subscript)
    pub baseline_shift: BaselineShift,
    /// Letter spacing adjustment in points (positive = wider)
    pub letter_spacing: f64,
    /// Whether text is all caps
    pub all_caps: bool,
    /// Whether text is small caps
    pub small_caps: bool,
}

impl Default for CharacterStyle {
    fn default() -> Self {
        Self {
            font: FontSpec::default(),
            font_size: 12.0,
            color: Color::BLACK,
            background_color: None,
            underline: UnderlineStyle::None,
            underline_color: None,
            strikethrough: StrikethroughStyle::None,
            baseline_shift: BaselineShift::Normal,
            letter_spacing: 0.0,
            all_caps: false,
            small_caps: false,
        }
    }
}

impl CharacterStyle {
    /// Create a new character style with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the font family.
    pub fn with_font(mut self, family: impl Into<String>) -> Self {
        self.font.family = family.into();
        self
    }

    /// Set the font size.
    pub fn with_size(mut self, size: f64) -> Self {
        self.font_size = size;
        self
    }

    /// Set the text color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Make the text bold.
    pub fn bold(mut self) -> Self {
        self.font.weight = FontWeight::Bold;
        self
    }

    /// Make the text italic.
    pub fn italic(mut self) -> Self {
        self.font.style = FontStyle::Italic;
        self
    }

    /// Add underline.
    pub fn underline(mut self) -> Self {
        self.underline = UnderlineStyle::Single;
        self
    }
}

/// Text alignment within a paragraph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextAlignment {
    /// Align to the left (or start in RTL)
    #[default]
    Left,
    /// Center the text
    Center,
    /// Align to the right (or end in RTL)
    Right,
    /// Justify (align both left and right)
    Justify,
}

/// Line spacing specification.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum LineSpacing {
    /// Single line spacing (1.0)
    #[default]
    Single,
    /// 1.5 line spacing
    OneAndHalf,
    /// Double line spacing (2.0)
    Double,
    /// Custom multiplier of font size
    Multiple(f64),
    /// Exact spacing in points
    Exact(f64),
    /// At least this many points
    AtLeast(f64),
}

impl LineSpacing {
    /// Calculate the actual line spacing for a given font size.
    pub fn calculate(&self, font_size: f64) -> f64 {
        match self {
            LineSpacing::Single => font_size * 1.2, // Typical single spacing
            LineSpacing::OneAndHalf => font_size * 1.5,
            LineSpacing::Double => font_size * 2.0,
            LineSpacing::Multiple(factor) => font_size * factor,
            LineSpacing::Exact(pts) => *pts,
            LineSpacing::AtLeast(min) => font_size.max(*min),
        }
    }
}

/// Bullet or numbering style.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ListStyle {
    /// No list style
    #[default]
    None,
    /// Bullet points
    Bullet {
        /// The bullet character
        character: char,
        /// Indent from margin in points
        indent: f64,
    },
    /// Numbered list
    Numbered {
        /// Number format (e.g., "1.", "a)", "i.")
        format: NumberFormat,
        /// Starting number
        start: u32,
        /// Indent from margin in points
        indent: f64,
    },
}

/// Number format for numbered lists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NumberFormat {
    /// Arabic numerals (1, 2, 3)
    #[default]
    Arabic,
    /// Lowercase letters (a, b, c)
    LowerAlpha,
    /// Uppercase letters (A, B, C)
    UpperAlpha,
    /// Lowercase roman numerals (i, ii, iii)
    LowerRoman,
    /// Uppercase roman numerals (I, II, III)
    UpperRoman,
}

/// Paragraph-level styling.
#[derive(Debug, Clone, PartialEq)]
pub struct ParagraphStyle {
    /// Text alignment
    pub alignment: TextAlignment,
    /// Line spacing
    pub line_spacing: LineSpacing,
    /// Space before paragraph in points
    pub space_before: f64,
    /// Space after paragraph in points
    pub space_after: f64,
    /// First line indent in points (positive = indent, negative = hanging)
    pub first_line_indent: f64,
    /// Left indent in points
    pub left_indent: f64,
    /// Right indent in points
    pub right_indent: f64,
    /// List/bullet style
    pub list_style: ListStyle,
    /// Keep with next paragraph (don't break between this and next)
    pub keep_with_next: bool,
    /// Keep lines together (don't break within paragraph)
    pub keep_together: bool,
    /// Default character style for the paragraph
    pub default_char_style: CharacterStyle,
}

impl Default for ParagraphStyle {
    fn default() -> Self {
        Self {
            alignment: TextAlignment::Left,
            line_spacing: LineSpacing::Single,
            space_before: 0.0,
            space_after: 0.0,
            first_line_indent: 0.0,
            left_indent: 0.0,
            right_indent: 0.0,
            list_style: ListStyle::None,
            keep_with_next: false,
            keep_together: false,
            default_char_style: CharacterStyle::default(),
        }
    }
}

impl ParagraphStyle {
    /// Create a new paragraph style.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the text alignment.
    pub fn with_alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Set line spacing.
    pub fn with_line_spacing(mut self, spacing: LineSpacing) -> Self {
        self.line_spacing = spacing;
        self
    }

    /// Set space before and after.
    pub fn with_spacing(mut self, before: f64, after: f64) -> Self {
        self.space_before = before;
        self.space_after = after;
        self
    }

    /// Set first line indent.
    pub fn with_first_line_indent(mut self, indent: f64) -> Self {
        self.first_line_indent = indent;
        self
    }
}

/// Fill style for shapes.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Fill {
    /// No fill (transparent)
    #[default]
    None,
    /// Solid color fill
    Solid(Color),
    /// Linear gradient
    LinearGradient(LinearGradient),
    /// Radial gradient
    RadialGradient(RadialGradient),
    /// Pattern fill
    Pattern(Pattern),
    /// Texture/image fill
    Texture(TexturePattern),
}

impl Fill {
    /// Create a solid color fill.
    pub fn solid(color: Color) -> Self {
        Fill::Solid(color)
    }

    /// Create a pattern fill.
    pub fn pattern(pattern: Pattern) -> Self {
        Fill::Pattern(pattern)
    }

    /// Create a texture fill.
    pub fn texture(source: impl Into<String>) -> Self {
        Fill::Texture(TexturePattern::new(source))
    }

    /// Create a linear gradient fill.
    pub fn linear_gradient(gradient: LinearGradient) -> Self {
        Fill::LinearGradient(gradient)
    }

    /// Create a radial gradient fill.
    pub fn radial_gradient(gradient: RadialGradient) -> Self {
        Fill::RadialGradient(gradient)
    }

    /// Get the primary color for this fill (useful for fallback rendering).
    /// Returns the solid color, the first gradient stop, pattern foreground, or white.
    pub fn primary_color(&self) -> Color {
        match self {
            Fill::None => Color::WHITE,
            Fill::Solid(color) => *color,
            Fill::LinearGradient(g) => g.stops.first().map(|s| s.color).unwrap_or(Color::WHITE),
            Fill::RadialGradient(g) => g.stops.first().map(|s| s.color).unwrap_or(Color::WHITE),
            Fill::Pattern(p) => p.foreground,
            Fill::Texture(_) => Color::WHITE,
        }
    }

    /// Check if this fill is transparent (no fill).
    pub fn is_none(&self) -> bool {
        matches!(self, Fill::None)
    }

    /// Check if this fill is a solid color.
    pub fn is_solid(&self) -> bool {
        matches!(self, Fill::Solid(_))
    }

    /// Check if this fill is a gradient (linear or radial).
    pub fn is_gradient(&self) -> bool {
        matches!(self, Fill::LinearGradient(_) | Fill::RadialGradient(_))
    }
}

/// Line cap style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineCap {
    /// Flat cap at the endpoint
    #[default]
    Butt,
    /// Rounded cap
    Round,
    /// Square cap extending past endpoint
    Square,
}

/// Line join style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineJoin {
    /// Sharp corner
    #[default]
    Miter,
    /// Rounded corner
    Round,
    /// Beveled corner
    Bevel,
}

/// Stroke/outline style.
#[derive(Debug, Clone, PartialEq)]
pub struct Stroke {
    /// Stroke color
    pub color: Color,
    /// Stroke width in points
    pub width: f64,
    /// Line cap style
    pub cap: LineCap,
    /// Line join style
    pub join: LineJoin,
    /// Miter limit (for miter joins)
    pub miter_limit: f64,
    /// Dash pattern (lengths of dashes and gaps in points)
    pub dash_pattern: Option<Vec<f64>>,
    /// Dash offset
    pub dash_offset: f64,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            color: Color::BLACK,
            width: 1.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            miter_limit: 4.0,
            dash_pattern: None,
            dash_offset: 0.0,
        }
    }
}

impl Stroke {
    /// Create a new stroke with the given color and width.
    pub fn new(color: Color, width: f64) -> Self {
        Self {
            color,
            width,
            ..Default::default()
        }
    }

    /// Create a solid stroke.
    pub fn solid(color: Color, width: f64) -> Self {
        Self::new(color, width)
    }

    /// Create a dashed stroke.
    pub fn dashed(color: Color, width: f64, dash: f64, gap: f64) -> Self {
        Self {
            color,
            width,
            dash_pattern: Some(vec![dash, gap]),
            ..Default::default()
        }
    }

    /// Create a dotted stroke.
    pub fn dotted(color: Color, width: f64) -> Self {
        Self::dashed(color, width, width, width * 2.0)
    }
}

/// Drop shadow effect.
#[derive(Debug, Clone, PartialEq)]
pub struct DropShadow {
    /// Shadow color
    pub color: Color,
    /// Horizontal offset in points
    pub offset_x: f64,
    /// Vertical offset in points
    pub offset_y: f64,
    /// Blur radius in points
    pub blur_radius: f64,
    /// Spread (expand/contract) in points
    pub spread: f64,
}

impl Default for DropShadow {
    fn default() -> Self {
        Self {
            color: Color::new(0.0, 0.0, 0.0, 0.5),
            offset_x: 2.0,
            offset_y: 2.0,
            blur_radius: 4.0,
            spread: 0.0,
        }
    }
}

impl DropShadow {
    /// Create a new drop shadow.
    pub fn new(offset_x: f64, offset_y: f64, blur_radius: f64) -> Self {
        Self {
            offset_x,
            offset_y,
            blur_radius,
            ..Default::default()
        }
    }

    /// Create a shadow with specific color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// Set the spread.
    pub fn with_spread(mut self, spread: f64) -> Self {
        self.spread = spread;
        self
    }

    /// Create a soft shadow preset.
    pub fn soft() -> Self {
        Self {
            color: Color::new(0.0, 0.0, 0.0, 0.3),
            offset_x: 4.0,
            offset_y: 4.0,
            blur_radius: 8.0,
            spread: 0.0,
        }
    }

    /// Create a hard shadow preset.
    pub fn hard() -> Self {
        Self {
            color: Color::new(0.0, 0.0, 0.0, 0.6),
            offset_x: 3.0,
            offset_y: 3.0,
            blur_radius: 0.0,
            spread: 0.0,
        }
    }
}

/// Glow effect (outer glow around object).
#[derive(Debug, Clone, PartialEq)]
pub struct Glow {
    /// Glow color
    pub color: Color,
    /// Blur radius in points
    pub radius: f64,
    /// Spread (how far the glow extends)
    pub spread: f64,
}

impl Default for Glow {
    fn default() -> Self {
        Self {
            color: Color::new(1.0, 1.0, 0.0, 0.5),
            radius: 5.0,
            spread: 0.0,
        }
    }
}

impl Glow {
    /// Create a new glow effect.
    pub fn new(color: Color, radius: f64) -> Self {
        Self {
            color,
            radius,
            spread: 0.0,
        }
    }

    /// Create a soft glow preset.
    pub fn soft(color: Color) -> Self {
        Self {
            color: color.with_alpha(0.5),
            radius: 10.0,
            spread: 0.0,
        }
    }

    /// Set the spread.
    pub fn with_spread(mut self, spread: f64) -> Self {
        self.spread = spread;
        self
    }
}

/// Inner shadow effect.
#[derive(Debug, Clone, PartialEq)]
pub struct InnerShadow {
    /// Shadow color
    pub color: Color,
    /// Horizontal offset in points
    pub offset_x: f64,
    /// Vertical offset in points
    pub offset_y: f64,
    /// Blur radius in points
    pub blur_radius: f64,
    /// Choke (how much the shadow contracts)
    pub choke: f64,
}

impl Default for InnerShadow {
    fn default() -> Self {
        Self {
            color: Color::new(0.0, 0.0, 0.0, 0.5),
            offset_x: 2.0,
            offset_y: 2.0,
            blur_radius: 4.0,
            choke: 0.0,
        }
    }
}

impl InnerShadow {
    /// Create a new inner shadow.
    pub fn new(offset_x: f64, offset_y: f64, blur_radius: f64) -> Self {
        Self {
            offset_x,
            offset_y,
            blur_radius,
            ..Default::default()
        }
    }

    /// Set the color.
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

/// Inner glow effect.
#[derive(Debug, Clone, PartialEq)]
pub struct InnerGlow {
    /// Glow color
    pub color: Color,
    /// Blur radius in points
    pub radius: f64,
    /// Source (edge or center)
    pub source: InnerGlowSource,
    /// Choke
    pub choke: f64,
}

/// Source of inner glow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InnerGlowSource {
    /// Glow from the edge inward
    #[default]
    Edge,
    /// Glow from the center outward
    Center,
}

impl Default for InnerGlow {
    fn default() -> Self {
        Self {
            color: Color::new(1.0, 1.0, 1.0, 0.75),
            radius: 5.0,
            source: InnerGlowSource::Edge,
            choke: 0.0,
        }
    }
}

impl InnerGlow {
    /// Create a new inner glow.
    pub fn new(color: Color, radius: f64) -> Self {
        Self {
            color,
            radius,
            source: InnerGlowSource::Edge,
            choke: 0.0,
        }
    }

    /// Set the source to center.
    pub fn from_center(mut self) -> Self {
        self.source = InnerGlowSource::Center;
        self
    }
}

/// Reflection effect (mirror image below object).
#[derive(Debug, Clone, PartialEq)]
pub struct Reflection {
    /// Reflection opacity at the top (0-1)
    pub opacity: f64,
    /// How far the reflection fades (0-1)
    pub fade_length: f64,
    /// Gap between object and reflection in points
    pub gap: f64,
    /// Blur amount for the reflection
    pub blur: f64,
    /// Direction of reflection in degrees (0 = below)
    pub direction: f64,
    /// Size of reflection relative to object (0-1)
    pub size: f64,
}

impl Default for Reflection {
    fn default() -> Self {
        Self {
            opacity: 0.5,
            fade_length: 0.5,
            gap: 0.0,
            blur: 0.0,
            direction: 0.0,
            size: 1.0,
        }
    }
}

impl Reflection {
    /// Create a new reflection effect.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a tight reflection (small gap, quick fade).
    pub fn tight() -> Self {
        Self {
            opacity: 0.4,
            fade_length: 0.3,
            gap: 0.0,
            blur: 0.0,
            direction: 0.0,
            size: 0.5,
        }
    }

    /// Create a full reflection.
    pub fn full() -> Self {
        Self {
            opacity: 0.5,
            fade_length: 0.8,
            gap: 0.0,
            blur: 0.0,
            direction: 0.0,
            size: 1.0,
        }
    }

    /// Create a half reflection.
    pub fn half() -> Self {
        Self {
            opacity: 0.5,
            fade_length: 0.5,
            gap: 4.0,
            blur: 0.0,
            direction: 0.0,
            size: 0.5,
        }
    }

    /// Set the opacity.
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// Set the gap.
    pub fn with_gap(mut self, gap: f64) -> Self {
        self.gap = gap;
        self
    }

    /// Set the blur.
    pub fn with_blur(mut self, blur: f64) -> Self {
        self.blur = blur;
        self
    }

    /// Set the size.
    pub fn with_size(mut self, size: f64) -> Self {
        self.size = size.clamp(0.0, 1.0);
        self
    }
}

/// Bevel effect (3D edge effect).
#[derive(Debug, Clone, PartialEq)]
pub struct Bevel {
    /// Bevel style
    pub style: BevelStyle,
    /// Bevel technique
    pub technique: BevelTechnique,
    /// Depth in points
    pub depth: f64,
    /// Size in points
    pub size: f64,
    /// Soften amount
    pub soften: f64,
    /// Light angle in degrees
    pub light_angle: f64,
    /// Light altitude in degrees
    pub light_altitude: f64,
    /// Highlight color
    pub highlight_color: Color,
    /// Highlight opacity
    pub highlight_opacity: f64,
    /// Shadow color
    pub shadow_color: Color,
    /// Shadow opacity
    pub shadow_opacity: f64,
}

/// Bevel style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BevelStyle {
    /// Outer bevel
    #[default]
    Outer,
    /// Inner bevel
    Inner,
    /// Emboss
    Emboss,
    /// Pillow emboss
    PillowEmboss,
}

/// Bevel technique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BevelTechnique {
    /// Smooth technique
    #[default]
    Smooth,
    /// Chisel hard
    ChiselHard,
    /// Chisel soft
    ChiselSoft,
}

impl Default for Bevel {
    fn default() -> Self {
        Self {
            style: BevelStyle::Outer,
            technique: BevelTechnique::Smooth,
            depth: 100.0,
            size: 5.0,
            soften: 0.0,
            light_angle: 120.0,
            light_altitude: 30.0,
            highlight_color: Color::WHITE,
            highlight_opacity: 0.75,
            shadow_color: Color::BLACK,
            shadow_opacity: 0.75,
        }
    }
}

impl Bevel {
    /// Create a new bevel effect.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an inner bevel.
    pub fn inner() -> Self {
        Self {
            style: BevelStyle::Inner,
            ..Default::default()
        }
    }

    /// Create an emboss effect.
    pub fn emboss() -> Self {
        Self {
            style: BevelStyle::Emboss,
            ..Default::default()
        }
    }

    /// Create a pillow emboss effect.
    pub fn pillow_emboss() -> Self {
        Self {
            style: BevelStyle::PillowEmboss,
            ..Default::default()
        }
    }

    /// Set the size.
    pub fn with_size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    /// Set the depth.
    pub fn with_depth(mut self, depth: f64) -> Self {
        self.depth = depth;
        self
    }

    /// Set the light angle.
    pub fn with_light_angle(mut self, angle: f64) -> Self {
        self.light_angle = angle;
        self
    }

    /// Set the technique.
    pub fn with_technique(mut self, technique: BevelTechnique) -> Self {
        self.technique = technique;
        self
    }

    /// Use chisel hard technique.
    pub fn chisel_hard(mut self) -> Self {
        self.technique = BevelTechnique::ChiselHard;
        self
    }

    /// Use chisel soft technique.
    pub fn chisel_soft(mut self) -> Self {
        self.technique = BevelTechnique::ChiselSoft;
        self
    }
}

/// Soft edges effect (feathered edges).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SoftEdges {
    /// Feather radius in points
    pub radius: f64,
}

impl Default for SoftEdges {
    fn default() -> Self {
        Self { radius: 5.0 }
    }
}

impl SoftEdges {
    /// Create a soft edges effect.
    pub fn new(radius: f64) -> Self {
        Self { radius }
    }
}

/// Combined effects that can be applied to an object.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Effects {
    /// Drop shadow
    pub drop_shadow: Option<DropShadow>,
    /// Inner shadow
    pub inner_shadow: Option<InnerShadow>,
    /// Outer glow
    pub outer_glow: Option<Glow>,
    /// Inner glow
    pub inner_glow: Option<InnerGlow>,
    /// Bevel and emboss
    pub bevel: Option<Bevel>,
    /// Reflection
    pub reflection: Option<Reflection>,
    /// Soft edges
    pub soft_edges: Option<SoftEdges>,
}

impl Effects {
    /// Create empty effects.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a drop shadow.
    pub fn with_drop_shadow(mut self, shadow: DropShadow) -> Self {
        self.drop_shadow = Some(shadow);
        self
    }

    /// Add an inner shadow.
    pub fn with_inner_shadow(mut self, shadow: InnerShadow) -> Self {
        self.inner_shadow = Some(shadow);
        self
    }

    /// Add an outer glow.
    pub fn with_outer_glow(mut self, glow: Glow) -> Self {
        self.outer_glow = Some(glow);
        self
    }

    /// Add an inner glow.
    pub fn with_inner_glow(mut self, glow: InnerGlow) -> Self {
        self.inner_glow = Some(glow);
        self
    }

    /// Add a bevel.
    pub fn with_bevel(mut self, bevel: Bevel) -> Self {
        self.bevel = Some(bevel);
        self
    }

    /// Add a reflection.
    pub fn with_reflection(mut self, reflection: Reflection) -> Self {
        self.reflection = Some(reflection);
        self
    }

    /// Add soft edges.
    pub fn with_soft_edges(mut self, radius: f64) -> Self {
        self.soft_edges = Some(SoftEdges::new(radius));
        self
    }

    /// Check if any effects are applied.
    pub fn has_effects(&self) -> bool {
        self.drop_shadow.is_some()
            || self.inner_shadow.is_some()
            || self.outer_glow.is_some()
            || self.inner_glow.is_some()
            || self.bevel.is_some()
            || self.reflection.is_some()
            || self.soft_edges.is_some()
    }

    /// Clear all effects.
    pub fn clear(&mut self) {
        self.drop_shadow = None;
        self.inner_shadow = None;
        self.outer_glow = None;
        self.inner_glow = None;
        self.bevel = None;
        self.reflection = None;
        self.soft_edges = None;
    }
}

/// An image-based pattern (texture).
///
/// The foundation [`repub_color`] layer provides the bitmap [`Pattern`] and
/// [`PatternType`]; image-based textures are a model-level concept and live
/// here.
#[derive(Debug, Clone, PartialEq)]
pub struct TexturePattern {
    /// Image source (path or embedded)
    pub source: String,
    /// Tile mode
    pub tile: TileMode,
    /// Scale X
    pub scale_x: f64,
    /// Scale Y
    pub scale_y: f64,
    /// Offset X (0-1)
    pub offset_x: f64,
    /// Offset Y (0-1)
    pub offset_y: f64,
    /// Rotation in degrees
    pub rotation: f64,
    /// Transparency (0 = opaque, 1 = fully transparent)
    pub transparency: f64,
}

impl Default for TexturePattern {
    fn default() -> Self {
        Self {
            source: String::new(),
            tile: TileMode::Tile,
            scale_x: 1.0,
            scale_y: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            rotation: 0.0,
            transparency: 0.0,
        }
    }
}

impl TexturePattern {
    /// Create a texture pattern from an image.
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            ..Default::default()
        }
    }

    /// Set uniform scale.
    pub fn with_scale(mut self, scale: f64) -> Self {
        self.scale_x = scale;
        self.scale_y = scale;
        self
    }

    /// Set non-uniform scale.
    pub fn with_scale_xy(mut self, scale_x: f64, scale_y: f64) -> Self {
        self.scale_x = scale_x;
        self.scale_y = scale_y;
        self
    }

    /// Set the tile mode.
    pub fn with_tile_mode(mut self, tile: TileMode) -> Self {
        self.tile = tile;
        self
    }
}

/// Tile mode for texture patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileMode {
    /// Tile the image
    #[default]
    Tile,
    /// Tile horizontally, stretch vertically
    TileHorizontal,
    /// Tile vertically, stretch horizontally
    TileVertical,
    /// Stretch to fill
    Stretch,
    /// Center without tiling
    Center,
}

/// How text wraps around an object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextWrapStyle {
    /// Text flows behind or in front of the object.
    #[default]
    None,
    /// Text wraps around the object's bounding box.
    Square,
    /// Text wraps tightly around the object's shape.
    Tight,
    /// Text flows through open areas in the object.
    Through,
    /// Text appears only above and below the object.
    TopAndBottom,
}

impl TextWrapStyle {
    /// Human-readable name of the wrap style.
    pub fn name(&self) -> &'static str {
        match self {
            TextWrapStyle::None => "None",
            TextWrapStyle::Square => "Square",
            TextWrapStyle::Tight => "Tight",
            TextWrapStyle::Through => "Through",
            TextWrapStyle::TopAndBottom => "Top and Bottom",
        }
    }

    /// Description of the wrap style's behavior.
    pub fn description(&self) -> &'static str {
        match self {
            TextWrapStyle::None => "Text flows behind or in front of the object",
            TextWrapStyle::Square => "Text wraps around the object's bounding box",
            TextWrapStyle::Tight => "Text wraps tightly around the object's shape",
            TextWrapStyle::Through => "Text flows through open areas in the object",
            TextWrapStyle::TopAndBottom => "Text appears only above and below the object",
        }
    }

    /// All wrap styles.
    pub const ALL: [TextWrapStyle; 5] = [
        TextWrapStyle::None,
        TextWrapStyle::Square,
        TextWrapStyle::Tight,
        TextWrapStyle::Through,
        TextWrapStyle::TopAndBottom,
    ];
}

/// Side of the object text should wrap to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextWrapSide {
    /// Wrap on both sides of the object.
    #[default]
    Both,
    /// Wrap on the left side only.
    Left,
    /// Wrap on the right side only.
    Right,
    /// Wrap on the larger side only.
    Largest,
}

impl TextWrapSide {
    /// Human-readable name of the wrap side.
    pub fn name(&self) -> &'static str {
        match self {
            TextWrapSide::Both => "Both sides",
            TextWrapSide::Left => "Left only",
            TextWrapSide::Right => "Right only",
            TextWrapSide::Largest => "Largest side",
        }
    }

    /// All wrap sides.
    pub const ALL: [TextWrapSide; 4] = [
        TextWrapSide::Both,
        TextWrapSide::Left,
        TextWrapSide::Right,
        TextWrapSide::Largest,
    ];
}

/// Complete text wrap settings for an object.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TextWrap {
    /// Wrap style.
    pub style: TextWrapStyle,
    /// Side to wrap on.
    pub side: TextWrapSide,
    /// Top margin in points.
    pub margin_top: f64,
    /// Bottom margin in points.
    pub margin_bottom: f64,
    /// Left margin in points.
    pub margin_left: f64,
    /// Right margin in points.
    pub margin_right: f64,
}

impl TextWrap {
    /// Create default (no-wrap) settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create wrap settings with the given style and default 4pt margins.
    pub fn with_style(style: TextWrapStyle) -> Self {
        Self {
            style,
            side: TextWrapSide::Both,
            margin_top: 4.0,
            margin_bottom: 4.0,
            margin_left: 4.0,
            margin_right: 4.0,
        }
    }
}

/// A named character style.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedCharacterStyle {
    /// Unique identifier
    pub id: StyleId,
    /// Style name
    pub name: String,
    /// Parent style (for inheritance)
    pub parent: Option<StyleId>,
    /// The style properties
    pub style: CharacterStyle,
    /// Whether this is a built-in style
    pub builtin: bool,
}

impl NamedCharacterStyle {
    /// Create a new named character style.
    pub fn new(name: impl Into<String>, style: CharacterStyle) -> Self {
        Self {
            id: StyleId::new(),
            name: name.into(),
            parent: None,
            style,
            builtin: false,
        }
    }

    /// Create with a parent style.
    pub fn with_parent(mut self, parent: StyleId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Mark as builtin.
    pub fn builtin(mut self) -> Self {
        self.builtin = true;
        self
    }
}

/// A named paragraph style.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedParagraphStyle {
    /// Unique identifier
    pub id: StyleId,
    /// Style name
    pub name: String,
    /// Parent style (for inheritance)
    pub parent: Option<StyleId>,
    /// The style properties
    pub style: ParagraphStyle,
    /// Next style (applied after pressing Enter)
    pub next_style: Option<StyleId>,
    /// Whether this is a built-in style
    pub builtin: bool,
}

impl NamedParagraphStyle {
    /// Create a new named paragraph style.
    pub fn new(name: impl Into<String>, style: ParagraphStyle) -> Self {
        Self {
            id: StyleId::new(),
            name: name.into(),
            parent: None,
            style,
            next_style: None,
            builtin: false,
        }
    }

    /// Create with a parent style.
    pub fn with_parent(mut self, parent: StyleId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Set the next style.
    pub fn with_next_style(mut self, next: StyleId) -> Self {
        self.next_style = Some(next);
        self
    }

    /// Mark as builtin.
    pub fn builtin(mut self) -> Self {
        self.builtin = true;
        self
    }
}

/// A collection of named styles.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct StyleSheet {
    /// Character styles
    character_styles: HashMap<StyleId, NamedCharacterStyle>,
    /// Character styles by name
    character_by_name: HashMap<String, StyleId>,

    /// Paragraph styles
    paragraph_styles: HashMap<StyleId, NamedParagraphStyle>,
    /// Paragraph styles by name
    paragraph_by_name: HashMap<String, StyleId>,

    /// Default character style
    pub default_character_style: Option<StyleId>,
    /// Default paragraph style
    pub default_paragraph_style: Option<StyleId>,
}

impl StyleSheet {
    /// Create a new empty stylesheet.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a stylesheet with default styles.
    pub fn with_defaults() -> Self {
        let mut sheet = Self::new();
        sheet.add_default_styles();
        sheet
    }

    /// Add the default built-in styles.
    pub fn add_default_styles(&mut self) {
        // Default character style (Normal)
        let normal_char = NamedCharacterStyle::new("Normal", CharacterStyle::default()).builtin();
        let normal_char_id = normal_char.id;
        self.add_character_style(normal_char);
        self.default_character_style = Some(normal_char_id);

        // Heading character styles
        self.add_character_style(
            NamedCharacterStyle::new(
                "Heading 1",
                CharacterStyle::default().with_size(24.0).bold(),
            )
            .with_parent(normal_char_id)
            .builtin(),
        );

        self.add_character_style(
            NamedCharacterStyle::new(
                "Heading 2",
                CharacterStyle::default().with_size(18.0).bold(),
            )
            .with_parent(normal_char_id)
            .builtin(),
        );

        self.add_character_style(
            NamedCharacterStyle::new(
                "Heading 3",
                CharacterStyle::default().with_size(14.0).bold(),
            )
            .with_parent(normal_char_id)
            .builtin(),
        );

        // Emphasis styles
        self.add_character_style(
            NamedCharacterStyle::new("Emphasis", CharacterStyle::default().italic())
                .with_parent(normal_char_id)
                .builtin(),
        );

        self.add_character_style(
            NamedCharacterStyle::new("Strong", CharacterStyle::default().bold())
                .with_parent(normal_char_id)
                .builtin(),
        );

        // Default paragraph style
        let normal_para = NamedParagraphStyle::new("Normal", ParagraphStyle::default()).builtin();
        let normal_para_id = normal_para.id;
        self.add_paragraph_style(normal_para);
        self.default_paragraph_style = Some(normal_para_id);

        // Heading paragraph styles
        self.add_paragraph_style(
            NamedParagraphStyle::new(
                "Heading 1",
                ParagraphStyle {
                    space_before: 12.0,
                    space_after: 6.0,
                    ..Default::default()
                },
            )
            .with_parent(normal_para_id)
            .with_next_style(normal_para_id)
            .builtin(),
        );

        self.add_paragraph_style(
            NamedParagraphStyle::new(
                "Heading 2",
                ParagraphStyle {
                    space_before: 10.0,
                    space_after: 4.0,
                    ..Default::default()
                },
            )
            .with_parent(normal_para_id)
            .with_next_style(normal_para_id)
            .builtin(),
        );
    }

    /// Add a character style.
    pub fn add_character_style(&mut self, style: NamedCharacterStyle) {
        self.character_by_name.insert(style.name.clone(), style.id);
        self.character_styles.insert(style.id, style);
    }

    /// Get a character style by ID.
    pub fn character_style(&self, id: StyleId) -> Option<&NamedCharacterStyle> {
        self.character_styles.get(&id)
    }

    /// Get a character style by name.
    pub fn character_style_by_name(&self, name: &str) -> Option<&NamedCharacterStyle> {
        self.character_by_name
            .get(name)
            .and_then(|id| self.character_styles.get(id))
    }

    /// Get all character styles.
    pub fn character_styles(&self) -> impl Iterator<Item = &NamedCharacterStyle> {
        self.character_styles.values()
    }

    /// Remove a character style.
    pub fn remove_character_style(&mut self, id: StyleId) -> Option<NamedCharacterStyle> {
        if let Some(style) = self.character_styles.remove(&id) {
            self.character_by_name.remove(&style.name);
            Some(style)
        } else {
            None
        }
    }

    /// Resolve a character style with inheritance.
    pub fn resolve_character_style(&self, id: StyleId) -> CharacterStyle {
        let mut result = CharacterStyle::default();

        // Build inheritance chain
        let mut chain = Vec::new();
        let mut current = Some(id);
        while let Some(style_id) = current {
            if let Some(style) = self.character_styles.get(&style_id) {
                chain.push(&style.style);
                current = style.parent;
            } else {
                break;
            }
        }

        // Apply from base to derived
        for style in chain.into_iter().rev() {
            result = style.clone();
        }

        result
    }

    /// Add a paragraph style.
    pub fn add_paragraph_style(&mut self, style: NamedParagraphStyle) {
        self.paragraph_by_name.insert(style.name.clone(), style.id);
        self.paragraph_styles.insert(style.id, style);
    }

    /// Get a paragraph style by ID.
    pub fn paragraph_style(&self, id: StyleId) -> Option<&NamedParagraphStyle> {
        self.paragraph_styles.get(&id)
    }

    /// Get a paragraph style by name.
    pub fn paragraph_style_by_name(&self, name: &str) -> Option<&NamedParagraphStyle> {
        self.paragraph_by_name
            .get(name)
            .and_then(|id| self.paragraph_styles.get(id))
    }

    /// Get all paragraph styles.
    pub fn paragraph_styles(&self) -> impl Iterator<Item = &NamedParagraphStyle> {
        self.paragraph_styles.values()
    }

    /// Remove a paragraph style.
    pub fn remove_paragraph_style(&mut self, id: StyleId) -> Option<NamedParagraphStyle> {
        if let Some(style) = self.paragraph_styles.remove(&id) {
            self.paragraph_by_name.remove(&style.name);
            Some(style)
        } else {
            None
        }
    }

    /// Resolve a paragraph style with inheritance.
    pub fn resolve_paragraph_style(&self, id: StyleId) -> ParagraphStyle {
        let mut result = ParagraphStyle::default();

        let mut chain = Vec::new();
        let mut current = Some(id);
        while let Some(style_id) = current {
            if let Some(style) = self.paragraph_styles.get(&style_id) {
                chain.push(&style.style);
                current = style.parent;
            } else {
                break;
            }
        }

        for style in chain.into_iter().rev() {
            result = style.clone();
        }

        result
    }

    /// Rebuild the name indexes (call after reconstructing the style maps).
    pub fn rebuild_indexes(&mut self) {
        self.character_by_name.clear();
        for (id, style) in &self.character_styles {
            self.character_by_name.insert(style.name.clone(), *id);
        }

        self.paragraph_by_name.clear();
        for (id, style) in &self.paragraph_styles {
            self.paragraph_by_name.insert(style.name.clone(), *id);
        }
    }

    /// Get the total number of styles.
    pub fn style_count(&self) -> usize {
        self.character_styles.len() + self.paragraph_styles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_character_style_builder() {
        let style = CharacterStyle::new()
            .with_font("Times New Roman")
            .with_size(14.0)
            .bold()
            .with_color(Color::BLUE);

        assert_eq!(style.font.family, "Times New Roman");
        assert_eq!(style.font_size, 14.0);
        assert_eq!(style.font.weight, FontWeight::Bold);
        assert_eq!(style.color, Color::BLUE);
    }

    #[test]
    fn test_line_spacing() {
        let single = LineSpacing::Single;
        let double = LineSpacing::Double;
        let exact = LineSpacing::Exact(20.0);

        assert!((single.calculate(12.0) - 14.4).abs() < 0.1);
        assert!((double.calculate(12.0) - 24.0).abs() < 0.1);
        assert!((exact.calculate(12.0) - 20.0).abs() < 0.1);
    }

    #[test]
    fn test_linear_gradient() {
        let gradient = LinearGradient::horizontal(Color::RED, Color::BLUE);
        assert_eq!(gradient.stops.len(), 2);
        assert_eq!(gradient.stops[0].color, Color::RED);
        assert_eq!(gradient.stops[1].color, Color::BLUE);
    }

    #[test]
    fn test_stroke_dashed() {
        let stroke = Stroke::dashed(Color::BLACK, 2.0, 5.0, 3.0);
        assert_eq!(stroke.dash_pattern, Some(vec![5.0, 3.0]));
    }

    #[test]
    fn test_drop_shadow() {
        let shadow = DropShadow::soft();
        assert!((shadow.blur_radius - 8.0).abs() < 0.01);
    }

    #[test]
    fn test_reflection() {
        let reflection = Reflection::half();
        assert!((reflection.size - 0.5).abs() < 0.01);
        assert!((reflection.gap - 4.0).abs() < 0.01);
    }

    #[test]
    fn test_bevel() {
        let bevel = Bevel::emboss().with_size(10.0).chisel_hard();
        assert!(matches!(bevel.style, BevelStyle::Emboss));
        assert!(matches!(bevel.technique, BevelTechnique::ChiselHard));
        assert!((bevel.size - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_effects_builder() {
        let effects = Effects::new()
            .with_drop_shadow(DropShadow::default())
            .with_reflection(Reflection::tight())
            .with_soft_edges(5.0);

        assert!(effects.has_effects());
        assert!(effects.drop_shadow.is_some());
        assert!(effects.reflection.is_some());
        assert!(effects.soft_edges.is_some());
        assert!(effects.bevel.is_none());
    }

    #[test]
    fn texture_pattern() {
        let texture = TexturePattern::new("texture.png")
            .with_scale(0.5)
            .with_tile_mode(TileMode::TileHorizontal);

        assert_eq!(texture.source, "texture.png");
        assert!((texture.scale_x - 0.5).abs() < 0.01);
        assert!(matches!(texture.tile, TileMode::TileHorizontal));
    }

    #[test]
    fn test_text_wrap_style_all() {
        assert_eq!(TextWrapStyle::ALL.len(), 5);
        assert_eq!(TextWrapSide::ALL.len(), 4);
        let wrap = TextWrap::with_style(TextWrapStyle::Square);
        assert_eq!(wrap.style, TextWrapStyle::Square);
        assert_eq!(wrap.margin_top, 4.0);
    }

    #[test]
    fn test_stylesheet_with_defaults() {
        let sheet = StyleSheet::with_defaults();

        assert!(sheet.default_character_style.is_some());
        assert!(sheet.default_paragraph_style.is_some());

        // Should have Normal and heading styles
        assert!(sheet.character_style_by_name("Normal").is_some());
        assert!(sheet.character_style_by_name("Heading 1").is_some());
        assert!(sheet.paragraph_style_by_name("Normal").is_some());
    }

    #[test]
    fn test_style_inheritance() {
        let mut sheet = StyleSheet::new();

        // Base style
        let base = CharacterStyle {
            font_size: 12.0,
            ..CharacterStyle::default()
        };
        let base_named = NamedCharacterStyle::new("Base", base);
        let base_id = base_named.id;
        sheet.add_character_style(base_named);

        // Derived style
        let derived = CharacterStyle {
            font_size: 16.0,
            font: FontSpec {
                weight: FontWeight::Bold,
                ..FontSpec::default()
            },
            ..CharacterStyle::default()
        };
        let derived_named = NamedCharacterStyle::new("Derived", derived).with_parent(base_id);
        let derived_id = derived_named.id;
        sheet.add_character_style(derived_named);

        // Resolve derived style
        let resolved = sheet.resolve_character_style(derived_id);
        assert!((resolved.font_size - 16.0).abs() < 0.01);
        assert_eq!(resolved.font.weight, FontWeight::Bold);
    }

    #[test]
    fn test_remove_style() {
        let mut sheet = StyleSheet::with_defaults();
        let initial_count = sheet.character_styles().count();

        let custom = NamedCharacterStyle::new("ToRemove", CharacterStyle::default());
        let id = custom.id;
        sheet.add_character_style(custom);

        assert_eq!(sheet.character_styles().count(), initial_count + 1);

        sheet.remove_character_style(id);
        assert_eq!(sheet.character_styles().count(), initial_count);
        assert!(sheet.character_style_by_name("ToRemove").is_none());
    }
}
