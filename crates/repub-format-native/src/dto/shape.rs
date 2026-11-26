//! Serde DTOs for shapes and their kind-specific payloads.

use repub_model::shape::{
    CellHAlign, CellVAlign, ColumnSpec, ConnectorType, Extrusion3D, GroupData, ImageFit,
    ImageSource, LineData, LineEnding, LineEndingStyle, Margins, PictureData, Rotation3D, Shape,
    ShapeCommon, ShapeKind, TableBorders, TableCell, TableData, TextEffectData, TextFrameData,
    TextOverflow, TextPathAlign, TextPathSpacing, TextTransform, TextFitMode, VerticalAlignment,
    WordArtAlign, WordArtPreset, BevelType,
};
use repub_model::id::ObjectId;
use serde::{Deserialize, Serialize};

use super::geometry::{ColorDto, PointDto, RectDto, TransformDto};
use super::shape_geometry::{AutoShapeDto, ModelPathDto};
use super::styles::{
    CharacterStyleDto, EffectsDto, FillDto, ParagraphStyleDto, StrokeDto,
};
use super::text::StyledTextDto;
use super::text_wrap::TextWrapDto;

/// DTO for [`ShapeCommon`].
///
/// The object `id` is stored as its raw numeric value and re-registered on
/// load via [`ObjectId::from_raw`] so later IDs never collide with it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeCommonDto {
    /// Raw object id.
    pub id: u64,
    /// Bounds.
    pub bounds: RectDto,
    /// Rotation in degrees.
    pub rotation: f64,
    /// Extra transform.
    pub transform: TransformDto,
    /// Visibility.
    pub visible: bool,
    /// Locked flag.
    pub locked: bool,
    /// Optional name.
    pub name: Option<String>,
    /// Z-index.
    pub z_index: i32,
    /// Text wrap settings.
    pub text_wrap: TextWrapDto,
    /// Horizontal flip.
    pub flip_h: bool,
    /// Vertical flip.
    pub flip_v: bool,
}

impl From<ShapeCommon> for ShapeCommonDto {
    fn from(c: ShapeCommon) -> Self {
        Self {
            id: c.id.raw(),
            bounds: c.bounds.into(),
            rotation: c.rotation,
            transform: c.transform.into(),
            visible: c.visible,
            locked: c.locked,
            name: c.name,
            z_index: c.z_index,
            text_wrap: c.text_wrap.into(),
            flip_h: c.flip_h,
            flip_v: c.flip_v,
        }
    }
}

impl From<ShapeCommonDto> for ShapeCommon {
    fn from(c: ShapeCommonDto) -> Self {
        ShapeCommon {
            id: ObjectId::from_raw(c.id),
            bounds: c.bounds.into(),
            rotation: c.rotation,
            transform: c.transform.into(),
            visible: c.visible,
            locked: c.locked,
            name: c.name,
            z_index: c.z_index,
            text_wrap: c.text_wrap.into(),
            flip_h: c.flip_h,
            flip_v: c.flip_v,
        }
    }
}

/// DTO for [`ColumnSpec`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnSpecDto {
    /// Column count.
    pub count: u32,
    /// Gap.
    pub gap: f64,
    /// Equal width flag.
    pub equal_width: bool,
    /// Custom widths.
    pub widths: Vec<f64>,
}

impl From<ColumnSpec> for ColumnSpecDto {
    fn from(c: ColumnSpec) -> Self {
        Self {
            count: c.count,
            gap: c.gap,
            equal_width: c.equal_width,
            widths: c.widths,
        }
    }
}

impl From<ColumnSpecDto> for ColumnSpec {
    fn from(c: ColumnSpecDto) -> Self {
        ColumnSpec {
            count: c.count,
            gap: c.gap,
            equal_width: c.equal_width,
            widths: c.widths,
        }
    }
}

/// DTO for [`TextOverflow`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum TextOverflowDto {
    Clip,
    ShowIndicator,
    AutoResize,
    FlowToLinked,
}

impl From<TextOverflow> for TextOverflowDto {
    fn from(o: TextOverflow) -> Self {
        match o {
            TextOverflow::Clip => TextOverflowDto::Clip,
            TextOverflow::ShowIndicator => TextOverflowDto::ShowIndicator,
            TextOverflow::AutoResize => TextOverflowDto::AutoResize,
            TextOverflow::FlowToLinked => TextOverflowDto::FlowToLinked,
        }
    }
}

impl From<TextOverflowDto> for TextOverflow {
    fn from(o: TextOverflowDto) -> Self {
        match o {
            TextOverflowDto::Clip => TextOverflow::Clip,
            TextOverflowDto::ShowIndicator => TextOverflow::ShowIndicator,
            TextOverflowDto::AutoResize => TextOverflow::AutoResize,
            TextOverflowDto::FlowToLinked => TextOverflow::FlowToLinked,
        }
    }
}

/// DTO for [`VerticalAlignment`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum VerticalAlignmentDto {
    Top,
    Center,
    Bottom,
}

impl From<VerticalAlignment> for VerticalAlignmentDto {
    fn from(v: VerticalAlignment) -> Self {
        match v {
            VerticalAlignment::Top => VerticalAlignmentDto::Top,
            VerticalAlignment::Center => VerticalAlignmentDto::Center,
            VerticalAlignment::Bottom => VerticalAlignmentDto::Bottom,
        }
    }
}

impl From<VerticalAlignmentDto> for VerticalAlignment {
    fn from(v: VerticalAlignmentDto) -> Self {
        match v {
            VerticalAlignmentDto::Top => VerticalAlignment::Top,
            VerticalAlignmentDto::Center => VerticalAlignment::Center,
            VerticalAlignmentDto::Bottom => VerticalAlignment::Bottom,
        }
    }
}

/// DTO for [`TextFitMode`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum TextFitModeDto {
    None,
    ShrinkOnOverflow,
    GrowToFit,
    BestFit,
}

impl From<TextFitMode> for TextFitModeDto {
    fn from(t: TextFitMode) -> Self {
        match t {
            TextFitMode::None => TextFitModeDto::None,
            TextFitMode::ShrinkOnOverflow => TextFitModeDto::ShrinkOnOverflow,
            TextFitMode::GrowToFit => TextFitModeDto::GrowToFit,
            TextFitMode::BestFit => TextFitModeDto::BestFit,
        }
    }
}

impl From<TextFitModeDto> for TextFitMode {
    fn from(t: TextFitModeDto) -> Self {
        match t {
            TextFitModeDto::None => TextFitMode::None,
            TextFitModeDto::ShrinkOnOverflow => TextFitMode::ShrinkOnOverflow,
            TextFitModeDto::GrowToFit => TextFitMode::GrowToFit,
            TextFitModeDto::BestFit => TextFitMode::BestFit,
        }
    }
}

/// DTO for shape [`Margins`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct MarginsDto {
    /// Top.
    pub top: f64,
    /// Bottom.
    pub bottom: f64,
    /// Left.
    pub left: f64,
    /// Right.
    pub right: f64,
}

impl From<Margins> for MarginsDto {
    fn from(m: Margins) -> Self {
        Self {
            top: m.top,
            bottom: m.bottom,
            left: m.left,
            right: m.right,
        }
    }
}

impl From<MarginsDto> for Margins {
    fn from(m: MarginsDto) -> Self {
        Margins {
            top: m.top,
            bottom: m.bottom,
            left: m.left,
            right: m.right,
        }
    }
}

/// DTO for [`TextFrameData`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextFrameDataDto {
    /// Content.
    pub content: StyledTextDto,
    /// Inset.
    pub inset: f64,
    /// Columns.
    pub columns: ColumnSpecDto,
    /// Overflow.
    pub overflow: TextOverflowDto,
    /// Vertical alignment.
    pub vertical_align: VerticalAlignmentDto,
    /// Linked frame (raw object id).
    pub linked_frame: Option<u64>,
    /// Continuation flag.
    pub is_continuation: bool,
    /// Text fit mode.
    pub text_fit: TextFitModeDto,
    /// Margins.
    pub margins: MarginsDto,
}

impl From<TextFrameData> for TextFrameDataDto {
    fn from(d: TextFrameData) -> Self {
        Self {
            content: d.content.into(),
            inset: d.inset,
            columns: d.columns.into(),
            overflow: d.overflow.into(),
            vertical_align: d.vertical_align.into(),
            linked_frame: d.linked_frame.map(|id| id.raw()),
            is_continuation: d.is_continuation,
            text_fit: d.text_fit.into(),
            margins: d.margins.into(),
        }
    }
}

impl From<TextFrameDataDto> for TextFrameData {
    fn from(d: TextFrameDataDto) -> Self {
        TextFrameData {
            content: d.content.into(),
            inset: d.inset,
            columns: d.columns.into(),
            overflow: d.overflow.into(),
            vertical_align: d.vertical_align.into(),
            linked_frame: d.linked_frame.map(ObjectId::from_raw),
            is_continuation: d.is_continuation,
            text_fit: d.text_fit.into(),
            margins: d.margins.into(),
        }
    }
}

/// DTO for [`ImageSource`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum ImageSourceDto {
    File(String),
    Url(String),
    Embedded { data: String, mime_type: String },
    Placeholder,
}

impl From<ImageSource> for ImageSourceDto {
    fn from(s: ImageSource) -> Self {
        match s {
            ImageSource::File(p) => ImageSourceDto::File(p),
            ImageSource::Url(u) => ImageSourceDto::Url(u),
            ImageSource::Embedded { data, mime_type } => {
                ImageSourceDto::Embedded { data, mime_type }
            }
            ImageSource::Placeholder => ImageSourceDto::Placeholder,
        }
    }
}

impl From<ImageSourceDto> for ImageSource {
    fn from(s: ImageSourceDto) -> Self {
        match s {
            ImageSourceDto::File(p) => ImageSource::File(p),
            ImageSourceDto::Url(u) => ImageSource::Url(u),
            ImageSourceDto::Embedded { data, mime_type } => {
                ImageSource::Embedded { data, mime_type }
            }
            ImageSourceDto::Placeholder => ImageSource::Placeholder,
        }
    }
}

/// DTO for [`ImageFit`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum ImageFitDto {
    Fill,
    Fit,
    Stretch,
    Tile,
    Center,
}

impl From<ImageFit> for ImageFitDto {
    fn from(f: ImageFit) -> Self {
        match f {
            ImageFit::Fill => ImageFitDto::Fill,
            ImageFit::Fit => ImageFitDto::Fit,
            ImageFit::Stretch => ImageFitDto::Stretch,
            ImageFit::Tile => ImageFitDto::Tile,
            ImageFit::Center => ImageFitDto::Center,
        }
    }
}

impl From<ImageFitDto> for ImageFit {
    fn from(f: ImageFitDto) -> Self {
        match f {
            ImageFitDto::Fill => ImageFit::Fill,
            ImageFitDto::Fit => ImageFit::Fit,
            ImageFitDto::Stretch => ImageFit::Stretch,
            ImageFitDto::Tile => ImageFit::Tile,
            ImageFitDto::Center => ImageFit::Center,
        }
    }
}

/// DTO for [`PictureData`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PictureDataDto {
    /// Source.
    pub source: ImageSourceDto,
    /// Fit.
    pub fit: ImageFitDto,
    /// Crop rectangle.
    pub crop: Option<RectDto>,
    /// Brightness.
    pub brightness: f64,
    /// Contrast.
    pub contrast: f64,
    /// Alt text.
    pub alt_text: Option<String>,
}

impl From<PictureData> for PictureDataDto {
    fn from(d: PictureData) -> Self {
        Self {
            source: d.source.into(),
            fit: d.fit.into(),
            crop: d.crop.map(Into::into),
            brightness: d.brightness,
            contrast: d.contrast,
            alt_text: d.alt_text,
        }
    }
}

impl From<PictureDataDto> for PictureData {
    fn from(d: PictureDataDto) -> Self {
        PictureData {
            source: d.source.into(),
            fit: d.fit.into(),
            crop: d.crop.map(Into::into),
            brightness: d.brightness,
            contrast: d.contrast,
            alt_text: d.alt_text,
        }
    }
}

/// DTO for [`CellHAlign`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum CellHAlignDto {
    Left,
    Center,
    Right,
}

impl From<CellHAlign> for CellHAlignDto {
    fn from(a: CellHAlign) -> Self {
        match a {
            CellHAlign::Left => CellHAlignDto::Left,
            CellHAlign::Center => CellHAlignDto::Center,
            CellHAlign::Right => CellHAlignDto::Right,
        }
    }
}

impl From<CellHAlignDto> for CellHAlign {
    fn from(a: CellHAlignDto) -> Self {
        match a {
            CellHAlignDto::Left => CellHAlign::Left,
            CellHAlignDto::Center => CellHAlign::Center,
            CellHAlignDto::Right => CellHAlign::Right,
        }
    }
}

/// DTO for [`CellVAlign`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum CellVAlignDto {
    Top,
    Middle,
    Bottom,
}

impl From<CellVAlign> for CellVAlignDto {
    fn from(a: CellVAlign) -> Self {
        match a {
            CellVAlign::Top => CellVAlignDto::Top,
            CellVAlign::Middle => CellVAlignDto::Middle,
            CellVAlign::Bottom => CellVAlignDto::Bottom,
        }
    }
}

impl From<CellVAlignDto> for CellVAlign {
    fn from(a: CellVAlignDto) -> Self {
        match a {
            CellVAlignDto::Top => CellVAlign::Top,
            CellVAlignDto::Middle => CellVAlign::Middle,
            CellVAlignDto::Bottom => CellVAlign::Bottom,
        }
    }
}

/// DTO for [`TableCell`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableCellDto {
    /// Content.
    pub content: StyledTextDto,
    /// Fill.
    pub fill: FillDto,
    /// Padding.
    pub padding: f64,
    /// Horizontal alignment.
    pub h_align: CellHAlignDto,
    /// Vertical alignment.
    pub v_align: CellVAlignDto,
    /// Column span.
    pub col_span: u32,
    /// Row span.
    pub row_span: u32,
}

impl From<TableCell> for TableCellDto {
    fn from(c: TableCell) -> Self {
        Self {
            content: c.content.into(),
            fill: c.fill.into(),
            padding: c.padding,
            h_align: c.h_align.into(),
            v_align: c.v_align.into(),
            col_span: c.col_span,
            row_span: c.row_span,
        }
    }
}

impl From<TableCellDto> for TableCell {
    fn from(c: TableCellDto) -> Self {
        TableCell {
            content: c.content.into(),
            fill: c.fill.into(),
            padding: c.padding,
            h_align: c.h_align.into(),
            v_align: c.v_align.into(),
            col_span: c.col_span,
            row_span: c.row_span,
        }
    }
}

/// DTO for [`TableBorders`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableBordersDto {
    /// Top.
    pub top: Option<StrokeDto>,
    /// Right.
    pub right: Option<StrokeDto>,
    /// Bottom.
    pub bottom: Option<StrokeDto>,
    /// Left.
    pub left: Option<StrokeDto>,
    /// Inner horizontal.
    pub inner_horizontal: Option<StrokeDto>,
    /// Inner vertical.
    pub inner_vertical: Option<StrokeDto>,
}

impl From<TableBorders> for TableBordersDto {
    fn from(b: TableBorders) -> Self {
        Self {
            top: b.top.map(Into::into),
            right: b.right.map(Into::into),
            bottom: b.bottom.map(Into::into),
            left: b.left.map(Into::into),
            inner_horizontal: b.inner_horizontal.map(Into::into),
            inner_vertical: b.inner_vertical.map(Into::into),
        }
    }
}

impl From<TableBordersDto> for TableBorders {
    fn from(b: TableBordersDto) -> Self {
        TableBorders {
            top: b.top.map(Into::into),
            right: b.right.map(Into::into),
            bottom: b.bottom.map(Into::into),
            left: b.left.map(Into::into),
            inner_horizontal: b.inner_horizontal.map(Into::into),
            inner_vertical: b.inner_vertical.map(Into::into),
        }
    }
}

/// DTO for [`TableData`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDataDto {
    /// Rows.
    pub rows: u32,
    /// Columns.
    pub cols: u32,
    /// Row heights.
    pub row_heights: Vec<f64>,
    /// Column widths.
    pub col_widths: Vec<f64>,
    /// Cells.
    pub cells: Vec<TableCellDto>,
    /// Borders.
    pub borders: TableBordersDto,
    /// Default character style.
    pub default_char_style: CharacterStyleDto,
    /// Default paragraph style.
    pub default_para_style: ParagraphStyleDto,
}

impl From<TableData> for TableDataDto {
    fn from(d: TableData) -> Self {
        Self {
            rows: d.rows,
            cols: d.cols,
            row_heights: d.row_heights,
            col_widths: d.col_widths,
            cells: d.cells.into_iter().map(Into::into).collect(),
            borders: d.borders.into(),
            default_char_style: d.default_char_style.into(),
            default_para_style: d.default_para_style.into(),
        }
    }
}

impl From<TableDataDto> for TableData {
    fn from(d: TableDataDto) -> Self {
        TableData {
            rows: d.rows,
            cols: d.cols,
            row_heights: d.row_heights,
            col_widths: d.col_widths,
            cells: d.cells.into_iter().map(Into::into).collect(),
            borders: d.borders.into(),
            default_char_style: d.default_char_style.into(),
            default_para_style: d.default_para_style.into(),
        }
    }
}

/// DTO for [`LineEndingStyle`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum LineEndingStyleDto {
    None,
    Arrow,
    OpenArrow,
    StealthArrow,
    Diamond,
    Circle,
    Square,
}

impl From<LineEndingStyle> for LineEndingStyleDto {
    fn from(s: LineEndingStyle) -> Self {
        match s {
            LineEndingStyle::None => LineEndingStyleDto::None,
            LineEndingStyle::Arrow => LineEndingStyleDto::Arrow,
            LineEndingStyle::OpenArrow => LineEndingStyleDto::OpenArrow,
            LineEndingStyle::StealthArrow => LineEndingStyleDto::StealthArrow,
            LineEndingStyle::Diamond => LineEndingStyleDto::Diamond,
            LineEndingStyle::Circle => LineEndingStyleDto::Circle,
            LineEndingStyle::Square => LineEndingStyleDto::Square,
        }
    }
}

impl From<LineEndingStyleDto> for LineEndingStyle {
    fn from(s: LineEndingStyleDto) -> Self {
        match s {
            LineEndingStyleDto::None => LineEndingStyle::None,
            LineEndingStyleDto::Arrow => LineEndingStyle::Arrow,
            LineEndingStyleDto::OpenArrow => LineEndingStyle::OpenArrow,
            LineEndingStyleDto::StealthArrow => LineEndingStyle::StealthArrow,
            LineEndingStyleDto::Diamond => LineEndingStyle::Diamond,
            LineEndingStyleDto::Circle => LineEndingStyle::Circle,
            LineEndingStyleDto::Square => LineEndingStyle::Square,
        }
    }
}

/// DTO for [`LineEnding`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LineEndingDto {
    /// Style.
    pub style: LineEndingStyleDto,
    /// Size.
    pub size: f64,
    /// Filled flag.
    pub filled: bool,
}

impl From<LineEnding> for LineEndingDto {
    fn from(e: LineEnding) -> Self {
        Self {
            style: e.style.into(),
            size: e.size,
            filled: e.filled,
        }
    }
}

impl From<LineEndingDto> for LineEnding {
    fn from(e: LineEndingDto) -> Self {
        LineEnding {
            style: e.style.into(),
            size: e.size,
            filled: e.filled,
        }
    }
}

/// DTO for [`ConnectorType`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum ConnectorTypeDto {
    Straight,
    Elbow,
    Curved,
}

impl From<ConnectorType> for ConnectorTypeDto {
    fn from(c: ConnectorType) -> Self {
        match c {
            ConnectorType::Straight => ConnectorTypeDto::Straight,
            ConnectorType::Elbow => ConnectorTypeDto::Elbow,
            ConnectorType::Curved => ConnectorTypeDto::Curved,
        }
    }
}

impl From<ConnectorTypeDto> for ConnectorType {
    fn from(c: ConnectorTypeDto) -> Self {
        match c {
            ConnectorTypeDto::Straight => ConnectorType::Straight,
            ConnectorTypeDto::Elbow => ConnectorType::Elbow,
            ConnectorTypeDto::Curved => ConnectorType::Curved,
        }
    }
}

/// DTO for [`LineData`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LineDataDto {
    /// Start.
    pub start: PointDto,
    /// End.
    pub end: PointDto,
    /// Control points.
    pub control_points: Vec<PointDto>,
    /// Connector type.
    pub connector_type: ConnectorTypeDto,
    /// Start ending.
    pub start_ending: LineEndingDto,
    /// End ending.
    pub end_ending: LineEndingDto,
}

impl From<LineData> for LineDataDto {
    fn from(d: LineData) -> Self {
        Self {
            start: d.start.into(),
            end: d.end.into(),
            control_points: d.control_points.into_iter().map(Into::into).collect(),
            connector_type: d.connector_type.into(),
            start_ending: d.start_ending.into(),
            end_ending: d.end_ending.into(),
        }
    }
}

impl From<LineDataDto> for LineData {
    fn from(d: LineDataDto) -> Self {
        LineData {
            start: d.start.into(),
            end: d.end.into(),
            control_points: d.control_points.into_iter().map(Into::into).collect(),
            connector_type: d.connector_type.into(),
            start_ending: d.start_ending.into(),
            end_ending: d.end_ending.into(),
        }
    }
}

/// DTO for [`WordArtPreset`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum WordArtPresetDto {
    Plain,
    Outline,
    Shadow,
    ThreeD,
    ArchUp,
    ArchDown,
    Circle,
    Wave,
    Inflate,
    Deflate,
    SlantUp,
    SlantDown,
    FadeRight,
    FadeLeft,
    ChevronUp,
    ChevronDown,
    TriangleUp,
    TriangleDown,
    CanUp,
    CanDown,
    CascadeUp,
    CascadeDown,
    Stop,
    RingInside,
    RingOutside,
    Button,
}

impl From<WordArtPreset> for WordArtPresetDto {
    fn from(p: WordArtPreset) -> Self {
        use WordArtPreset as W;
        match p {
            W::Plain => WordArtPresetDto::Plain,
            W::Outline => WordArtPresetDto::Outline,
            W::Shadow => WordArtPresetDto::Shadow,
            W::ThreeD => WordArtPresetDto::ThreeD,
            W::ArchUp => WordArtPresetDto::ArchUp,
            W::ArchDown => WordArtPresetDto::ArchDown,
            W::Circle => WordArtPresetDto::Circle,
            W::Wave => WordArtPresetDto::Wave,
            W::Inflate => WordArtPresetDto::Inflate,
            W::Deflate => WordArtPresetDto::Deflate,
            W::SlantUp => WordArtPresetDto::SlantUp,
            W::SlantDown => WordArtPresetDto::SlantDown,
            W::FadeRight => WordArtPresetDto::FadeRight,
            W::FadeLeft => WordArtPresetDto::FadeLeft,
            W::ChevronUp => WordArtPresetDto::ChevronUp,
            W::ChevronDown => WordArtPresetDto::ChevronDown,
            W::TriangleUp => WordArtPresetDto::TriangleUp,
            W::TriangleDown => WordArtPresetDto::TriangleDown,
            W::CanUp => WordArtPresetDto::CanUp,
            W::CanDown => WordArtPresetDto::CanDown,
            W::CascadeUp => WordArtPresetDto::CascadeUp,
            W::CascadeDown => WordArtPresetDto::CascadeDown,
            W::Stop => WordArtPresetDto::Stop,
            W::RingInside => WordArtPresetDto::RingInside,
            W::RingOutside => WordArtPresetDto::RingOutside,
            W::Button => WordArtPresetDto::Button,
        }
    }
}

impl From<WordArtPresetDto> for WordArtPreset {
    fn from(p: WordArtPresetDto) -> Self {
        use WordArtPresetDto as W;
        match p {
            W::Plain => WordArtPreset::Plain,
            W::Outline => WordArtPreset::Outline,
            W::Shadow => WordArtPreset::Shadow,
            W::ThreeD => WordArtPreset::ThreeD,
            W::ArchUp => WordArtPreset::ArchUp,
            W::ArchDown => WordArtPreset::ArchDown,
            W::Circle => WordArtPreset::Circle,
            W::Wave => WordArtPreset::Wave,
            W::Inflate => WordArtPreset::Inflate,
            W::Deflate => WordArtPreset::Deflate,
            W::SlantUp => WordArtPreset::SlantUp,
            W::SlantDown => WordArtPreset::SlantDown,
            W::FadeRight => WordArtPreset::FadeRight,
            W::FadeLeft => WordArtPreset::FadeLeft,
            W::ChevronUp => WordArtPreset::ChevronUp,
            W::ChevronDown => WordArtPreset::ChevronDown,
            W::TriangleUp => WordArtPreset::TriangleUp,
            W::TriangleDown => WordArtPreset::TriangleDown,
            W::CanUp => WordArtPreset::CanUp,
            W::CanDown => WordArtPreset::CanDown,
            W::CascadeUp => WordArtPreset::CascadeUp,
            W::CascadeDown => WordArtPreset::CascadeDown,
            W::Stop => WordArtPreset::Stop,
            W::RingInside => WordArtPreset::RingInside,
            W::RingOutside => WordArtPreset::RingOutside,
            W::Button => WordArtPreset::Button,
        }
    }
}

/// DTO for [`TextTransform`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum TextTransformDto {
    None,
    Preset(WordArtPresetDto),
    Path(ModelPathDto),
    Warp {
        top: Vec<PointDto>,
        bottom: Vec<PointDto>,
    },
}

impl From<TextTransform> for TextTransformDto {
    fn from(t: TextTransform) -> Self {
        match t {
            TextTransform::None => TextTransformDto::None,
            TextTransform::Preset(p) => TextTransformDto::Preset(p.into()),
            TextTransform::Path(p) => TextTransformDto::Path(p.into()),
            TextTransform::Warp { top, bottom } => TextTransformDto::Warp {
                top: top.into_iter().map(Into::into).collect(),
                bottom: bottom.into_iter().map(Into::into).collect(),
            },
        }
    }
}

impl From<TextTransformDto> for TextTransform {
    fn from(t: TextTransformDto) -> Self {
        match t {
            TextTransformDto::None => TextTransform::None,
            TextTransformDto::Preset(p) => TextTransform::Preset(p.into()),
            TextTransformDto::Path(p) => TextTransform::Path(p.into()),
            TextTransformDto::Warp { top, bottom } => TextTransform::Warp {
                top: top.into_iter().map(Into::into).collect(),
                bottom: bottom.into_iter().map(Into::into).collect(),
            },
        }
    }
}

/// DTO for [`Rotation3D`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rotation3DDto {
    /// X rotation in degrees.
    pub x: f64,
    /// Y rotation in degrees.
    pub y: f64,
    /// Z rotation in degrees.
    pub z: f64,
    /// Perspective.
    pub perspective: f64,
}

impl From<Rotation3D> for Rotation3DDto {
    fn from(r: Rotation3D) -> Self {
        Self {
            x: r.x,
            y: r.y,
            z: r.z,
            perspective: r.perspective,
        }
    }
}

impl From<Rotation3DDto> for Rotation3D {
    fn from(r: Rotation3DDto) -> Self {
        Rotation3D {
            x: r.x,
            y: r.y,
            z: r.z,
            perspective: r.perspective,
        }
    }
}

/// DTO for [`BevelType`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum BevelTypeDto {
    Flat,
    Circle,
    RelaxedInset,
    Cross,
    Convex,
    CoolSlant,
    Angle,
    SoftRound,
    ArtDeco,
    Slope,
    Divot,
    Riblet,
    HardEdge,
}

impl From<BevelType> for BevelTypeDto {
    fn from(b: BevelType) -> Self {
        use BevelType as B;
        match b {
            B::Flat => BevelTypeDto::Flat,
            B::Circle => BevelTypeDto::Circle,
            B::RelaxedInset => BevelTypeDto::RelaxedInset,
            B::Cross => BevelTypeDto::Cross,
            B::Convex => BevelTypeDto::Convex,
            B::CoolSlant => BevelTypeDto::CoolSlant,
            B::Angle => BevelTypeDto::Angle,
            B::SoftRound => BevelTypeDto::SoftRound,
            B::ArtDeco => BevelTypeDto::ArtDeco,
            B::Slope => BevelTypeDto::Slope,
            B::Divot => BevelTypeDto::Divot,
            B::Riblet => BevelTypeDto::Riblet,
            B::HardEdge => BevelTypeDto::HardEdge,
        }
    }
}

impl From<BevelTypeDto> for BevelType {
    fn from(b: BevelTypeDto) -> Self {
        use BevelTypeDto as B;
        match b {
            B::Flat => BevelType::Flat,
            B::Circle => BevelType::Circle,
            B::RelaxedInset => BevelType::RelaxedInset,
            B::Cross => BevelType::Cross,
            B::Convex => BevelType::Convex,
            B::CoolSlant => BevelType::CoolSlant,
            B::Angle => BevelType::Angle,
            B::SoftRound => BevelType::SoftRound,
            B::ArtDeco => BevelType::ArtDeco,
            B::Slope => BevelType::Slope,
            B::Divot => BevelType::Divot,
            B::Riblet => BevelType::Riblet,
            B::HardEdge => BevelType::HardEdge,
        }
    }
}

/// DTO for [`Extrusion3D`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extrusion3DDto {
    /// Depth.
    pub depth: f64,
    /// Color.
    pub color: Option<ColorDto>,
    /// Contour width.
    pub contour_width: f64,
    /// Bevel.
    pub bevel: Option<BevelTypeDto>,
    /// Bevel size.
    pub bevel_size: f64,
}

impl From<Extrusion3D> for Extrusion3DDto {
    fn from(e: Extrusion3D) -> Self {
        Self {
            depth: e.depth,
            color: e.color.map(Into::into),
            contour_width: e.contour_width,
            bevel: e.bevel.map(Into::into),
            bevel_size: e.bevel_size,
        }
    }
}

impl From<Extrusion3DDto> for Extrusion3D {
    fn from(e: Extrusion3DDto) -> Self {
        Extrusion3D {
            depth: e.depth,
            color: e.color.map(Into::into),
            contour_width: e.contour_width,
            bevel: e.bevel.map(Into::into),
            bevel_size: e.bevel_size,
        }
    }
}

/// DTO for [`WordArtAlign`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum WordArtAlignDto {
    Left,
    Center,
    Right,
    Stretch,
}

impl From<WordArtAlign> for WordArtAlignDto {
    fn from(a: WordArtAlign) -> Self {
        match a {
            WordArtAlign::Left => WordArtAlignDto::Left,
            WordArtAlign::Center => WordArtAlignDto::Center,
            WordArtAlign::Right => WordArtAlignDto::Right,
            WordArtAlign::Stretch => WordArtAlignDto::Stretch,
        }
    }
}

impl From<WordArtAlignDto> for WordArtAlign {
    fn from(a: WordArtAlignDto) -> Self {
        match a {
            WordArtAlignDto::Left => WordArtAlign::Left,
            WordArtAlignDto::Center => WordArtAlign::Center,
            WordArtAlignDto::Right => WordArtAlign::Right,
            WordArtAlignDto::Stretch => WordArtAlign::Stretch,
        }
    }
}

/// DTO for [`TextPathAlign`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum TextPathAlignDto {
    Left,
    Center,
    Right,
    Stretch,
}

impl From<TextPathAlign> for TextPathAlignDto {
    fn from(a: TextPathAlign) -> Self {
        match a {
            TextPathAlign::Left => TextPathAlignDto::Left,
            TextPathAlign::Center => TextPathAlignDto::Center,
            TextPathAlign::Right => TextPathAlignDto::Right,
            TextPathAlign::Stretch => TextPathAlignDto::Stretch,
        }
    }
}

impl From<TextPathAlignDto> for TextPathAlign {
    fn from(a: TextPathAlignDto) -> Self {
        match a {
            TextPathAlignDto::Left => TextPathAlign::Left,
            TextPathAlignDto::Center => TextPathAlign::Center,
            TextPathAlignDto::Right => TextPathAlign::Right,
            TextPathAlignDto::Stretch => TextPathAlign::Stretch,
        }
    }
}

/// DTO for [`TextPathSpacing`].
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(missing_docs)]
pub enum TextPathSpacingDto {
    Exact,
    Auto,
}

impl From<TextPathSpacing> for TextPathSpacingDto {
    fn from(s: TextPathSpacing) -> Self {
        match s {
            TextPathSpacing::Exact => TextPathSpacingDto::Exact,
            TextPathSpacing::Auto => TextPathSpacingDto::Auto,
        }
    }
}

impl From<TextPathSpacingDto> for TextPathSpacing {
    fn from(s: TextPathSpacingDto) -> Self {
        match s {
            TextPathSpacingDto::Exact => TextPathSpacing::Exact,
            TextPathSpacingDto::Auto => TextPathSpacing::Auto,
        }
    }
}

/// DTO for [`TextEffectData`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEffectDataDto {
    /// Text.
    pub text: String,
    /// Character style.
    pub style: CharacterStyleDto,
    /// Transform.
    pub transform: TextTransformDto,
    /// 3D rotation.
    pub rotation_3d: Rotation3DDto,
    /// Extrusion.
    pub extrusion: Option<Extrusion3DDto>,
    /// Letter spacing.
    pub letter_spacing: f64,
    /// Line spacing.
    pub line_spacing: f64,
    /// Alignment.
    pub align: WordArtAlignDto,
    /// Same letter heights flag.
    pub same_letter_heights: bool,
    /// Vertical flag.
    pub vertical: bool,
    /// Path start offset.
    pub path_start_offset: f64,
    /// Path alignment.
    pub path_align: TextPathAlignDto,
    /// Path spacing.
    pub path_spacing: TextPathSpacingDto,
    /// Path flip.
    pub path_flip: bool,
}

impl From<TextEffectData> for TextEffectDataDto {
    fn from(d: TextEffectData) -> Self {
        Self {
            text: d.text,
            style: d.style.into(),
            transform: d.transform.into(),
            rotation_3d: d.rotation_3d.into(),
            extrusion: d.extrusion.map(Into::into),
            letter_spacing: d.letter_spacing,
            line_spacing: d.line_spacing,
            align: d.align.into(),
            same_letter_heights: d.same_letter_heights,
            vertical: d.vertical,
            path_start_offset: d.path_start_offset,
            path_align: d.path_align.into(),
            path_spacing: d.path_spacing.into(),
            path_flip: d.path_flip,
        }
    }
}

impl From<TextEffectDataDto> for TextEffectData {
    fn from(d: TextEffectDataDto) -> Self {
        TextEffectData {
            text: d.text,
            style: d.style.into(),
            transform: d.transform.into(),
            rotation_3d: d.rotation_3d.into(),
            extrusion: d.extrusion.map(Into::into),
            letter_spacing: d.letter_spacing,
            line_spacing: d.line_spacing,
            align: d.align.into(),
            same_letter_heights: d.same_letter_heights,
            vertical: d.vertical,
            path_start_offset: d.path_start_offset,
            path_align: d.path_align.into(),
            path_spacing: d.path_spacing.into(),
            path_flip: d.path_flip,
        }
    }
}

/// DTO for [`GroupData`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupDataDto {
    /// Children.
    pub children: Vec<ShapeDto>,
    /// Bounds.
    pub bounds: RectDto,
}

impl From<GroupData> for GroupDataDto {
    fn from(g: GroupData) -> Self {
        Self {
            children: g.children.into_iter().map(Into::into).collect(),
            bounds: g.bounds.into(),
        }
    }
}

impl From<GroupDataDto> for GroupData {
    fn from(g: GroupDataDto) -> Self {
        GroupData {
            children: g.children.into_iter().map(Into::into).collect(),
            bounds: g.bounds.into(),
        }
    }
}

/// DTO for [`ShapeKind`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(missing_docs)]
#[allow(clippy::large_enum_variant)]
pub enum ShapeKindDto {
    AutoShape(AutoShapeDto),
    TextFrame(TextFrameDataDto),
    Table(TableDataDto),
    Picture(PictureDataDto),
    TextEffect(TextEffectDataDto),
    Line(LineDataDto),
    Group(GroupDataDto),
}

impl From<ShapeKind> for ShapeKindDto {
    fn from(k: ShapeKind) -> Self {
        match k {
            ShapeKind::AutoShape(a) => ShapeKindDto::AutoShape(a.into()),
            ShapeKind::TextFrame(d) => ShapeKindDto::TextFrame(d.into()),
            ShapeKind::Table(d) => ShapeKindDto::Table(d.into()),
            ShapeKind::Picture(d) => ShapeKindDto::Picture(d.into()),
            ShapeKind::TextEffect(d) => ShapeKindDto::TextEffect(d.into()),
            ShapeKind::Line(d) => ShapeKindDto::Line(d.into()),
            ShapeKind::Group(d) => ShapeKindDto::Group(d.into()),
        }
    }
}

impl From<ShapeKindDto> for ShapeKind {
    fn from(k: ShapeKindDto) -> Self {
        match k {
            ShapeKindDto::AutoShape(a) => ShapeKind::AutoShape(a.into()),
            ShapeKindDto::TextFrame(d) => ShapeKind::TextFrame(d.into()),
            ShapeKindDto::Table(d) => ShapeKind::Table(d.into()),
            ShapeKindDto::Picture(d) => ShapeKind::Picture(d.into()),
            ShapeKindDto::TextEffect(d) => ShapeKind::TextEffect(d.into()),
            ShapeKindDto::Line(d) => ShapeKind::Line(d.into()),
            ShapeKindDto::Group(d) => ShapeKind::Group(d.into()),
        }
    }
}

/// DTO for [`Shape`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShapeDto {
    /// Common placement/bookkeeping.
    pub common: ShapeCommonDto,
    /// Fill.
    pub fill: FillDto,
    /// Line.
    pub line: Option<StrokeDto>,
    /// Effects.
    pub effects: EffectsDto,
    /// Opacity.
    pub opacity: f64,
    /// Kind.
    pub kind: ShapeKindDto,
}

impl From<Shape> for ShapeDto {
    fn from(s: Shape) -> Self {
        Self {
            common: s.common.into(),
            fill: s.fill.into(),
            line: s.line.map(Into::into),
            effects: s.effects.into(),
            opacity: s.opacity,
            kind: s.kind.into(),
        }
    }
}

impl From<ShapeDto> for Shape {
    fn from(s: ShapeDto) -> Self {
        Shape {
            common: s.common.into(),
            fill: s.fill.into(),
            line: s.line.map(Into::into),
            effects: s.effects.into(),
            opacity: s.opacity,
            kind: s.kind.into(),
        }
    }
}
