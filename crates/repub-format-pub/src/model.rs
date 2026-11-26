//! Compatibility object model for the ported Publisher importer and exporter.
//!
//! The Publisher import/export logic was written against the old `repub_core`
//! object model, where every page object was a variant of an [`Object`] enum
//! (`TextFrame`, `ImageFrame`, `Shape`, `Line`, `WordArt`, `TextOnPath`,
//! `Table`, `Group`), each carrying its own `common` placement and a
//! `ShapeStyle`. The new [`repub_model`] uses a single unified
//! [`repub_model::shape::Shape`] with a [`repub_model::shape::ShapeKind`]
//! payload.
//!
//! This module reproduces the old object types so the bulk of the ported
//! import/export code can stay unchanged, and bridges them to the new model
//! with [`Object::into_shape`] (import side) and [`Object::from_shape`]
//! (export side). The style, text, colour and geometry layers are identical
//! between the two models, so those types are re-used directly from
//! [`repub_model`].
//!
//! This is an internal translation layer, not a public API.
#![allow(missing_docs)]

use repub_model::geometry::{AutoShape, ShapeGeometry};
use repub_model::shape::{
    GroupData, ImageFit as NewImageFit, ImageSource as NewImageSource, LineData,
    PictureData, ShapeCommon, ShapeKind, TableData, TextEffectData, TextFrameData, TextTransform as NewTextTransform,
    WordArtAlign as NewWordArtAlign, WordArtPreset as NewWordArtPreset,
};
use repub_model::styles::{
    Bevel, DropShadow, Effects, Fill, Glow, Reflection, SoftEdges, Stroke, TextWrap,
};
use repub_model::text::StyledText;
use repub_model::{Color, Point, Rect, Transform2D};

pub use repub_model::geometry::{Path, PathCommand, SubpathStyle};
pub use repub_model::shape::{
    CellHAlign, CellVAlign, ColumnSpec, ConnectorType, Extrusion3D, ImageFit, ImageSource,
    LineEnding, LineEndingStyle, Margins, Rotation3D, TableBorders, TableCell, TextFitMode,
    TextOverflow, TextPathAlign, TextPathSpacing, VerticalAlignment, WordArtAlign, WordArtPreset,
};
pub use repub_model::styles::{CharacterStyle, ParagraphStyle};

pub use repub_model::geometry::PresetGeometry;

/// Placement and bookkeeping shared by all compat objects (old `ObjectCommon`).
#[derive(Debug, Clone, PartialEq)]
pub struct ObjectCommon {
    pub id: repub_model::ObjectId,
    pub bounds: Rect,
    pub rotation: f64,
    pub transform: Transform2D,
    pub visible: bool,
    pub locked: bool,
    pub name: Option<String>,
    pub z_index: i32,
    pub text_wrap: TextWrap,
    pub flip_h: bool,
    pub flip_v: bool,
}

impl ObjectCommon {
    pub fn new(bounds: Rect) -> Self {
        Self {
            id: repub_model::ObjectId::new(),
            bounds,
            rotation: 0.0,
            transform: Transform2D::IDENTITY,
            visible: true,
            locked: false,
            name: None,
            z_index: 0,
            text_wrap: TextWrap::default(),
            flip_h: false,
            flip_v: false,
        }
    }

    fn apply_to(&self, common: &mut ShapeCommon) {
        common.id = self.id;
        common.bounds = self.bounds;
        common.rotation = self.rotation;
        common.transform = self.transform;
        common.visible = self.visible;
        common.locked = self.locked;
        common.name = self.name.clone();
        common.z_index = self.z_index;
        common.text_wrap = self.text_wrap;
        common.flip_h = self.flip_h;
        common.flip_v = self.flip_v;
    }

    fn from_shape(common: &ShapeCommon) -> Self {
        Self {
            id: common.id,
            bounds: common.bounds,
            rotation: common.rotation,
            transform: common.transform,
            visible: common.visible,
            locked: common.locked,
            name: common.name.clone(),
            z_index: common.z_index,
            text_wrap: common.text_wrap,
            flip_h: common.flip_h,
            flip_v: common.flip_v,
        }
    }
}

impl Default for ObjectCommon {
    fn default() -> Self {
        Self::new(Rect::default())
    }
}

/// Combined shape styling (old `ShapeStyle`): fill, stroke and effects.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeStyle {
    pub fill: Fill,
    pub stroke: Option<Stroke>,
    pub shadow: Option<DropShadow>,
    pub glow: Option<Glow>,
    pub reflection: Option<Reflection>,
    pub bevel: Option<Bevel>,
    pub soft_edges: Option<SoftEdges>,
    pub opacity: f64,
}

impl Default for ShapeStyle {
    fn default() -> Self {
        Self {
            fill: Fill::default(),
            stroke: None,
            shadow: None,
            glow: None,
            reflection: None,
            bevel: None,
            soft_edges: None,
            opacity: 1.0,
        }
    }
}

impl ShapeStyle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Build the new-model [`Effects`] from these per-field effects.
    fn to_effects(&self) -> Effects {
        Effects {
            drop_shadow: self.shadow.clone(),
            inner_shadow: None,
            outer_glow: self.glow.clone(),
            inner_glow: None,
            bevel: self.bevel.clone(),
            reflection: self.reflection.clone(),
            soft_edges: self.soft_edges,
        }
    }

    /// Reconstruct a [`ShapeStyle`] from a new-model shape's parts.
    fn from_parts(fill: Fill, stroke: Option<Stroke>, effects: &Effects, opacity: f64) -> Self {
        Self {
            fill,
            stroke,
            shadow: effects.drop_shadow.clone(),
            glow: effects.outer_glow.clone(),
            reflection: effects.reflection.clone(),
            bevel: effects.bevel.clone(),
            soft_edges: effects.soft_edges,
            opacity,
        }
    }
}

/// The type of a vector [`Shape`] (old `ShapeType`).
#[derive(Debug, Clone, PartialEq)]
pub enum ShapeType {
    Rectangle { corner_radius: f64 },
    Ellipse,
    Polygon { sides: u32 },
    Star { points: u32, inner_ratio: f64 },
    Path(Path),
}

impl Default for ShapeType {
    fn default() -> Self {
        ShapeType::Rectangle { corner_radius: 0.0 }
    }
}

impl From<ShapeType> for ShapeGeometry {
    fn from(st: ShapeType) -> ShapeGeometry {
        match st {
            ShapeType::Rectangle { corner_radius } => ShapeGeometry::Rectangle { corner_radius },
            ShapeType::Ellipse => ShapeGeometry::Ellipse,
            ShapeType::Polygon { sides } => ShapeGeometry::Polygon { sides },
            ShapeType::Star { points, inner_ratio } => ShapeGeometry::Star { points, inner_ratio },
            ShapeType::Path(p) => ShapeGeometry::Path(p),
        }
    }
}

impl From<ShapeGeometry> for ShapeType {
    fn from(g: ShapeGeometry) -> ShapeType {
        match g {
            ShapeGeometry::Rectangle { corner_radius } => ShapeType::Rectangle { corner_radius },
            ShapeGeometry::Ellipse => ShapeType::Ellipse,
            ShapeGeometry::Polygon { sides } => ShapeType::Polygon { sides },
            ShapeGeometry::Star { points, inner_ratio } => ShapeType::Star { points, inner_ratio },
            ShapeGeometry::Path(p) => ShapeType::Path(p),
        }
    }
}

/// A vector shape (old `objects::Shape`).
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    pub common: ObjectCommon,
    pub shape_type: ShapeType,
    pub style: ShapeStyle,
    pub preset: Option<PresetGeometry>,
}

impl Shape {
    pub fn new(bounds: Rect, shape_type: ShapeType) -> Self {
        Self {
            common: ObjectCommon::new(bounds),
            shape_type,
            style: ShapeStyle::new(),
            preset: None,
        }
    }

    pub fn rectangle(bounds: Rect) -> Self {
        Self::new(bounds, ShapeType::Rectangle { corner_radius: 0.0 })
    }

    pub fn ellipse(bounds: Rect) -> Self {
        Self::new(bounds, ShapeType::Ellipse)
    }

    pub fn from_path(bounds: Rect, path: Path) -> Self {
        Self::new(bounds, ShapeType::Path(path))
    }
}

/// A text frame (old `objects::TextFrame`).
#[derive(Debug, Clone, PartialEq)]
pub struct TextFrame {
    pub common: ObjectCommon,
    pub content: StyledText,
    pub style: ShapeStyle,
    pub inset: f64,
    pub columns: ColumnSpec,
    pub overflow: TextOverflow,
    pub vertical_align: VerticalAlignment,
    pub linked_frame: Option<repub_model::ObjectId>,
    pub is_continuation: bool,
    pub text_fit: TextFitMode,
    pub margins: Margins,
}

impl TextFrame {
    pub fn new(bounds: Rect) -> Self {
        Self {
            common: ObjectCommon::new(bounds),
            content: StyledText::new(),
            style: ShapeStyle::new(),
            inset: 1.0,
            columns: ColumnSpec::single(),
            overflow: TextOverflow::Clip,
            vertical_align: VerticalAlignment::Top,
            linked_frame: None,
            is_continuation: false,
            text_fit: TextFitMode::None,
            margins: Margins::zero(),
        }
    }

    pub fn with_text(bounds: Rect, text: &str) -> Self {
        let mut frame = Self::new(bounds);
        frame.content = StyledText::plain(text);
        frame
    }

    pub fn set_text(&mut self, text: &str) {
        self.content = StyledText::plain(text);
    }

    pub fn plain_text(&self) -> String {
        self.content.plain_text()
    }
}

/// An image frame (old `objects::ImageFrame`); maps to the new picture shape.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageFrame {
    pub common: ObjectCommon,
    pub source: ImageSource,
    pub fit: ImageFit,
    pub style: ShapeStyle,
    pub crop: Option<Rect>,
    pub brightness: f64,
    pub contrast: f64,
    pub opacity: f64,
    pub alt_text: Option<String>,
}

impl ImageFrame {
    pub fn new(bounds: Rect) -> Self {
        Self {
            common: ObjectCommon::new(bounds),
            source: ImageSource::Placeholder,
            fit: ImageFit::Stretch,
            style: ShapeStyle::new(),
            crop: None,
            brightness: 0.0,
            contrast: 0.0,
            opacity: 1.0,
            alt_text: None,
        }
    }
}

/// A line or connector (old `objects::Line`).
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub common: ObjectCommon,
    pub start: Point,
    pub end: Point,
    pub control_points: Vec<Point>,
    pub connector_type: ConnectorType,
    pub stroke: Stroke,
    pub start_ending: LineEnding,
    pub end_ending: LineEnding,
}

impl Line {
    pub fn new(start: Point, end: Point) -> Self {
        let min_x = start.x.min(end.x);
        let min_y = start.y.min(end.y);
        let max_x = start.x.max(end.x);
        let max_y = start.y.max(end.y);
        let bounds = Rect::new(min_x, min_y, max_x - min_x, max_y - min_y);
        Self {
            common: ObjectCommon::new(bounds),
            start,
            end,
            control_points: Vec::new(),
            connector_type: ConnectorType::Straight,
            stroke: Stroke::default(),
            start_ending: LineEnding::default(),
            end_ending: LineEnding::default(),
        }
    }
}

/// Text transformation for WordArt (old `objects::TextTransform`).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TextTransform {
    #[default]
    None,
    Preset(WordArtPreset),
    Path(Path),
    Warp { top: Vec<Point>, bottom: Vec<Point> },
}

/// A WordArt text effect (old `objects::WordArt`); maps to a text-effect shape.
#[derive(Debug, Clone, PartialEq)]
pub struct WordArt {
    pub common: ObjectCommon,
    pub text: String,
    pub style: CharacterStyle,
    pub fill: Fill,
    pub outline: Option<Stroke>,
    pub transform: TextTransform,
    pub rotation_3d: Rotation3D,
    pub extrusion: Option<Extrusion3D>,
    pub letter_spacing: f64,
    pub line_spacing: f64,
    pub align: WordArtAlign,
    pub same_letter_heights: bool,
    pub vertical: bool,
}

impl WordArt {
    pub fn new(bounds: Rect, text: impl Into<String>) -> Self {
        Self {
            common: ObjectCommon::new(bounds),
            text: text.into(),
            style: CharacterStyle::default(),
            fill: Fill::Solid(Color::BLACK),
            outline: None,
            transform: TextTransform::None,
            rotation_3d: Rotation3D::default(),
            extrusion: None,
            letter_spacing: 0.0,
            line_spacing: 1.0,
            align: WordArtAlign::Center,
            same_letter_heights: false,
            vertical: false,
        }
    }
}

/// Text on a path (old `objects::TextOnPath`); maps to a text-effect shape.
#[derive(Debug, Clone, PartialEq)]
pub struct TextOnPath {
    pub common: ObjectCommon,
    pub text: String,
    pub style: CharacterStyle,
    pub path: Path,
    pub start_offset: f64,
    pub align: TextPathAlign,
    pub spacing: TextPathSpacing,
    pub flip: bool,
}

impl TextOnPath {
    pub fn new(bounds: Rect, text: impl Into<String>, path: Path) -> Self {
        Self {
            common: ObjectCommon::new(bounds),
            text: text.into(),
            style: CharacterStyle::default(),
            path,
            start_offset: 0.0,
            align: TextPathAlign::Left,
            spacing: TextPathSpacing::Auto,
            flip: false,
        }
    }
}

/// A table (old `objects::Table`).
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub common: ObjectCommon,
    pub rows: u32,
    pub cols: u32,
    pub row_heights: Vec<f64>,
    pub col_widths: Vec<f64>,
    pub cells: Vec<TableCell>,
    pub borders: TableBorders,
    pub default_char_style: CharacterStyle,
    pub default_para_style: ParagraphStyle,
}

impl Table {
    pub fn new(bounds: Rect, rows: u32, cols: u32) -> Self {
        let row_height = bounds.height() / rows.max(1) as f64;
        let col_width = bounds.width() / cols.max(1) as f64;
        let cells: Vec<TableCell> = (0..(rows * cols)).map(|_| TableCell::new()).collect();
        Self {
            common: ObjectCommon::new(bounds),
            rows,
            cols,
            row_heights: vec![row_height; rows as usize],
            col_widths: vec![col_width; cols as usize],
            cells,
            borders: TableBorders::default(),
            default_char_style: CharacterStyle::default(),
            default_para_style: ParagraphStyle::default(),
        }
    }

    pub fn cell_mut(&mut self, row: u32, col: u32) -> Option<&mut TableCell> {
        if row < self.rows && col < self.cols {
            Some(&mut self.cells[(row * self.cols + col) as usize])
        } else {
            None
        }
    }

    pub fn set_cell_text(&mut self, row: u32, col: u32, text: &str) {
        if let Some(cell) = self.cell_mut(row, col) {
            cell.set_text(text);
        }
    }
}

/// A group of objects (old `objects::Group`).
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub common: ObjectCommon,
    pub children: Vec<Object>,
}

impl Group {
    pub fn new() -> Self {
        Self {
            common: ObjectCommon::new(Rect::default()),
            children: Vec::new(),
        }
    }

    pub fn from_objects(objects: Vec<Object>) -> Self {
        let mut group = Self::new();
        for obj in objects {
            group.children.push(obj);
        }
        group.recalculate_bounds();
        group
    }

    pub fn recalculate_bounds(&mut self) {
        if self.children.is_empty() {
            self.common.bounds = Rect::default();
            return;
        }
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        for child in &self.children {
            let b = child.bounds();
            min_x = min_x.min(b.x());
            min_y = min_y.min(b.y());
            max_x = max_x.max(b.right());
            max_y = max_y.max(b.bottom());
        }
        self.common.bounds = Rect::new(min_x, min_y, max_x - min_x, max_y - min_y);
    }
}

impl Default for Group {
    fn default() -> Self {
        Self::new()
    }
}

/// Any object that can be placed on a page (old `objects::Object`).
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum Object {
    TextFrame(TextFrame),
    ImageFrame(ImageFrame),
    Shape(Shape),
    Line(Line),
    Group(Group),
    Table(Table),
    WordArt(WordArt),
    TextOnPath(TextOnPath),
}

impl Object {
    pub fn id(&self) -> repub_model::ObjectId {
        self.common().id
    }

    pub fn common(&self) -> &ObjectCommon {
        match self {
            Object::TextFrame(o) => &o.common,
            Object::ImageFrame(o) => &o.common,
            Object::Shape(o) => &o.common,
            Object::Line(o) => &o.common,
            Object::Group(o) => &o.common,
            Object::Table(o) => &o.common,
            Object::WordArt(o) => &o.common,
            Object::TextOnPath(o) => &o.common,
        }
    }

    pub fn bounds(&self) -> Rect {
        self.common().bounds
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Object::TextFrame(_) => "TextFrame",
            Object::ImageFrame(_) => "ImageFrame",
            Object::Shape(_) => "Shape",
            Object::Line(_) => "Line",
            Object::Group(_) => "Group",
            Object::Table(_) => "Table",
            Object::WordArt(_) => "WordArt",
            Object::TextOnPath(_) => "TextOnPath",
        }
    }

    /// Convert this compat object into a new-model [`repub_model::shape::Shape`].
    pub fn into_shape(self) -> repub_model::shape::Shape {
        match self {
            Object::TextFrame(tf) => tf.into_shape(),
            Object::ImageFrame(img) => img.into_shape(),
            Object::Shape(s) => s.into_shape(),
            Object::Line(l) => l.into_shape(),
            Object::Group(g) => g.into_shape(),
            Object::Table(t) => t.into_shape(),
            Object::WordArt(w) => w.into_shape(),
            Object::TextOnPath(t) => t.into_shape(),
        }
    }

    /// Reconstruct a compat object from a new-model shape (export side).
    pub fn from_shape(shape: &repub_model::shape::Shape) -> Object {
        from_shape(shape)
    }
}

fn base_shape(common: &ObjectCommon, style: &ShapeStyle, kind: ShapeKind) -> repub_model::shape::Shape {
    let mut shape = repub_model::shape::Shape::new(common.bounds, kind);
    common.apply_to(&mut shape.common);
    shape.fill = style.fill.clone();
    shape.line = style.stroke.clone();
    shape.effects = style.to_effects();
    shape.opacity = style.opacity;
    shape
}

impl TextFrame {
    fn into_shape(self) -> repub_model::shape::Shape {
        let data = TextFrameData {
            content: self.content,
            inset: self.inset,
            columns: self.columns,
            overflow: self.overflow,
            vertical_align: self.vertical_align,
            linked_frame: self.linked_frame,
            is_continuation: self.is_continuation,
            text_fit: self.text_fit,
            margins: self.margins,
        };
        base_shape(&self.common, &self.style, ShapeKind::TextFrame(data))
    }
}

impl Shape {
    fn into_shape(self) -> repub_model::shape::Shape {
        let auto = AutoShape {
            geometry: self.shape_type.into(),
            preset: self.preset,
        };
        base_shape(&self.common, &self.style, ShapeKind::AutoShape(auto))
    }
}

impl ImageFrame {
    fn into_shape(self) -> repub_model::shape::Shape {
        let data = PictureData {
            source: convert_image_source(self.source),
            fit: convert_image_fit(self.fit),
            crop: self.crop,
            brightness: self.brightness,
            contrast: self.contrast,
            alt_text: self.alt_text,
        };
        let mut style = self.style.clone();
        // ImageFrame carried its own opacity separate from the frame style.
        if self.opacity != 1.0 {
            style.opacity = self.opacity;
        }
        base_shape(&self.common, &style, ShapeKind::Picture(data))
    }
}

impl Line {
    fn into_shape(self) -> repub_model::shape::Shape {
        let data = LineData {
            start: self.start,
            end: self.end,
            control_points: self.control_points,
            connector_type: self.connector_type,
            start_ending: self.start_ending,
            end_ending: self.end_ending,
        };
        let mut shape = repub_model::shape::Shape::new(self.common.bounds, ShapeKind::Line(data));
        self.common.apply_to(&mut shape.common);
        shape.line = Some(self.stroke);
        shape
    }
}

impl Table {
    fn into_shape(self) -> repub_model::shape::Shape {
        let data = TableData {
            rows: self.rows,
            cols: self.cols,
            row_heights: self.row_heights,
            col_widths: self.col_widths,
            cells: self.cells,
            borders: self.borders,
            default_char_style: self.default_char_style,
            default_para_style: self.default_para_style,
        };
        let mut shape = repub_model::shape::Shape::new(self.common.bounds, ShapeKind::Table(data));
        self.common.apply_to(&mut shape.common);
        shape
    }
}

impl Group {
    fn into_shape(self) -> repub_model::shape::Shape {
        let children: Vec<repub_model::shape::Shape> =
            self.children.into_iter().map(|c| c.into_shape()).collect();
        let mut data = GroupData::from_shapes(children);
        if !self.common.bounds.is_empty() {
            data.bounds = self.common.bounds;
        }
        let mut shape = repub_model::shape::Shape::new(data.bounds, ShapeKind::Group(data));
        self.common.apply_to(&mut shape.common);
        shape
    }
}

impl WordArt {
    fn into_shape(self) -> repub_model::shape::Shape {
        let mut data = TextEffectData::new(self.text);
        data.style = self.style;
        data.transform = convert_text_transform(self.transform);
        data.rotation_3d = self.rotation_3d;
        data.extrusion = self.extrusion;
        data.letter_spacing = self.letter_spacing;
        data.line_spacing = self.line_spacing;
        data.align = convert_wordart_align(self.align);
        data.same_letter_heights = self.same_letter_heights;
        data.vertical = self.vertical;
        let style = ShapeStyle {
            fill: self.fill,
            stroke: self.outline,
            ..ShapeStyle::new()
        };
        base_shape(&self.common, &style, ShapeKind::TextEffect(data))
    }
}

impl TextOnPath {
    fn into_shape(self) -> repub_model::shape::Shape {
        let mut data = TextEffectData::new(self.text);
        data.style = self.style;
        data.transform = NewTextTransform::Path(self.path);
        data.path_start_offset = self.start_offset;
        data.path_align = self.align;
        data.path_spacing = self.spacing;
        data.path_flip = self.flip;
        base_shape(&self.common, &ShapeStyle::new(), ShapeKind::TextEffect(data))
    }
}

fn convert_image_source(src: ImageSource) -> NewImageSource {
    match src {
        ImageSource::File(p) => NewImageSource::File(p),
        ImageSource::Url(u) => NewImageSource::Url(u),
        ImageSource::Embedded { data, mime_type } => NewImageSource::Embedded { data, mime_type },
        ImageSource::Placeholder => NewImageSource::Placeholder,
    }
}

fn convert_image_source_back(src: &NewImageSource) -> ImageSource {
    match src {
        NewImageSource::File(p) => ImageSource::File(p.clone()),
        NewImageSource::Url(u) => ImageSource::Url(u.clone()),
        NewImageSource::Embedded { data, mime_type } => ImageSource::Embedded {
            data: data.clone(),
            mime_type: mime_type.clone(),
        },
        NewImageSource::Placeholder => ImageSource::Placeholder,
    }
}

fn convert_image_fit(fit: ImageFit) -> NewImageFit {
    match fit {
        ImageFit::Fill => NewImageFit::Fill,
        ImageFit::Fit => NewImageFit::Fit,
        ImageFit::Stretch => NewImageFit::Stretch,
        ImageFit::Tile => NewImageFit::Tile,
        ImageFit::Center => NewImageFit::Center,
    }
}

fn convert_text_transform(t: TextTransform) -> NewTextTransform {
    match t {
        TextTransform::None => NewTextTransform::None,
        TextTransform::Preset(p) => NewTextTransform::Preset(convert_wordart_preset(p)),
        TextTransform::Path(path) => NewTextTransform::Path(path),
        TextTransform::Warp { top, bottom } => NewTextTransform::Warp { top, bottom },
    }
}

fn convert_wordart_preset(p: WordArtPreset) -> NewWordArtPreset {
    // The old and new enums have identical variants in the same order.
    p
}

fn convert_wordart_align(a: WordArtAlign) -> NewWordArtAlign {
    a
}

/// Reconstruct a compat [`Object`] from a new-model shape (export side).
pub fn from_shape(shape: &repub_model::shape::Shape) -> Object {
    let common = ObjectCommon::from_shape(&shape.common);
    let style = ShapeStyle::from_parts(
        shape.fill.clone(),
        shape.line.clone(),
        &shape.effects,
        shape.opacity,
    );
    match &shape.kind {
        ShapeKind::TextFrame(tf) => Object::TextFrame(TextFrame {
            common,
            content: tf.content.clone(),
            style,
            inset: tf.inset,
            columns: tf.columns.clone(),
            overflow: tf.overflow,
            vertical_align: tf.vertical_align,
            linked_frame: tf.linked_frame,
            is_continuation: tf.is_continuation,
            text_fit: tf.text_fit,
            margins: tf.margins,
        }),
        ShapeKind::Picture(pic) => Object::ImageFrame(ImageFrame {
            common,
            source: convert_image_source_back(&pic.source),
            fit: convert_image_fit_back(pic.fit),
            opacity: shape.opacity,
            style,
            crop: pic.crop,
            brightness: pic.brightness,
            contrast: pic.contrast,
            alt_text: pic.alt_text.clone(),
        }),
        ShapeKind::AutoShape(auto) => Object::Shape(Shape {
            common,
            shape_type: auto.geometry.clone().into(),
            style,
            preset: auto.preset.clone(),
        }),
        ShapeKind::Line(line) => Object::Line(Line {
            common,
            start: line.start,
            end: line.end,
            control_points: line.control_points.clone(),
            connector_type: line.connector_type,
            stroke: shape.line.clone().unwrap_or_default(),
            start_ending: line.start_ending,
            end_ending: line.end_ending,
        }),
        ShapeKind::Table(t) => Object::Table(Table {
            common,
            rows: t.rows,
            cols: t.cols,
            row_heights: t.row_heights.clone(),
            col_widths: t.col_widths.clone(),
            cells: t.cells.clone(),
            borders: t.borders.clone(),
            default_char_style: t.default_char_style.clone(),
            default_para_style: t.default_para_style.clone(),
        }),
        ShapeKind::TextEffect(te) => text_effect_to_object(common, style, te),
        ShapeKind::Group(g) => {
            let children = g.children.iter().map(from_shape).collect();
            let mut group = Group { common, children };
            group.common.bounds = g.bounds;
            Object::Group(group)
        }
    }
}

fn convert_image_fit_back(fit: NewImageFit) -> ImageFit {
    match fit {
        NewImageFit::Fill => ImageFit::Fill,
        NewImageFit::Fit => ImageFit::Fit,
        NewImageFit::Stretch => ImageFit::Stretch,
        NewImageFit::Tile => ImageFit::Tile,
        NewImageFit::Center => ImageFit::Center,
    }
}

fn text_effect_to_object(common: ObjectCommon, style: ShapeStyle, te: &TextEffectData) -> Object {
    if let NewTextTransform::Path(path) = &te.transform {
        Object::TextOnPath(TextOnPath {
            common,
            text: te.text.clone(),
            style: te.style.clone(),
            path: path.clone(),
            start_offset: te.path_start_offset,
            align: te.path_align,
            spacing: te.path_spacing,
            flip: te.path_flip,
        })
    } else {
        let transform = match &te.transform {
            NewTextTransform::None => TextTransform::None,
            NewTextTransform::Preset(p) => TextTransform::Preset(*p),
            NewTextTransform::Warp { top, bottom } => TextTransform::Warp {
                top: top.clone(),
                bottom: bottom.clone(),
            },
            NewTextTransform::Path(_) => TextTransform::None,
        };
        Object::WordArt(WordArt {
            common,
            text: te.text.clone(),
            style: te.style.clone(),
            fill: style.fill,
            outline: style.stroke,
            transform,
            rotation_3d: te.rotation_3d,
            extrusion: te.extrusion.clone(),
            letter_spacing: te.letter_spacing,
            line_spacing: te.line_spacing,
            align: te.align,
            same_letter_heights: te.same_letter_heights,
            vertical: te.vertical,
        })
    }
}

/// Bridge from a new-model [`repub_model::Page`] to the compat object model.
pub trait PageObjectExt {
    /// Add a compat object to the page (converting it to a model shape).
    fn add_object(&mut self, obj: Object) -> repub_model::ObjectId;
    /// Insert a compat object at `index` (converting it to a model shape).
    fn insert_object(&mut self, index: usize, obj: Object) -> repub_model::ObjectId;
    /// All objects on the page, reconstructed from the page's shapes.
    fn all_objects(&self) -> Vec<Object>;
}

impl PageObjectExt for repub_model::Page {
    fn add_object(&mut self, obj: Object) -> repub_model::ObjectId {
        self.add_shape(obj.into_shape())
    }

    fn insert_object(&mut self, index: usize, obj: Object) -> repub_model::ObjectId {
        self.insert_shape(index, obj.into_shape())
    }

    fn all_objects(&self) -> Vec<Object> {
        self.all_shapes().map(from_shape).collect()
    }
}

/// Bridge from a new-model [`repub_model::Layer`] to the compat object model.
pub trait LayerObjectExt {
    /// All objects on the layer, reconstructed from the layer's shapes.
    fn objects(&self) -> Vec<Object>;
}

impl LayerObjectExt for repub_model::Layer {
    fn objects(&self) -> Vec<Object> {
        self.shapes().iter().map(from_shape).collect()
    }
}
