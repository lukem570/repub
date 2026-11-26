//! Serde DTOs for auto-shape vector geometry.

use repub_model::geometry::{AutoShape, Path, PathCommand, PresetGeometry, ShapeGeometry, SubpathStyle};
use serde::{Deserialize, Serialize};

use super::geometry::PointDto;

/// DTO for [`PathCommand`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum PathCommandDto {
    MoveTo(PointDto),
    LineTo(PointDto),
    QuadTo { control: PointDto, end: PointDto },
    CubicTo {
        control1: PointDto,
        control2: PointDto,
        end: PointDto,
    },
    Close,
}

impl From<PathCommand> for PathCommandDto {
    fn from(c: PathCommand) -> Self {
        match c {
            PathCommand::MoveTo(p) => PathCommandDto::MoveTo(p.into()),
            PathCommand::LineTo(p) => PathCommandDto::LineTo(p.into()),
            PathCommand::QuadTo { control, end } => PathCommandDto::QuadTo {
                control: control.into(),
                end: end.into(),
            },
            PathCommand::CubicTo {
                control1,
                control2,
                end,
            } => PathCommandDto::CubicTo {
                control1: control1.into(),
                control2: control2.into(),
                end: end.into(),
            },
            PathCommand::Close => PathCommandDto::Close,
        }
    }
}

impl From<PathCommandDto> for PathCommand {
    fn from(c: PathCommandDto) -> Self {
        match c {
            PathCommandDto::MoveTo(p) => PathCommand::MoveTo(p.into()),
            PathCommandDto::LineTo(p) => PathCommand::LineTo(p.into()),
            PathCommandDto::QuadTo { control, end } => PathCommand::QuadTo {
                control: control.into(),
                end: end.into(),
            },
            PathCommandDto::CubicTo {
                control1,
                control2,
                end,
            } => PathCommand::CubicTo {
                control1: control1.into(),
                control2: control2.into(),
                end: end.into(),
            },
            PathCommandDto::Close => PathCommand::Close,
        }
    }
}

/// DTO for [`SubpathStyle`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SubpathStyleDto {
    /// Not filled.
    pub no_fill: bool,
    /// Not outlined.
    pub no_stroke: bool,
}

impl From<SubpathStyle> for SubpathStyleDto {
    fn from(s: SubpathStyle) -> Self {
        Self {
            no_fill: s.no_fill,
            no_stroke: s.no_stroke,
        }
    }
}

impl From<SubpathStyleDto> for SubpathStyle {
    fn from(s: SubpathStyleDto) -> Self {
        SubpathStyle {
            no_fill: s.no_fill,
            no_stroke: s.no_stroke,
        }
    }
}

/// DTO for the model's auto-shape [`Path`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ModelPathDto {
    /// Path commands.
    pub commands: Vec<PathCommandDto>,
    /// Per-subpath drawing flags.
    pub subpaths: Vec<SubpathStyleDto>,
    /// Even-odd fill flag.
    pub even_odd: bool,
}

impl From<Path> for ModelPathDto {
    fn from(p: Path) -> Self {
        Self {
            commands: p.commands.into_iter().map(Into::into).collect(),
            subpaths: p.subpaths.into_iter().map(Into::into).collect(),
            even_odd: p.even_odd,
        }
    }
}

impl From<ModelPathDto> for Path {
    fn from(p: ModelPathDto) -> Self {
        Path {
            commands: p.commands.into_iter().map(Into::into).collect(),
            subpaths: p.subpaths.into_iter().map(Into::into).collect(),
            even_odd: p.even_odd,
        }
    }
}

/// DTO for [`ShapeGeometry`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum ShapeGeometryDto {
    Rectangle { corner_radius: f64 },
    Ellipse,
    Polygon { sides: u32 },
    Star { points: u32, inner_ratio: f64 },
    Path(ModelPathDto),
}

impl From<ShapeGeometry> for ShapeGeometryDto {
    fn from(g: ShapeGeometry) -> Self {
        match g {
            ShapeGeometry::Rectangle { corner_radius } => {
                ShapeGeometryDto::Rectangle { corner_radius }
            }
            ShapeGeometry::Ellipse => ShapeGeometryDto::Ellipse,
            ShapeGeometry::Polygon { sides } => ShapeGeometryDto::Polygon { sides },
            ShapeGeometry::Star {
                points,
                inner_ratio,
            } => ShapeGeometryDto::Star {
                points,
                inner_ratio,
            },
            ShapeGeometry::Path(p) => ShapeGeometryDto::Path(p.into()),
        }
    }
}

impl From<ShapeGeometryDto> for ShapeGeometry {
    fn from(g: ShapeGeometryDto) -> Self {
        match g {
            ShapeGeometryDto::Rectangle { corner_radius } => {
                ShapeGeometry::Rectangle { corner_radius }
            }
            ShapeGeometryDto::Ellipse => ShapeGeometry::Ellipse,
            ShapeGeometryDto::Polygon { sides } => ShapeGeometry::Polygon { sides },
            ShapeGeometryDto::Star {
                points,
                inner_ratio,
            } => ShapeGeometry::Star {
                points,
                inner_ratio,
            },
            ShapeGeometryDto::Path(p) => ShapeGeometry::Path(p.into()),
        }
    }
}

/// DTO for [`PresetGeometry`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresetGeometryDto {
    /// Escher shape type.
    pub spt: u16,
    /// Adjust-value overrides.
    pub adjust: Vec<Option<i32>>,
}

impl From<PresetGeometry> for PresetGeometryDto {
    fn from(p: PresetGeometry) -> Self {
        Self {
            spt: p.spt,
            adjust: p.adjust,
        }
    }
}

impl From<PresetGeometryDto> for PresetGeometry {
    fn from(p: PresetGeometryDto) -> Self {
        PresetGeometry {
            spt: p.spt,
            adjust: p.adjust,
        }
    }
}

/// DTO for [`AutoShape`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoShapeDto {
    /// Geometry.
    pub geometry: ShapeGeometryDto,
    /// Office preset, if any.
    pub preset: Option<PresetGeometryDto>,
}

impl From<AutoShape> for AutoShapeDto {
    fn from(a: AutoShape) -> Self {
        Self {
            geometry: a.geometry.into(),
            preset: a.preset.map(Into::into),
        }
    }
}

impl From<AutoShapeDto> for AutoShape {
    fn from(a: AutoShapeDto) -> Self {
        AutoShape {
            geometry: a.geometry.into(),
            preset: a.preset.map(Into::into),
        }
    }
}
