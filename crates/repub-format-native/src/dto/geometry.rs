//! Serde DTOs for foundation geometry and color types.
//!
//! `repub_geometry` and `repub_color` are serialization-free, so this module
//! provides mirror structs with `serde` derives and `From` conversions in both
//! directions.

use repub_color::{
    Color, GradientStop, LinearGradient, Pattern, PatternType, RadialGradient,
};
use repub_geometry::{Point, Rect, Size, Transform2D};
use serde::{Deserialize, Serialize};

/// DTO for [`Point`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PointDto {
    /// X coordinate in points.
    pub x: f64,
    /// Y coordinate in points.
    pub y: f64,
}

impl From<Point> for PointDto {
    fn from(p: Point) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl From<PointDto> for Point {
    fn from(p: PointDto) -> Self {
        Point::new(p.x, p.y)
    }
}

/// DTO for [`Size`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SizeDto {
    /// Width in points.
    pub width: f64,
    /// Height in points.
    pub height: f64,
}

impl From<Size> for SizeDto {
    fn from(s: Size) -> Self {
        Self {
            width: s.width,
            height: s.height,
        }
    }
}

impl From<SizeDto> for Size {
    fn from(s: SizeDto) -> Self {
        Size::new(s.width, s.height)
    }
}

/// DTO for [`Rect`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RectDto {
    /// X of the origin, in points.
    pub x: f64,
    /// Y of the origin, in points.
    pub y: f64,
    /// Width in points.
    pub width: f64,
    /// Height in points.
    pub height: f64,
}

impl From<Rect> for RectDto {
    fn from(r: Rect) -> Self {
        Self {
            x: r.x(),
            y: r.y(),
            width: r.width(),
            height: r.height(),
        }
    }
}

impl From<RectDto> for Rect {
    fn from(r: RectDto) -> Self {
        Rect::new(r.x, r.y, r.width, r.height)
    }
}

/// DTO for [`Transform2D`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TransformDto {
    /// Matrix component a.
    pub a: f64,
    /// Matrix component b.
    pub b: f64,
    /// Matrix component c.
    pub c: f64,
    /// Matrix component d.
    pub d: f64,
    /// Translation x.
    pub tx: f64,
    /// Translation y.
    pub ty: f64,
}

impl From<Transform2D> for TransformDto {
    fn from(t: Transform2D) -> Self {
        Self {
            a: t.a,
            b: t.b,
            c: t.c,
            d: t.d,
            tx: t.tx,
            ty: t.ty,
        }
    }
}

impl From<TransformDto> for Transform2D {
    fn from(t: TransformDto) -> Self {
        Transform2D::new(t.a, t.b, t.c, t.d, t.tx, t.ty)
    }
}

/// DTO for [`Color`] (RGBA, components 0–1).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ColorDto {
    /// Red (0–1).
    pub r: f64,
    /// Green (0–1).
    pub g: f64,
    /// Blue (0–1).
    pub b: f64,
    /// Alpha (0–1).
    pub a: f64,
}

impl From<Color> for ColorDto {
    fn from(c: Color) -> Self {
        Self {
            r: c.r,
            g: c.g,
            b: c.b,
            a: c.a,
        }
    }
}

impl From<ColorDto> for Color {
    fn from(c: ColorDto) -> Self {
        Color::new(c.r, c.g, c.b, c.a)
    }
}

/// DTO for [`GradientStop`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct GradientStopDto {
    /// Position (0–1).
    pub position: f64,
    /// Color at the stop.
    pub color: ColorDto,
}

impl From<GradientStop> for GradientStopDto {
    fn from(s: GradientStop) -> Self {
        Self {
            position: s.position,
            color: s.color.into(),
        }
    }
}

impl From<GradientStopDto> for GradientStop {
    fn from(s: GradientStopDto) -> Self {
        GradientStop::new(s.position, s.color.into())
    }
}

/// DTO for [`LinearGradient`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinearGradientDto {
    /// Start point (normalized 0–1).
    pub start: PointDto,
    /// End point (normalized 0–1).
    pub end: PointDto,
    /// Color stops.
    pub stops: Vec<GradientStopDto>,
}

impl From<LinearGradient> for LinearGradientDto {
    fn from(g: LinearGradient) -> Self {
        Self {
            start: g.start.into(),
            end: g.end.into(),
            stops: g.stops.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<LinearGradientDto> for LinearGradient {
    fn from(g: LinearGradientDto) -> Self {
        LinearGradient {
            start: g.start.into(),
            end: g.end.into(),
            stops: g.stops.into_iter().map(Into::into).collect(),
        }
    }
}

/// DTO for [`RadialGradient`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadialGradientDto {
    /// Center point (normalized 0–1).
    pub center: PointDto,
    /// Radius (normalized).
    pub radius: f64,
    /// Color stops.
    pub stops: Vec<GradientStopDto>,
}

impl From<RadialGradient> for RadialGradientDto {
    fn from(g: RadialGradient) -> Self {
        Self {
            center: g.center.into(),
            radius: g.radius,
            stops: g.stops.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<RadialGradientDto> for RadialGradient {
    fn from(g: RadialGradientDto) -> Self {
        RadialGradient {
            center: g.center.into(),
            radius: g.radius,
            stops: g.stops.into_iter().map(Into::into).collect(),
        }
    }
}

/// DTO for [`PatternType`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PatternTypeDto {
    /// Solid fill.
    Solid,
    /// Other pattern types are preserved by name.
    Named(PatternKindDto),
}

/// A serializable mirror of each [`PatternType`] variant.
///
/// All variants are enumerated so the round trip is lossless.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum PatternKindDto {
    Percent5,
    Percent10,
    Percent20,
    Percent25,
    Percent30,
    Percent40,
    Percent50,
    Percent60,
    Percent70,
    Percent75,
    Percent80,
    Percent90,
    LightHorizontal,
    LightVertical,
    LightDownwardDiagonal,
    LightUpwardDiagonal,
    DarkHorizontal,
    DarkVertical,
    DarkDownwardDiagonal,
    DarkUpwardDiagonal,
    WideDownwardDiagonal,
    WideUpwardDiagonal,
    HorizontalStripe,
    VerticalStripe,
    DiagonalStripeRight,
    DiagonalStripeLeft,
    LightHorizontalCross,
    LightDiagonalCross,
    DarkHorizontalCross,
    DarkDiagonalCross,
    Cross,
    DiagonalCross,
    SmallGrid,
    LargeGrid,
    DottedGrid,
    SmallChecker,
    LargeChecker,
    HorizontalBrick,
    DiagonalBrick,
    Weave,
    Plaid,
    Divot,
    DottedDiamond,
    SmallConfetti,
    LargeConfetti,
    ZigZag,
    Wave,
    Shingle,
    Trellis,
    Sphere,
    OutlinedDiamond,
    SolidDiamond,
}

impl From<PatternType> for PatternTypeDto {
    fn from(p: PatternType) -> Self {
        use PatternKindDto as K;
        use PatternType as P;
        match p {
            P::Solid => PatternTypeDto::Solid,
            P::Percent5 => PatternTypeDto::Named(K::Percent5),
            P::Percent10 => PatternTypeDto::Named(K::Percent10),
            P::Percent20 => PatternTypeDto::Named(K::Percent20),
            P::Percent25 => PatternTypeDto::Named(K::Percent25),
            P::Percent30 => PatternTypeDto::Named(K::Percent30),
            P::Percent40 => PatternTypeDto::Named(K::Percent40),
            P::Percent50 => PatternTypeDto::Named(K::Percent50),
            P::Percent60 => PatternTypeDto::Named(K::Percent60),
            P::Percent70 => PatternTypeDto::Named(K::Percent70),
            P::Percent75 => PatternTypeDto::Named(K::Percent75),
            P::Percent80 => PatternTypeDto::Named(K::Percent80),
            P::Percent90 => PatternTypeDto::Named(K::Percent90),
            P::LightHorizontal => PatternTypeDto::Named(K::LightHorizontal),
            P::LightVertical => PatternTypeDto::Named(K::LightVertical),
            P::LightDownwardDiagonal => PatternTypeDto::Named(K::LightDownwardDiagonal),
            P::LightUpwardDiagonal => PatternTypeDto::Named(K::LightUpwardDiagonal),
            P::DarkHorizontal => PatternTypeDto::Named(K::DarkHorizontal),
            P::DarkVertical => PatternTypeDto::Named(K::DarkVertical),
            P::DarkDownwardDiagonal => PatternTypeDto::Named(K::DarkDownwardDiagonal),
            P::DarkUpwardDiagonal => PatternTypeDto::Named(K::DarkUpwardDiagonal),
            P::WideDownwardDiagonal => PatternTypeDto::Named(K::WideDownwardDiagonal),
            P::WideUpwardDiagonal => PatternTypeDto::Named(K::WideUpwardDiagonal),
            P::HorizontalStripe => PatternTypeDto::Named(K::HorizontalStripe),
            P::VerticalStripe => PatternTypeDto::Named(K::VerticalStripe),
            P::DiagonalStripeRight => PatternTypeDto::Named(K::DiagonalStripeRight),
            P::DiagonalStripeLeft => PatternTypeDto::Named(K::DiagonalStripeLeft),
            P::LightHorizontalCross => PatternTypeDto::Named(K::LightHorizontalCross),
            P::LightDiagonalCross => PatternTypeDto::Named(K::LightDiagonalCross),
            P::DarkHorizontalCross => PatternTypeDto::Named(K::DarkHorizontalCross),
            P::DarkDiagonalCross => PatternTypeDto::Named(K::DarkDiagonalCross),
            P::Cross => PatternTypeDto::Named(K::Cross),
            P::DiagonalCross => PatternTypeDto::Named(K::DiagonalCross),
            P::SmallGrid => PatternTypeDto::Named(K::SmallGrid),
            P::LargeGrid => PatternTypeDto::Named(K::LargeGrid),
            P::DottedGrid => PatternTypeDto::Named(K::DottedGrid),
            P::SmallChecker => PatternTypeDto::Named(K::SmallChecker),
            P::LargeChecker => PatternTypeDto::Named(K::LargeChecker),
            P::HorizontalBrick => PatternTypeDto::Named(K::HorizontalBrick),
            P::DiagonalBrick => PatternTypeDto::Named(K::DiagonalBrick),
            P::Weave => PatternTypeDto::Named(K::Weave),
            P::Plaid => PatternTypeDto::Named(K::Plaid),
            P::Divot => PatternTypeDto::Named(K::Divot),
            P::DottedDiamond => PatternTypeDto::Named(K::DottedDiamond),
            P::SmallConfetti => PatternTypeDto::Named(K::SmallConfetti),
            P::LargeConfetti => PatternTypeDto::Named(K::LargeConfetti),
            P::ZigZag => PatternTypeDto::Named(K::ZigZag),
            P::Wave => PatternTypeDto::Named(K::Wave),
            P::Shingle => PatternTypeDto::Named(K::Shingle),
            P::Trellis => PatternTypeDto::Named(K::Trellis),
            P::Sphere => PatternTypeDto::Named(K::Sphere),
            P::OutlinedDiamond => PatternTypeDto::Named(K::OutlinedDiamond),
            P::SolidDiamond => PatternTypeDto::Named(K::SolidDiamond),
        }
    }
}

impl From<PatternTypeDto> for PatternType {
    fn from(p: PatternTypeDto) -> Self {
        use PatternKindDto as K;
        use PatternType as P;
        match p {
            PatternTypeDto::Solid => P::Solid,
            PatternTypeDto::Named(k) => match k {
                K::Percent5 => P::Percent5,
                K::Percent10 => P::Percent10,
                K::Percent20 => P::Percent20,
                K::Percent25 => P::Percent25,
                K::Percent30 => P::Percent30,
                K::Percent40 => P::Percent40,
                K::Percent50 => P::Percent50,
                K::Percent60 => P::Percent60,
                K::Percent70 => P::Percent70,
                K::Percent75 => P::Percent75,
                K::Percent80 => P::Percent80,
                K::Percent90 => P::Percent90,
                K::LightHorizontal => P::LightHorizontal,
                K::LightVertical => P::LightVertical,
                K::LightDownwardDiagonal => P::LightDownwardDiagonal,
                K::LightUpwardDiagonal => P::LightUpwardDiagonal,
                K::DarkHorizontal => P::DarkHorizontal,
                K::DarkVertical => P::DarkVertical,
                K::DarkDownwardDiagonal => P::DarkDownwardDiagonal,
                K::DarkUpwardDiagonal => P::DarkUpwardDiagonal,
                K::WideDownwardDiagonal => P::WideDownwardDiagonal,
                K::WideUpwardDiagonal => P::WideUpwardDiagonal,
                K::HorizontalStripe => P::HorizontalStripe,
                K::VerticalStripe => P::VerticalStripe,
                K::DiagonalStripeRight => P::DiagonalStripeRight,
                K::DiagonalStripeLeft => P::DiagonalStripeLeft,
                K::LightHorizontalCross => P::LightHorizontalCross,
                K::LightDiagonalCross => P::LightDiagonalCross,
                K::DarkHorizontalCross => P::DarkHorizontalCross,
                K::DarkDiagonalCross => P::DarkDiagonalCross,
                K::Cross => P::Cross,
                K::DiagonalCross => P::DiagonalCross,
                K::SmallGrid => P::SmallGrid,
                K::LargeGrid => P::LargeGrid,
                K::DottedGrid => P::DottedGrid,
                K::SmallChecker => P::SmallChecker,
                K::LargeChecker => P::LargeChecker,
                K::HorizontalBrick => P::HorizontalBrick,
                K::DiagonalBrick => P::DiagonalBrick,
                K::Weave => P::Weave,
                K::Plaid => P::Plaid,
                K::Divot => P::Divot,
                K::DottedDiamond => P::DottedDiamond,
                K::SmallConfetti => P::SmallConfetti,
                K::LargeConfetti => P::LargeConfetti,
                K::ZigZag => P::ZigZag,
                K::Wave => P::Wave,
                K::Shingle => P::Shingle,
                K::Trellis => P::Trellis,
                K::Sphere => P::Sphere,
                K::OutlinedDiamond => P::OutlinedDiamond,
                K::SolidDiamond => P::SolidDiamond,
            },
        }
    }
}

/// DTO for [`Pattern`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternDto {
    /// Pattern type.
    pub pattern_type: PatternTypeDto,
    /// Foreground color.
    pub foreground: ColorDto,
    /// Background color.
    pub background: ColorDto,
    /// Scale factor.
    pub scale: f64,
    /// Rotation in degrees.
    pub rotation: f64,
}

impl From<Pattern> for PatternDto {
    fn from(p: Pattern) -> Self {
        Self {
            pattern_type: p.pattern_type.into(),
            foreground: p.foreground.into(),
            background: p.background.into(),
            scale: p.scale,
            rotation: p.rotation,
        }
    }
}

impl From<PatternDto> for Pattern {
    fn from(p: PatternDto) -> Self {
        Pattern {
            pattern_type: p.pattern_type.into(),
            foreground: p.foreground.into(),
            background: p.background.into(),
            scale: p.scale,
            rotation: p.rotation,
        }
    }
}
