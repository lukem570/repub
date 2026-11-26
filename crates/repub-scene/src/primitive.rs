//! The flat per-frame primitive vocabulary a [`Scene`](crate::Scene) is built
//! from.
//!
//! A scene is an immutable, flat list of [`Primitive`]s produced fresh by
//! [`compose`](crate::compose) for one page. Each primitive carries its own
//! transform, an optional clip rectangle, and a list of [`SceneEffect`]
//! descriptors. Geometry (warps, text-on-path, 3D extrusion) is fully resolved
//! into concrete paths and glyph runs; blur-style effects are left as data for
//! a backend to realize.

use repub_color::{Color, LinearGradient, Pattern, RadialGradient};
use repub_geometry::{Path, Rect, Size, Transform2D};

/// How a filled path decides which regions are inside.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FillRule {
    /// Nonzero winding rule, as Office fills its shapes.
    #[default]
    NonZero,
    /// Even-odd rule (overlapping subpaths cut holes).
    EvenOdd,
}

/// How the ends of an open stroked path are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineCap {
    /// Flat cap flush with the endpoint.
    #[default]
    Butt,
    /// Rounded cap.
    Round,
    /// Square cap extending past the endpoint.
    Square,
}

/// How corners between stroked segments are joined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineJoin {
    /// Sharp mitered corner.
    #[default]
    Miter,
    /// Rounded corner.
    Round,
    /// Flattened (beveled) corner.
    Bevel,
}

impl From<repub_model::styles::LineCap> for LineCap {
    fn from(c: repub_model::styles::LineCap) -> Self {
        match c {
            repub_model::styles::LineCap::Butt => LineCap::Butt,
            repub_model::styles::LineCap::Round => LineCap::Round,
            repub_model::styles::LineCap::Square => LineCap::Square,
        }
    }
}

impl From<repub_model::styles::LineJoin> for LineJoin {
    fn from(j: repub_model::styles::LineJoin) -> Self {
        match j {
            repub_model::styles::LineJoin::Miter => LineJoin::Miter,
            repub_model::styles::LineJoin::Round => LineJoin::Round,
            repub_model::styles::LineJoin::Bevel => LineJoin::Bevel,
        }
    }
}

/// A concrete paint for a filled or stroked primitive.
///
/// All colors are concrete: scheme-color references are resolved against the
/// document's [`ColorScheme`](repub_color::ColorScheme) during
/// [`compose`](crate::compose). Gradient variants carry the on-page bounds the
/// normalized gradient coordinates map into, so a backend needs no further
/// context to realize them.
#[derive(Debug, Clone, PartialEq)]
pub enum Paint {
    /// A single solid color.
    Solid(Color),
    /// A linear gradient, with the bounds its normalized coordinates fill.
    LinearGradient {
        /// The gradient definition (normalized 0–1 coordinates).
        gradient: LinearGradient,
        /// The on-page rectangle the gradient maps onto.
        bounds: Rect,
    },
    /// A radial gradient, with the bounds its normalized coordinates fill.
    RadialGradient {
        /// The gradient definition (normalized 0–1 coordinates).
        gradient: RadialGradient,
        /// The on-page rectangle the gradient maps onto.
        bounds: Rect,
    },
    /// A bitmap pattern fill, with the bounds it tiles over.
    Pattern {
        /// The pattern definition.
        pattern: Pattern,
        /// The on-page rectangle the pattern fills.
        bounds: Rect,
    },
}

impl Paint {
    /// A representative solid color for this paint, for simple fallbacks.
    pub fn primary_color(&self) -> Color {
        match self {
            Paint::Solid(c) => *c,
            Paint::LinearGradient { gradient, .. } => {
                gradient.stops.first().map(|s| s.color).unwrap_or(Color::BLACK)
            }
            Paint::RadialGradient { gradient, .. } => {
                gradient.stops.first().map(|s| s.color).unwrap_or(Color::BLACK)
            }
            Paint::Pattern { pattern, .. } => pattern.foreground,
        }
    }
}

/// A reference to a font to render a [`Primitive::Glyphs`] run with.
///
/// Scene glyph runs identify their font by family, weight and style so a
/// backend can resolve the same face via a
/// [`FontProvider`](repub_fonts::FontProvider). The scene itself does not own
/// font data.
#[derive(Debug, Clone, PartialEq)]
pub struct FontRef {
    /// Font family name (e.g. "Arial").
    pub family: String,
    /// Numeric font weight (100–900).
    pub weight: u16,
    /// Whether the face is italic.
    pub italic: bool,
}

impl FontRef {
    /// Create a font reference.
    pub fn new(family: impl Into<String>, weight: u16, italic: bool) -> Self {
        Self {
            family: family.into(),
            weight,
            italic,
        }
    }
}

/// A single positioned glyph within a [`Primitive::Glyphs`] run.
///
/// Positions are relative to the run's transform origin. A backend renders the
/// glyph either from its character (resolving the glyph through the font) or,
/// when known, directly from its glyph id.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PositionedGlyph {
    /// The source character.
    pub ch: char,
    /// The resolved glyph id, if known (0 when unresolved).
    pub glyph_id: u16,
    /// X position of the glyph origin, in the run's coordinate space.
    pub x: f64,
    /// Y position of the glyph baseline, in the run's coordinate space.
    pub y: f64,
    /// Advance width of the glyph.
    pub advance: f64,
}

/// A reference to image data for a [`Primitive::Image`].
///
/// Mirrors [`repub_model::shape::ImageSource`] without owning decoded pixels:
/// a backend resolves the bytes from the path, URL or embedded data.
#[derive(Debug, Clone, PartialEq)]
pub enum ImageRef {
    /// Image from a file path.
    File(String),
    /// Image from a URL.
    Url(String),
    /// Embedded image data (base64 encoded) with its MIME type.
    Embedded {
        /// Base64-encoded image bytes.
        data: String,
        /// MIME type such as "image/png".
        mime_type: String,
    },
    /// A placeholder with no resolved source.
    Placeholder,
}

impl From<&repub_model::shape::ImageSource> for ImageRef {
    fn from(source: &repub_model::shape::ImageSource) -> Self {
        match source {
            repub_model::shape::ImageSource::File(p) => ImageRef::File(p.clone()),
            repub_model::shape::ImageSource::Url(u) => ImageRef::Url(u.clone()),
            repub_model::shape::ImageSource::Embedded { data, mime_type } => ImageRef::Embedded {
                data: data.clone(),
                mime_type: mime_type.clone(),
            },
            repub_model::shape::ImageSource::Placeholder => ImageRef::Placeholder,
        }
    }
}

/// A drawing effect carried as a data descriptor on a [`Primitive`].
///
/// Effects are not pre-baked into geometry. A CPU backend rasterizes and blurs
/// them; a GPU backend can realize them with shaders. Each variant maps from
/// the matching type in [`repub_model::styles`].
#[derive(Debug, Clone, PartialEq)]
pub enum SceneEffect {
    /// A drop shadow cast behind the primitive.
    DropShadow {
        /// Shadow color.
        color: Color,
        /// Horizontal offset in points.
        offset_x: f64,
        /// Vertical offset in points.
        offset_y: f64,
        /// Blur radius in points.
        blur_radius: f64,
        /// Spread (expand/contract) in points.
        spread: f64,
    },
    /// An inner shadow cast inside the primitive.
    InnerShadow {
        /// Shadow color.
        color: Color,
        /// Horizontal offset in points.
        offset_x: f64,
        /// Vertical offset in points.
        offset_y: f64,
        /// Blur radius in points.
        blur_radius: f64,
        /// Choke (contraction) in points.
        choke: f64,
    },
    /// A glow around the outside of the primitive.
    OuterGlow {
        /// Glow color.
        color: Color,
        /// Blur radius in points.
        radius: f64,
        /// Spread in points.
        spread: f64,
    },
    /// A glow inside the primitive.
    InnerGlow {
        /// Glow color.
        color: Color,
        /// Blur radius in points.
        radius: f64,
        /// Choke (contraction) in points.
        choke: f64,
        /// Whether the glow originates at the center rather than the edge.
        from_center: bool,
    },
    /// A faded mirror image below the primitive.
    Reflection {
        /// Opacity at the top of the reflection (0–1).
        opacity: f64,
        /// How far the reflection fades (0–1).
        fade_length: f64,
        /// Gap between the primitive and its reflection, in points.
        gap: f64,
        /// Blur amount for the reflection.
        blur: f64,
        /// Size of the reflection relative to the primitive (0–1).
        size: f64,
    },
    /// Feathered (softened) edges.
    SoftEdges {
        /// Feather radius in points.
        radius: f64,
    },
    /// A beveled 3D edge.
    Bevel {
        /// Bevel size in points.
        size: f64,
        /// Bevel depth in points.
        depth: f64,
        /// Light angle in degrees.
        light_angle: f64,
        /// Highlight color.
        highlight: Color,
        /// Shadow color.
        shadow: Color,
    },
}

/// A single drawing primitive in a [`Scene`].
///
/// Every variant carries a `transform` (the shape's full on-page transform), an
/// optional axis-aligned `clip` rectangle, and a list of [`SceneEffect`]s to be
/// realized by the backend.
#[derive(Debug, Clone, PartialEq)]
pub enum Primitive {
    /// A filled vector path.
    FillPath {
        /// The path to fill, in page coordinates before `transform`.
        path: Path,
        /// The fill paint.
        paint: Paint,
        /// The fill rule.
        fill_rule: FillRule,
        /// Transform applied to the path.
        transform: Transform2D,
        /// Optional clip rectangle in page coordinates.
        clip: Option<Rect>,
        /// Effects to realize for this primitive.
        effects: Vec<SceneEffect>,
    },
    /// A stroked vector path.
    StrokePath {
        /// The path to stroke, in page coordinates before `transform`.
        path: Path,
        /// The stroke paint.
        paint: Paint,
        /// Stroke width in points.
        width: f64,
        /// Line cap style.
        cap: LineCap,
        /// Line join style.
        join: LineJoin,
        /// Optional dash pattern (alternating dash/gap lengths).
        dash: Option<Vec<f64>>,
        /// Transform applied to the path.
        transform: Transform2D,
        /// Optional clip rectangle in page coordinates.
        clip: Option<Rect>,
        /// Effects to realize for this primitive.
        effects: Vec<SceneEffect>,
    },
    /// A run of positioned glyphs.
    Glyphs {
        /// The font the glyphs are drawn with.
        font: FontRef,
        /// Font size in points.
        size: f64,
        /// The positioned glyphs.
        glyphs: Vec<PositionedGlyph>,
        /// The text paint.
        paint: Paint,
        /// Transform applied to the glyph positions.
        transform: Transform2D,
        /// Optional clip rectangle in page coordinates.
        clip: Option<Rect>,
        /// Effects to realize for this primitive.
        effects: Vec<SceneEffect>,
    },
    /// An image drawn into a destination rectangle.
    Image {
        /// The image source reference.
        source: ImageRef,
        /// Destination rectangle in page coordinates before `transform`.
        dest: Rect,
        /// Optional source crop rectangle (normalized 0–1).
        crop: Option<Rect>,
        /// Overall opacity (0–1).
        opacity: f64,
        /// Transform applied to the destination rectangle.
        transform: Transform2D,
        /// Optional clip rectangle in page coordinates.
        clip: Option<Rect>,
        /// Effects to realize for this primitive.
        effects: Vec<SceneEffect>,
    },
}

impl Primitive {
    /// The transform applied to this primitive.
    pub fn transform(&self) -> Transform2D {
        match self {
            Primitive::FillPath { transform, .. }
            | Primitive::StrokePath { transform, .. }
            | Primitive::Glyphs { transform, .. }
            | Primitive::Image { transform, .. } => *transform,
        }
    }

    /// The clip rectangle of this primitive, if any.
    pub fn clip(&self) -> Option<Rect> {
        match self {
            Primitive::FillPath { clip, .. }
            | Primitive::StrokePath { clip, .. }
            | Primitive::Glyphs { clip, .. }
            | Primitive::Image { clip, .. } => *clip,
        }
    }

    /// The effect descriptors attached to this primitive.
    pub fn effects(&self) -> &[SceneEffect] {
        match self {
            Primitive::FillPath { effects, .. }
            | Primitive::StrokePath { effects, .. }
            | Primitive::Glyphs { effects, .. }
            | Primitive::Image { effects, .. } => effects,
        }
    }
}

/// An immutable, flat, per-frame display list for one page.
///
/// A scene owns its [`size`](Scene::size), an optional background paint, and an
/// ordered list of [`Primitive`]s in back-to-front draw order. It carries no
/// retained or diff state: [`compose`](crate::compose) produces a fresh scene
/// each call.
#[derive(Debug, Clone, PartialEq)]
pub struct Scene {
    /// The page size in points.
    pub size: Size,
    /// The page background paint, if any.
    pub background: Option<Paint>,
    /// The primitives in back-to-front draw order.
    pub primitives: Vec<Primitive>,
}

impl Scene {
    /// Create an empty scene of the given size.
    pub fn new(size: Size) -> Self {
        Self {
            size,
            background: None,
            primitives: Vec::new(),
        }
    }

    /// Append a primitive to the scene.
    pub fn push(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
    }

    /// The number of primitives in the scene.
    pub fn len(&self) -> usize {
        self.primitives.len()
    }

    /// Whether the scene has no primitives.
    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }
}
