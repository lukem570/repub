//! The unified shape type and all its kind-specific payloads.
//!
//! Publisher represents every object on a page as one shape class
//! (`CPubShape`) with a `PbShapeType` discriminator. Every shape carries a
//! fill, a line (stroke), effects and placement, plus type-specific data. This
//! module mirrors that: [`Shape`] holds the common placement and styling, and
//! [`ShapeKind`] holds the type-specific payload. [`ShapeType`] is the
//! lightweight discriminator that mirrors `PbShapeType`.

use repub_color::Color;
use repub_geometry::{Point, Rect, Transform2D};

use crate::geometry::{AutoShape, Path, PathCommand, ShapeGeometry};
use crate::id::ObjectId;
use crate::styles::{
    CharacterStyle, Effects, Fill, FontStyle, FontWeight, ParagraphStyle, Stroke, TextWrap,
};
use crate::text::StyledText;

/// Placement and bookkeeping common to all shapes, regardless of kind.
///
/// Mirrors the fields every `CPubShape` carries in Publisher: identity,
/// geometry (position/size/rotation/flips), stacking order, and editing flags.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeCommon {
    /// Unique identifier for this shape.
    pub id: ObjectId,
    /// Bounding rectangle in page coordinates.
    pub bounds: Rect,
    /// Rotation angle in degrees (clockwise).
    pub rotation: f64,
    /// Additional 2D transform applied after rotation.
    pub transform: Transform2D,
    /// Whether the shape is visible.
    pub visible: bool,
    /// Whether the shape is locked against editing.
    pub locked: bool,
    /// Optional author-assigned name.
    pub name: Option<String>,
    /// Z-order within the layer (higher is on top).
    pub z_index: i32,
    /// How surrounding text wraps around this shape.
    pub text_wrap: TextWrap,
    /// Mirrored left to right within its frame (before rotation).
    pub flip_h: bool,
    /// Mirrored top to bottom within its frame (before rotation).
    pub flip_v: bool,
}

impl ShapeCommon {
    /// Create common properties for a shape at the given bounds, with a fresh ID.
    pub fn new(bounds: Rect) -> Self {
        Self {
            id: ObjectId::new(),
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

    /// The center point of the bounds.
    pub fn center(&self) -> Point {
        self.bounds.center()
    }

    /// The full transform including flips and rotation about the centre
    /// (flips first), then the extra [`ShapeCommon::transform`].
    pub fn full_transform(&self) -> Transform2D {
        if self.rotation == 0.0 && !self.flip_h && !self.flip_v {
            return self.transform;
        }
        let center = self.center();
        let to_origin = Transform2D::translate(-center.x, -center.y);
        let flip = Transform2D::scale(
            if self.flip_h { -1.0 } else { 1.0 },
            if self.flip_v { -1.0 } else { 1.0 },
        );
        let rotate = Transform2D::rotate(self.rotation.to_radians());
        let from_origin = Transform2D::translate(center.x, center.y);
        // `a.concat(b)` applies `b` to a point before `a`, so the chain reads
        // left-to-right as "outermost first" but points flow right-to-left. To
        // flip and rotate about the centre and then apply the extra transform,
        // a point must go: to_origin (−centre) → flip → rotate → from_origin
        // (+centre) → transform. That is the matrix product
        // transform · from_origin · rotate · flip · to_origin.
        self.transform
            .concat(&from_origin)
            .concat(&rotate)
            .concat(&flip)
            .concat(&to_origin)
    }

    /// Set the position (top-left corner).
    pub fn set_position(&mut self, x: f64, y: f64) {
        self.bounds.origin.x = x;
        self.bounds.origin.y = y;
    }

    /// Set the size.
    pub fn set_size(&mut self, width: f64, height: f64) {
        self.bounds.size.width = width;
        self.bounds.size.height = height;
    }

    /// Move by the given offset.
    pub fn translate(&mut self, dx: f64, dy: f64) {
        self.bounds.origin.x += dx;
        self.bounds.origin.y += dy;
    }
}

impl Default for ShapeCommon {
    fn default() -> Self {
        Self::new(Rect::default())
    }
}

/// Behavior when text overflows the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextOverflow {
    /// Clip text at frame boundary
    #[default]
    Clip,
    /// Show overflow indicator
    ShowIndicator,
    /// Auto-resize frame to fit text
    AutoResize,
    /// Flow text to linked frame
    FlowToLinked,
}

/// Column specification for multi-column text.
#[derive(Debug, Clone, PartialEq)]
pub struct ColumnSpec {
    /// Number of columns
    pub count: u32,
    /// Gap between columns in points
    pub gap: f64,
    /// Whether columns have equal width
    pub equal_width: bool,
    /// Custom column widths (if not equal)
    pub widths: Vec<f64>,
}

impl Default for ColumnSpec {
    fn default() -> Self {
        Self {
            count: 1,
            gap: 12.0,
            equal_width: true,
            widths: Vec::new(),
        }
    }
}

impl ColumnSpec {
    /// Create a single-column spec.
    pub fn single() -> Self {
        Self::default()
    }

    /// Create equal-width columns.
    pub fn equal(count: u32, gap: f64) -> Self {
        Self {
            count,
            gap,
            equal_width: true,
            widths: Vec::new(),
        }
    }

    /// Create columns with custom widths.
    pub fn custom(widths: Vec<f64>, gap: f64) -> Self {
        Self {
            count: widths.len() as u32,
            gap,
            equal_width: false,
            widths,
        }
    }
}

/// Vertical alignment within text frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum VerticalAlignment {
    /// Align to top
    #[default]
    Top,
    /// Center vertically
    Center,
    /// Align to bottom
    Bottom,
}

/// Text fit mode for auto-sizing text to fit the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextFitMode {
    /// Do not auto-fit text
    #[default]
    None,
    /// Shrink text only if it overflows
    ShrinkOnOverflow,
    /// Grow text to fill the frame
    GrowToFit,
    /// Find optimal size (shrink or grow)
    BestFit,
}

/// Per-side text box margins in points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Margins {
    /// Top margin
    pub top: f64,
    /// Bottom margin
    pub bottom: f64,
    /// Left margin
    pub left: f64,
    /// Right margin
    pub right: f64,
}

impl Margins {
    /// Uniform margins on all sides.
    pub fn uniform(value: f64) -> Self {
        Self {
            top: value,
            bottom: value,
            left: value,
            right: value,
        }
    }

    /// Zero margins.
    pub fn zero() -> Self {
        Self::uniform(0.0)
    }
}

impl Default for Margins {
    fn default() -> Self {
        Self::uniform(4.0)
    }
}

/// The text-specific payload of a text-frame shape.
///
/// The frame's placement, background fill and border live on the enclosing
/// [`Shape`]; this holds only the text content and layout.
#[derive(Debug, Clone, PartialEq)]
pub struct TextFrameData {
    /// The text content
    pub content: StyledText,
    /// Text inset from frame edges in points
    pub inset: f64,
    /// Column specification
    pub columns: ColumnSpec,
    /// Overflow behavior
    pub overflow: TextOverflow,
    /// Vertical alignment
    pub vertical_align: VerticalAlignment,
    /// ID of linked frame for text overflow (if any)
    pub linked_frame: Option<ObjectId>,
    /// Whether this frame receives overflow from another frame
    pub is_continuation: bool,
    /// Text fit mode for auto-sizing
    pub text_fit: TextFitMode,
    /// Per-side text box margins (falls back to `inset` when all zero)
    pub margins: Margins,
}

impl Default for TextFrameData {
    fn default() -> Self {
        Self::new()
    }
}

impl TextFrameData {
    /// Create new empty text-frame data.
    pub fn new() -> Self {
        Self {
            content: StyledText::new(),
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

    /// Margins actually used for layout: per-side `margins` when set, else `inset`.
    pub fn effective_margins(&self) -> Margins {
        if self.margins.top == 0.0
            && self.margins.bottom == 0.0
            && self.margins.left == 0.0
            && self.margins.right == 0.0
        {
            Margins::uniform(self.inset)
        } else {
            self.margins
        }
    }

    /// Set the text content.
    pub fn set_text(&mut self, text: &str) {
        self.content = StyledText::plain(text);
    }

    /// Get the plain text content.
    pub fn plain_text(&self) -> String {
        self.content.plain_text()
    }

    /// Set the number of columns.
    pub fn set_columns(&mut self, count: u32, gap: f64) {
        self.columns = ColumnSpec::equal(count, gap);
    }

    /// Link this frame to another for text overflow.
    pub fn link_to(&mut self, other_id: ObjectId) {
        self.linked_frame = Some(other_id);
        self.overflow = TextOverflow::FlowToLinked;
    }

    /// Get the content area (`bounds` minus inset).
    pub fn content_rect(&self, bounds: Rect) -> Rect {
        Rect::new(
            bounds.x() + self.inset,
            bounds.y() + self.inset,
            bounds.width() - self.inset * 2.0,
            bounds.height() - self.inset * 2.0,
        )
    }
}

/// Source of an image.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ImageSource {
    /// Image from a file path
    File(String),
    /// Image from a URL
    Url(String),
    /// Embedded image data (base64 encoded)
    Embedded {
        /// Image data
        data: String,
        /// MIME type (e.g., "image/png")
        mime_type: String,
    },
    /// Placeholder (image not yet loaded)
    #[default]
    Placeholder,
}

/// How the image fits within the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ImageFit {
    /// Scale to fill frame, maintaining aspect ratio, cropping if needed
    Fill,
    /// Scale to fit within frame, maintaining aspect ratio, may show background
    Fit,
    /// Stretch to exactly fill frame, may distort. The default, as in
    /// Publisher: resizing a picture frame resizes the picture with it.
    #[default]
    Stretch,
    /// Tile the image to fill the frame
    Tile,
    /// Center at original size
    Center,
}

/// The picture-specific payload of a picture shape.
///
/// The frame's placement, background fill, border and overall opacity live on
/// the enclosing [`Shape`]; this holds only the image data.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PictureData {
    /// Image source
    pub source: ImageSource,
    /// How the image fits in the frame
    pub fit: ImageFit,
    /// Crop rectangle (normalized 0-1, relative to original image)
    pub crop: Option<Rect>,
    /// Image brightness adjustment (-1.0 to 1.0)
    pub brightness: f64,
    /// Image contrast adjustment (-1.0 to 1.0)
    pub contrast: f64,
    /// Alternative text for accessibility
    pub alt_text: Option<String>,
}

impl PictureData {
    /// Create new empty picture data (a placeholder image).
    pub fn new() -> Self {
        Self::default()
    }

    /// Create picture data from a file path.
    pub fn from_file(path: impl Into<String>) -> Self {
        let mut data = Self::new();
        data.source = ImageSource::File(path.into());
        data
    }

    /// Create picture data from a URL.
    pub fn from_url(url: impl Into<String>) -> Self {
        let mut data = Self::new();
        data.source = ImageSource::Url(url.into());
        data
    }

    /// Set the image source.
    pub fn set_source(&mut self, source: ImageSource) {
        self.source = source;
    }

    /// Set how the image fits in the frame.
    pub fn set_fit(&mut self, fit: ImageFit) {
        self.fit = fit;
    }

    /// Set the crop rectangle (normalized 0-1 coordinates).
    pub fn set_crop(&mut self, x: f64, y: f64, width: f64, height: f64) {
        self.crop = Some(Rect::new(x, y, width, height));
    }

    /// Clear the crop.
    pub fn clear_crop(&mut self) {
        self.crop = None;
    }

    /// Check if the image source is set.
    pub fn has_image(&self) -> bool {
        !matches!(self.source, ImageSource::Placeholder)
    }
}

/// Horizontal alignment within a table cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellHAlign {
    /// Align content to the left edge of the cell
    #[default]
    Left,
    /// Center content horizontally within the cell
    Center,
    /// Align content to the right edge of the cell
    Right,
}

/// Vertical alignment within a table cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CellVAlign {
    /// Align content to the top of the cell
    #[default]
    Top,
    /// Center content vertically within the cell
    Middle,
    /// Align content to the bottom of the cell
    Bottom,
}

/// A single cell in a table.
#[derive(Debug, Clone, PartialEq)]
pub struct TableCell {
    /// Cell content
    pub content: StyledText,
    /// Background fill (Publisher stores a fill per cell)
    pub fill: Fill,
    /// Cell padding in points
    pub padding: f64,
    /// Horizontal alignment
    pub h_align: CellHAlign,
    /// Vertical alignment
    pub v_align: CellVAlign,
    /// Number of columns this cell spans
    pub col_span: u32,
    /// Number of rows this cell spans
    pub row_span: u32,
}

impl Default for TableCell {
    fn default() -> Self {
        Self {
            content: StyledText::new(),
            fill: Fill::None,
            padding: 1.0,
            h_align: CellHAlign::Left,
            v_align: CellVAlign::Top,
            col_span: 1,
            row_span: 1,
        }
    }
}

impl TableCell {
    /// Create an empty cell.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a cell with text content.
    pub fn with_text(text: &str) -> Self {
        Self {
            content: StyledText::plain(text),
            ..Default::default()
        }
    }

    /// Set the cell text.
    pub fn set_text(&mut self, text: &str) {
        self.content = StyledText::plain(text);
    }

    /// Get the plain text content.
    pub fn plain_text(&self) -> String {
        self.content.plain_text()
    }

    /// Set the background fill.
    pub fn with_fill(mut self, fill: Fill) -> Self {
        self.fill = fill;
        self
    }

    /// Set the background color.
    pub fn with_background(mut self, color: Color) -> Self {
        self.fill = Fill::Solid(color);
        self
    }

    /// Set alignment.
    pub fn with_alignment(mut self, h: CellHAlign, v: CellVAlign) -> Self {
        self.h_align = h;
        self.v_align = v;
        self
    }

    /// Set cell spanning.
    pub fn with_span(mut self, col_span: u32, row_span: u32) -> Self {
        self.col_span = col_span.max(1);
        self.row_span = row_span.max(1);
        self
    }
}

/// Border style for table edges.
#[derive(Debug, Clone, PartialEq)]
pub struct TableBorders {
    /// Top border
    pub top: Option<Stroke>,
    /// Right border
    pub right: Option<Stroke>,
    /// Bottom border
    pub bottom: Option<Stroke>,
    /// Left border
    pub left: Option<Stroke>,
    /// Horizontal inner borders
    pub inner_horizontal: Option<Stroke>,
    /// Vertical inner borders
    pub inner_vertical: Option<Stroke>,
}

impl Default for TableBorders {
    fn default() -> Self {
        let default_border = Some(Stroke::solid(Color::BLACK, 0.5));
        Self {
            top: default_border.clone(),
            right: default_border.clone(),
            bottom: default_border.clone(),
            left: default_border.clone(),
            inner_horizontal: default_border.clone(),
            inner_vertical: default_border,
        }
    }
}

impl TableBorders {
    /// Create borders with no lines.
    pub fn none() -> Self {
        Self {
            top: None,
            right: None,
            bottom: None,
            left: None,
            inner_horizontal: None,
            inner_vertical: None,
        }
    }

    /// Create uniform borders.
    pub fn uniform(stroke: Stroke) -> Self {
        let s = Some(stroke);
        Self {
            top: s.clone(),
            right: s.clone(),
            bottom: s.clone(),
            left: s.clone(),
            inner_horizontal: s.clone(),
            inner_vertical: s,
        }
    }

    /// Create only outer borders.
    pub fn outer_only(stroke: Stroke) -> Self {
        let s = Some(stroke);
        Self {
            top: s.clone(),
            right: s.clone(),
            bottom: s.clone(),
            left: s,
            inner_horizontal: None,
            inner_vertical: None,
        }
    }
}

/// The table-specific payload of a table shape.
///
/// The table's placement, fill, border and effects live on the enclosing
/// [`Shape`]; this holds the grid, its cells and defaults. Cells keep their
/// own fill, as Publisher stores a fill per cell. The grid's cell sizes
/// (`row_heights`/`col_widths`) are relative units scaled to the shape's
/// on-page bounds; methods that need the on-page rectangle take `bounds`.
#[derive(Debug, Clone, PartialEq)]
pub struct TableData {
    /// Number of rows
    pub rows: u32,
    /// Number of columns
    pub cols: u32,
    /// Row heights in points
    pub row_heights: Vec<f64>,
    /// Column widths in points
    pub col_widths: Vec<f64>,
    /// Cells (row-major order)
    pub cells: Vec<TableCell>,
    /// Border styles
    pub borders: TableBorders,
    /// Default character style for cells
    pub default_char_style: CharacterStyle,
    /// Default paragraph style for cells
    pub default_para_style: ParagraphStyle,
}

impl TableData {
    /// Create new table data sized to `bounds` with the given dimensions.
    pub fn new(bounds: Rect, rows: u32, cols: u32) -> Self {
        let row_height = bounds.height() / rows as f64;
        let col_width = bounds.width() / cols as f64;

        let cells: Vec<TableCell> = (0..(rows * cols)).map(|_| TableCell::new()).collect();

        Self {
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

    /// Get a cell by row and column.
    pub fn cell(&self, row: u32, col: u32) -> Option<&TableCell> {
        if row < self.rows && col < self.cols {
            Some(&self.cells[(row * self.cols + col) as usize])
        } else {
            None
        }
    }

    /// Get a mutable cell by row and column.
    pub fn cell_mut(&mut self, row: u32, col: u32) -> Option<&mut TableCell> {
        if row < self.rows && col < self.cols {
            Some(&mut self.cells[(row * self.cols + col) as usize])
        } else {
            None
        }
    }

    /// Set cell text.
    pub fn set_cell_text(&mut self, row: u32, col: u32, text: &str) {
        if let Some(cell) = self.cell_mut(row, col) {
            cell.set_text(text);
        }
    }

    /// Get the on-page bounds of a specific cell within the table's `bounds`.
    pub fn cell_bounds(&self, bounds: Rect, row: u32, col: u32) -> Option<Rect> {
        if row >= self.rows || col >= self.cols {
            return None;
        }

        let x: f64 = self.col_widths[..col as usize].iter().sum();
        let y: f64 = self.row_heights[..row as usize].iter().sum();
        let width = self.col_widths[col as usize];
        let height = self.row_heights[row as usize];

        Some(Rect::new(bounds.x() + x, bounds.y() + y, width, height))
    }

    /// Set the width of a column.
    pub fn set_col_width(&mut self, col: u32, width: f64) {
        if (col as usize) < self.col_widths.len() {
            self.col_widths[col as usize] = width;
        }
    }

    /// Set the height of a row.
    pub fn set_row_height(&mut self, row: u32, height: f64) {
        if (row as usize) < self.row_heights.len() {
            self.row_heights[row as usize] = height;
        }
    }

    /// Add a row at the end.
    pub fn add_row(&mut self) {
        let default_height = if self.row_heights.is_empty() {
            20.0
        } else {
            self.row_heights[self.row_heights.len() - 1]
        };
        self.row_heights.push(default_height);
        self.rows += 1;

        for _ in 0..self.cols {
            self.cells.push(TableCell::new());
        }
    }

    /// Add a column at the end.
    pub fn add_col(&mut self) {
        let default_width = if self.col_widths.is_empty() {
            60.0
        } else {
            self.col_widths[self.col_widths.len() - 1]
        };
        self.col_widths.push(default_width);

        // Insert new cells at the end of each row (work backwards to keep indices stable)
        let old_cols = self.cols;
        self.cols += 1;

        for row in (0..self.rows).rev() {
            let insert_pos = (row * old_cols + old_cols) as usize;
            self.cells.insert(insert_pos, TableCell::new());
        }
    }

    /// An empty cell formatted like `like` (fill, padding, alignment and text
    /// style), for a new row or column.
    fn blank_like(like: Option<&TableCell>) -> TableCell {
        let Some(like) = like else {
            return TableCell::new();
        };
        let mut cell = like.clone();
        cell.col_span = 1;
        cell.row_span = 1;
        // Keep the first paragraph's style (and the first run's character style) for typing
        let mut para = like.content.paragraphs.first().cloned().unwrap_or_default();
        let char_style = para.elements.iter().find_map(|e| match e {
            crate::text::InlineElement::Text(run) => Some(run.style.clone()),
            _ => None,
        });
        if let Some(style) = char_style {
            para.style.default_char_style = style;
        }
        para.elements.clear();
        cell.content = StyledText {
            paragraphs: vec![para],
        };
        cell
    }

    /// Insert an empty row at index `at` (0..=rows), formatted like the row
    /// beside it and as tall as it (in grid units). Merged cells crossing the
    /// new row span it too. The on-page size scales with the shape's bounds.
    pub fn insert_row_at(&mut self, at: u32) {
        let at = at.min(self.rows);
        let cols = self.cols as usize;
        let like_row = if at > 0 { at - 1 } else { 0 };
        let new_cells: Vec<TableCell> = (0..cols)
            .map(|c| Self::blank_like(self.cell(like_row, c as u32)))
            .collect();
        // Spans crossing the insertion point grow
        for r in 0..at as usize {
            for c in 0..cols {
                if let Some(cell) = self.cells.get_mut(r * cols + c) {
                    if r + cell.row_span.max(1) as usize > at as usize {
                        cell.row_span += 1;
                    }
                }
            }
        }
        let index = at as usize * cols;
        self.cells.splice(index..index, new_cells);
        // Give the new row the same grid-unit height as its neighbour
        let unit = self
            .row_heights
            .get(like_row as usize)
            .copied()
            .unwrap_or(20.0);
        self.row_heights.insert(at as usize, unit);
        self.rows += 1;
    }

    /// Insert an empty column at index `at` (0..=cols); see [`Self::insert_row_at`].
    pub fn insert_col_at(&mut self, at: u32) {
        let at = at.min(self.cols);
        let old_cols = self.cols as usize;
        let like_col = if at > 0 { at - 1 } else { 0 };
        for r in 0..self.rows as usize {
            for c in 0..at as usize {
                if let Some(cell) = self.cells.get_mut(r * old_cols + c) {
                    if c + cell.col_span.max(1) as usize > at as usize {
                        cell.col_span += 1;
                    }
                }
            }
        }
        for r in (0..self.rows as usize).rev() {
            let blank = Self::blank_like(self.cell(r as u32, like_col));
            self.cells.insert(r * old_cols + at as usize, blank);
        }
        let unit = self
            .col_widths
            .get(like_col as usize)
            .copied()
            .unwrap_or(60.0);
        self.col_widths.insert(at as usize, unit);
        self.cols += 1;
    }

    /// Remove row `row` (its text goes with it). Merged cells crossing it span
    /// one row less. The last row can't be removed.
    pub fn remove_row(&mut self, row: u32) -> bool {
        if self.rows <= 1 || row >= self.rows {
            return false;
        }
        let cols = self.cols as usize;
        for r in 0..row as usize {
            for c in 0..cols {
                if let Some(cell) = self.cells.get_mut(r * cols + c) {
                    if r + cell.row_span.max(1) as usize > row as usize {
                        cell.row_span = cell.row_span.saturating_sub(1).max(1);
                    }
                }
            }
        }
        let index = row as usize * cols;
        self.cells.drain(index..index + cols);
        self.row_heights.remove(row as usize);
        self.rows -= 1;
        true
    }

    /// Remove column `col`; see [`Self::remove_row`].
    pub fn remove_col(&mut self, col: u32) -> bool {
        if self.cols <= 1 || col >= self.cols {
            return false;
        }
        let old_cols = self.cols as usize;
        for r in 0..self.rows as usize {
            for c in 0..col as usize {
                if let Some(cell) = self.cells.get_mut(r * old_cols + c) {
                    if c + cell.col_span.max(1) as usize > col as usize {
                        cell.col_span = cell.col_span.saturating_sub(1).max(1);
                    }
                }
            }
        }
        for r in (0..self.rows as usize).rev() {
            self.cells.remove(r * old_cols + col as usize);
        }
        self.col_widths.remove(col as usize);
        self.cols -= 1;
        true
    }

    /// Total width of the table (sum of grid-unit column widths).
    pub fn total_width(&self) -> f64 {
        self.col_widths.iter().sum()
    }

    /// Total height of the table (sum of grid-unit row heights).
    pub fn total_height(&self) -> f64 {
        self.row_heights.iter().sum()
    }
}

/// Style of line ending (arrow, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineEndingStyle {
    /// No ending
    #[default]
    None,
    /// Triangle arrow
    Arrow,
    /// Open arrow (two lines forming a V)
    OpenArrow,
    /// Stealth arrow (sharp triangle)
    StealthArrow,
    /// Diamond
    Diamond,
    /// Circle/dot
    Circle,
    /// Square
    Square,
}

/// Line ending specification.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LineEnding {
    /// Style of the ending
    pub style: LineEndingStyle,
    /// Size of the ending (relative to line width)
    pub size: f64,
    /// Whether the ending is filled or just outlined
    pub filled: bool,
}

impl Default for LineEnding {
    fn default() -> Self {
        Self {
            style: LineEndingStyle::None,
            size: 3.0,
            filled: true,
        }
    }
}

impl LineEnding {
    /// Create a line ending with the given style.
    pub fn new(style: LineEndingStyle) -> Self {
        Self {
            style,
            size: 3.0,
            filled: true,
        }
    }

    /// Create an arrow ending.
    pub fn arrow() -> Self {
        Self::new(LineEndingStyle::Arrow)
    }

    /// Create a circle/dot ending.
    pub fn circle() -> Self {
        Self::new(LineEndingStyle::Circle)
    }
}

/// Type of connector routing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectorType {
    /// Straight line
    #[default]
    Straight,
    /// Elbow (right-angle bends)
    Elbow,
    /// Curved line
    Curved,
}

/// The line-specific payload of a line (or connector) shape.
///
/// In Publisher a line is a shape too; its stroke lives on the enclosing
/// [`Shape`] as [`Shape::line`]. This holds only the geometry and endings.
#[derive(Debug, Clone, PartialEq)]
pub struct LineData {
    /// Start point (page coordinates)
    pub start: Point,
    /// End point (page coordinates)
    pub end: Point,
    /// Control points for curved lines
    pub control_points: Vec<Point>,
    /// Connector type
    pub connector_type: ConnectorType,
    /// Starting line ending
    pub start_ending: LineEnding,
    /// Ending line ending
    pub end_ending: LineEnding,
}

impl LineData {
    /// Create a straight line between two points.
    pub fn new(start: Point, end: Point) -> Self {
        Self {
            start,
            end,
            control_points: Vec::new(),
            connector_type: ConnectorType::Straight,
            start_ending: LineEnding::default(),
            end_ending: LineEnding::default(),
        }
    }

    /// Get the length of the line (straight-line distance).
    pub fn length(&self) -> f64 {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        (dx * dx + dy * dy).sqrt()
    }

    /// Get the angle of the line in radians.
    pub fn angle(&self) -> f64 {
        let dx = self.end.x - self.start.x;
        let dy = self.end.y - self.start.y;
        dy.atan2(dx)
    }
}

/// Preset WordArt styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WordArtPreset {
    /// Plain text with custom styling
    #[default]
    Plain,
    /// Text with outline only
    Outline,
    /// Text with shadow
    Shadow,
    /// Text with 3D effect
    ThreeD,
    /// Curved text (arch up)
    ArchUp,
    /// Curved text (arch down)
    ArchDown,
    /// Circle text
    Circle,
    /// Wave effect
    Wave,
    /// Inflate effect
    Inflate,
    /// Deflate effect
    Deflate,
    /// Slant up
    SlantUp,
    /// Slant down
    SlantDown,
    /// Fade right
    FadeRight,
    /// Fade left
    FadeLeft,
    /// Chevron up
    ChevronUp,
    /// Chevron down
    ChevronDown,
    /// Triangle up
    TriangleUp,
    /// Triangle down
    TriangleDown,
    /// Can up (cylindrical text curving up)
    CanUp,
    /// Can down (cylindrical text curving down)
    CanDown,
    /// Cascade up (text gets smaller going right, tilted up)
    CascadeUp,
    /// Cascade down (text gets smaller going right, tilted down)
    CascadeDown,
    /// Stop sign shape (octagonal)
    Stop,
    /// Ring inside (text on inside of ring)
    RingInside,
    /// Ring outside (text on outside of ring)
    RingOutside,
    /// Button (text curved around button shape)
    Button,
}

/// Alignment for text on a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextPathAlign {
    /// Align to start of path
    #[default]
    Left,
    /// Center on path
    Center,
    /// Align to end of path
    Right,
    /// Stretch to fill path
    Stretch,
}

/// Spacing method for text on a path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextPathSpacing {
    /// Exact spacing (may distort letters)
    #[default]
    Exact,
    /// Auto spacing (preserves letter shapes)
    Auto,
}

/// Text transformation type for a text effect.
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TextTransform {
    /// No transformation
    #[default]
    None,
    /// Follow a preset shape
    Preset(WordArtPreset),
    /// Follow a custom path (text on a path); path-specific settings live on
    /// [`TextEffectData`].
    Path(Path),
    /// Warp with control points
    Warp {
        /// Top edge control points (normalized 0-1)
        top: Vec<Point>,
        /// Bottom edge control points (normalized 0-1)
        bottom: Vec<Point>,
    },
}

/// 3D rotation for a text effect.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rotation3D {
    /// Rotation around X axis in degrees
    pub x: f64,
    /// Rotation around Y axis in degrees
    pub y: f64,
    /// Rotation around Z axis in degrees
    pub z: f64,
    /// Perspective depth (0 = no perspective)
    pub perspective: f64,
}

impl Default for Rotation3D {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            perspective: 0.0,
        }
    }
}

impl Rotation3D {
    /// Create a new 3D rotation.
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            x,
            y,
            z,
            perspective: 0.0,
        }
    }

    /// Create rotation with perspective.
    pub fn with_perspective(x: f64, y: f64, z: f64, perspective: f64) -> Self {
        Self {
            x,
            y,
            z,
            perspective,
        }
    }
}

/// 3D extrusion settings for a text effect.
#[derive(Debug, Clone, PartialEq)]
pub struct Extrusion3D {
    /// Extrusion depth in points
    pub depth: f64,
    /// Extrusion color (None = same as text)
    pub color: Option<Color>,
    /// Contour width
    pub contour_width: f64,
    /// Bevel type for the front face
    pub bevel: Option<BevelType>,
    /// Bevel size in points
    pub bevel_size: f64,
}

impl Default for Extrusion3D {
    fn default() -> Self {
        Self {
            depth: 0.0,
            color: None,
            contour_width: 0.0,
            bevel: None,
            bevel_size: 6.0,
        }
    }
}

/// Bevel types for 3D effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BevelType {
    /// Flat bevel
    Flat,
    /// Circle bevel
    Circle,
    /// Relaxed inset
    RelaxedInset,
    /// Cross bevel
    Cross,
    /// Convex bevel
    Convex,
    /// Cool slant
    CoolSlant,
    /// Angle bevel
    Angle,
    /// Soft round
    SoftRound,
    /// Art deco
    ArtDeco,
    /// Slope
    Slope,
    /// Divot
    Divot,
    /// Riblet
    Riblet,
    /// Hard edge
    HardEdge,
}

/// Preset 3D styles matching Publisher's gallery (1-20)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preset3DStyle {
    /// No 3D effect
    None,
    /// Style 1: Parallel right
    ParallelRight,
    /// Style 2: Parallel left
    ParallelLeft,
    /// Style 3: Parallel top
    ParallelTop,
    /// Style 4: Parallel bottom
    ParallelBottom,
    /// Style 5: Perspective right
    PerspectiveRight,
    /// Style 6: Perspective left
    PerspectiveLeft,
    /// Style 7: Perspective contrasting left
    PerspectiveContrastingLeft,
    /// Style 8: Perspective contrasting right
    PerspectiveContrastingRight,
    /// Style 9: Perspective heroic extreme right
    PerspectiveHeroicRight,
    /// Style 10: Perspective heroic extreme left
    PerspectiveHeroicLeft,
    /// Style 11: Off axis 1 right
    OffAxis1Right,
    /// Style 12: Off axis 1 left
    OffAxis1Left,
    /// Style 13: Off axis 1 top
    OffAxis1Top,
    /// Style 14: Off axis 1 bottom
    OffAxis1Bottom,
    /// Style 15: Off axis 2 right
    OffAxis2Right,
    /// Style 16: Off axis 2 left
    OffAxis2Left,
    /// Style 17: Isometric left down
    IsometricLeftDown,
    /// Style 18: Isometric right down
    IsometricRightDown,
    /// Style 19: Isometric left up
    IsometricLeftUp,
    /// Style 20: Isometric right up
    IsometricRightUp,
}

impl Preset3DStyle {
    /// Get the extrusion parameters for this preset
    pub fn to_extrusion(&self) -> Option<Extrusion3D> {
        match self {
            Preset3DStyle::None => None,
            _ => Some(Extrusion3D {
                depth: self.default_depth(),
                color: None,
                contour_width: 0.0,
                bevel: self.default_bevel(),
                bevel_size: 6.0,
            }),
        }
    }

    /// Get the 3D rotation for this preset
    pub fn to_rotation(&self) -> Rotation3D {
        match self {
            Preset3DStyle::None => Rotation3D::default(),
            Preset3DStyle::ParallelRight => Rotation3D::new(0.0, 10.0, 0.0),
            Preset3DStyle::ParallelLeft => Rotation3D::new(0.0, -10.0, 0.0),
            Preset3DStyle::ParallelTop => Rotation3D::new(-10.0, 0.0, 0.0),
            Preset3DStyle::ParallelBottom => Rotation3D::new(10.0, 0.0, 0.0),
            Preset3DStyle::PerspectiveRight => Rotation3D::with_perspective(0.0, 25.0, 0.0, 0.5),
            Preset3DStyle::PerspectiveLeft => Rotation3D::with_perspective(0.0, -25.0, 0.0, 0.5),
            Preset3DStyle::PerspectiveContrastingLeft => {
                Rotation3D::with_perspective(10.0, -20.0, 0.0, 0.6)
            }
            Preset3DStyle::PerspectiveContrastingRight => {
                Rotation3D::with_perspective(10.0, 20.0, 0.0, 0.6)
            }
            Preset3DStyle::PerspectiveHeroicRight => {
                Rotation3D::with_perspective(0.0, 45.0, 0.0, 0.8)
            }
            Preset3DStyle::PerspectiveHeroicLeft => {
                Rotation3D::with_perspective(0.0, -45.0, 0.0, 0.8)
            }
            Preset3DStyle::OffAxis1Right => Rotation3D::new(15.0, 25.0, 0.0),
            Preset3DStyle::OffAxis1Left => Rotation3D::new(15.0, -25.0, 0.0),
            Preset3DStyle::OffAxis1Top => Rotation3D::new(-25.0, 15.0, 0.0),
            Preset3DStyle::OffAxis1Bottom => Rotation3D::new(25.0, 15.0, 0.0),
            Preset3DStyle::OffAxis2Right => Rotation3D::new(-15.0, 30.0, 0.0),
            Preset3DStyle::OffAxis2Left => Rotation3D::new(-15.0, -30.0, 0.0),
            Preset3DStyle::IsometricLeftDown => Rotation3D::new(45.0, -45.0, 0.0),
            Preset3DStyle::IsometricRightDown => Rotation3D::new(45.0, 45.0, 0.0),
            Preset3DStyle::IsometricLeftUp => Rotation3D::new(-45.0, -45.0, 0.0),
            Preset3DStyle::IsometricRightUp => Rotation3D::new(-45.0, 45.0, 0.0),
        }
    }

    fn default_depth(&self) -> f64 {
        match self {
            Preset3DStyle::None => 0.0,
            Preset3DStyle::PerspectiveHeroicRight | Preset3DStyle::PerspectiveHeroicLeft => 72.0,
            Preset3DStyle::IsometricLeftDown
            | Preset3DStyle::IsometricRightDown
            | Preset3DStyle::IsometricLeftUp
            | Preset3DStyle::IsometricRightUp => 48.0,
            _ => 36.0,
        }
    }

    fn default_bevel(&self) -> Option<BevelType> {
        match self {
            Preset3DStyle::None => None,
            Preset3DStyle::PerspectiveHeroicRight | Preset3DStyle::PerspectiveHeroicLeft => {
                Some(BevelType::Circle)
            }
            Preset3DStyle::IsometricLeftDown
            | Preset3DStyle::IsometricRightDown
            | Preset3DStyle::IsometricLeftUp
            | Preset3DStyle::IsometricRightUp => Some(BevelType::Angle),
            _ => None,
        }
    }
}

/// Horizontal alignment for a text effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WordArtAlign {
    /// Align left
    #[default]
    Left,
    /// Center text
    Center,
    /// Align right
    Right,
    /// Stretch to fill width
    Stretch,
}

/// The text-effect payload of a text-effect shape.
///
/// This is Publisher's `pbTextEffect`, covering both WordArt (a warped preset)
/// and text on a path. The text fill and outline live on the enclosing
/// [`Shape`] as [`Shape::fill`] / [`Shape::line`]. When [`TextEffectData::transform`]
/// is [`TextTransform::Path`], the `path_*` fields drive how the text follows
/// the path.
#[derive(Debug, Clone, PartialEq)]
pub struct TextEffectData {
    /// The text content
    pub text: String,
    /// Character styling
    pub style: CharacterStyle,
    /// Text transformation (preset warp, custom path, or free warp)
    pub transform: TextTransform,
    /// 3D rotation
    pub rotation_3d: Rotation3D,
    /// 3D extrusion
    pub extrusion: Option<Extrusion3D>,
    /// Letter spacing adjustment (0 = normal, positive = wider)
    pub letter_spacing: f64,
    /// Line spacing for multi-line text
    pub line_spacing: f64,
    /// Horizontal alignment
    pub align: WordArtAlign,
    /// Whether letters are the same height
    pub same_letter_heights: bool,
    /// Whether text is vertical
    pub vertical: bool,
    /// Starting offset along the path (0.0 to 1.0), for text on a path
    pub path_start_offset: f64,
    /// Text alignment along the path, for text on a path
    pub path_align: TextPathAlign,
    /// Spacing method along the path, for text on a path
    pub path_spacing: TextPathSpacing,
    /// Whether to flip the text to the other side of the path
    pub path_flip: bool,
}

impl TextEffectData {
    /// Create a new plain text effect.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            style: CharacterStyle::default(),
            transform: TextTransform::None,
            rotation_3d: Rotation3D::default(),
            extrusion: None,
            letter_spacing: 0.0,
            line_spacing: 1.0,
            align: WordArtAlign::Center,
            same_letter_heights: false,
            vertical: false,
            path_start_offset: 0.0,
            path_align: TextPathAlign::Left,
            path_spacing: TextPathSpacing::Auto,
            path_flip: false,
        }
    }

    /// Create a text effect with a preset warp style.
    pub fn with_preset(text: impl Into<String>, preset: WordArtPreset) -> Self {
        let mut data = Self::new(text);
        data.transform = TextTransform::Preset(preset);
        data
    }

    /// Create a text effect that follows a custom path.
    pub fn on_path(text: impl Into<String>, path: Path) -> Self {
        let mut data = Self::new(text);
        data.transform = TextTransform::Path(path);
        data.path_align = TextPathAlign::Left;
        data.path_spacing = TextPathSpacing::Auto;
        data
    }

    /// Set 3D rotation.
    pub fn with_rotation_3d(mut self, rotation: Rotation3D) -> Self {
        self.rotation_3d = rotation;
        self
    }

    /// Add 3D extrusion.
    pub fn with_extrusion(mut self, depth: f64) -> Self {
        self.extrusion = Some(Extrusion3D {
            depth,
            ..Default::default()
        });
        self
    }

    /// Set the font.
    pub fn with_font(mut self, family: impl Into<String>, size: f64) -> Self {
        self.style.font.family = family.into();
        self.style.font_size = size;
        self
    }

    /// Make the text bold.
    pub fn bold(mut self) -> Self {
        self.style.font.weight = FontWeight::Bold;
        self
    }

    /// Make the text italic.
    pub fn italic(mut self) -> Self {
        self.style.font.style = FontStyle::Italic;
        self
    }
}

/// The group-specific payload of a group shape: its child shapes.
///
/// `bounds` is the group's own bounding box, kept in sync with its members by
/// [`GroupData::recalculate_bounds`] and mirrored onto the enclosing
/// [`Shape`]'s common bounds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GroupData {
    /// Child shapes contained in the group.
    pub children: Vec<Shape>,
    /// The group's bounding box (encloses all children).
    pub bounds: Rect,
}

impl GroupData {
    /// Create a new empty group.
    pub fn new() -> Self {
        Self {
            children: Vec::new(),
            bounds: Rect::default(),
        }
    }

    /// Create a group from a list of shapes.
    pub fn from_shapes(shapes: Vec<Shape>) -> Self {
        let mut group = Self::new();
        for shape in shapes {
            group.add(shape);
        }
        group.recalculate_bounds();
        group
    }

    /// Add a shape to the group.
    pub fn add(&mut self, shape: Shape) {
        self.children.push(shape);
    }

    /// Remove a shape from the group by ID.
    pub fn remove(&mut self, id: ObjectId) -> Option<Shape> {
        if let Some(index) = self.children.iter().position(|s| s.id() == id) {
            Some(self.children.remove(index))
        } else {
            None
        }
    }

    /// Get the number of shapes in the group.
    pub fn len(&self) -> usize {
        self.children.len()
    }

    /// Check if the group is empty.
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Recalculate the group's bounding box from its children.
    pub fn recalculate_bounds(&mut self) {
        if self.children.is_empty() {
            self.bounds = Rect::default();
            return;
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;

        for child in &self.children {
            let bounds = child.bounds();
            min_x = min_x.min(bounds.x());
            min_y = min_y.min(bounds.y());
            max_x = max_x.max(bounds.right());
            max_y = max_y.max(bounds.bottom());
        }

        self.bounds = Rect::new(min_x, min_y, max_x - min_x, max_y - min_y);
    }

    /// Get an iterator over the children.
    pub fn iter(&self) -> impl Iterator<Item = &Shape> {
        self.children.iter()
    }

    /// Get a mutable iterator over the children.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Shape> {
        self.children.iter_mut()
    }

    /// Flatten the group into its component shapes.
    pub fn ungroup(self) -> Vec<Shape> {
        self.children
    }

    /// Find a shape by ID within the group (recursively).
    pub fn find(&self, id: ObjectId) -> Option<&Shape> {
        for child in &self.children {
            if let Some(found) = child.find(id) {
                return Some(found);
            }
        }
        None
    }

    /// Find a shape by ID within the group (recursively, mutable).
    pub fn find_mut(&mut self, id: ObjectId) -> Option<&mut Shape> {
        for child in &mut self.children {
            if let Some(found) = child.find_mut(id) {
                return Some(found);
            }
        }
        None
    }

    /// Count all shapes recursively (including nested groups).
    pub fn total_shape_count(&self) -> usize {
        let mut count = 0;
        for child in &self.children {
            count += 1;
            if let ShapeKind::Group(group) = &child.kind {
                count += group.total_shape_count();
            }
        }
        count
    }
}

/// The lightweight shape discriminator, mirroring Publisher's `PbShapeType`.
///
/// This names the kind of shape without carrying its payload, so code can
/// branch on the type cheaply. The heavy payload lives in [`ShapeKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ShapeType {
    /// A vector auto shape (rectangle, ellipse, polygon, star or path).
    AutoShape,
    /// A text frame.
    TextFrame,
    /// A table.
    Table,
    /// A picture.
    Picture,
    /// A text effect (WordArt / text on a path).
    TextEffect,
    /// A line or connector.
    Line,
    /// A group of shapes.
    Group,
}

/// The type-specific payload of a [`Shape`].
#[derive(Debug, Clone, PartialEq)]
#[allow(clippy::large_enum_variant)] // We optimize for direct access over memory density
pub enum ShapeKind {
    /// A vector auto shape: primitive outline or custom path.
    AutoShape(AutoShape),
    /// A text frame containing styled text.
    TextFrame(TextFrameData),
    /// A table with rows and columns.
    Table(TableData),
    /// A picture.
    Picture(PictureData),
    /// A text effect (WordArt and text on a path).
    TextEffect(TextEffectData),
    /// A line or connector.
    Line(LineData),
    /// A group of shapes.
    Group(GroupData),
}

impl ShapeKind {
    /// The lightweight discriminator for this payload.
    pub fn shape_type(&self) -> ShapeType {
        match self {
            ShapeKind::AutoShape(_) => ShapeType::AutoShape,
            ShapeKind::TextFrame(_) => ShapeType::TextFrame,
            ShapeKind::Table(_) => ShapeType::Table,
            ShapeKind::Picture(_) => ShapeType::Picture,
            ShapeKind::TextEffect(_) => ShapeType::TextEffect,
            ShapeKind::Line(_) => ShapeType::Line,
            ShapeKind::Group(_) => ShapeType::Group,
        }
    }
}

/// A shape on a page: placement, styling, and a type-specific payload.
///
/// This is Publisher's single shape class. The placement and bookkeeping live
/// in [`ShapeCommon`]; the fill, line and effects apply to every kind; and the
/// type-specific data lives in [`ShapeKind`].
#[derive(Debug, Clone, PartialEq)]
pub struct Shape {
    /// Placement and bookkeeping common to all shapes.
    pub common: ShapeCommon,
    /// Fill of the shape.
    pub fill: Fill,
    /// Outline of the shape (`None` for no outline).
    pub line: Option<Stroke>,
    /// Visual effects (shadow, glow, bevel, 3D, …).
    pub effects: Effects,
    /// Overall opacity (0.0 to 1.0).
    pub opacity: f64,
    /// The type-specific payload.
    pub kind: ShapeKind,
}

impl Shape {
    /// Create a shape at the given bounds with the given payload.
    pub fn new(bounds: Rect, kind: ShapeKind) -> Self {
        Self {
            common: ShapeCommon::new(bounds),
            fill: Fill::default(),
            line: None,
            effects: Effects::new(),
            opacity: 1.0,
            kind,
        }
    }

    /// Create a rectangle auto shape.
    pub fn rectangle(bounds: Rect) -> Self {
        Self::new(
            bounds,
            ShapeKind::AutoShape(AutoShape::new(ShapeGeometry::Rectangle {
                corner_radius: 0.0,
            })),
        )
    }

    /// Create a rounded-rectangle auto shape.
    pub fn rounded_rectangle(bounds: Rect, corner_radius: f64) -> Self {
        Self::new(
            bounds,
            ShapeKind::AutoShape(AutoShape::new(ShapeGeometry::Rectangle { corner_radius })),
        )
    }

    /// Create an ellipse auto shape.
    pub fn ellipse(bounds: Rect) -> Self {
        Self::new(
            bounds,
            ShapeKind::AutoShape(AutoShape::new(ShapeGeometry::Ellipse)),
        )
    }

    /// Create a circle auto shape centred at `center` with the given radius.
    pub fn circle(center: Point, radius: f64) -> Self {
        let bounds = Rect::new(
            center.x - radius,
            center.y - radius,
            radius * 2.0,
            radius * 2.0,
        );
        Self::ellipse(bounds)
    }

    /// Create a regular-polygon auto shape.
    pub fn polygon(bounds: Rect, sides: u32) -> Self {
        Self::new(
            bounds,
            ShapeKind::AutoShape(AutoShape::new(ShapeGeometry::Polygon { sides })),
        )
    }

    /// Create a triangle auto shape.
    pub fn triangle(bounds: Rect) -> Self {
        Self::polygon(bounds, 3)
    }

    /// Create a star auto shape.
    pub fn star(bounds: Rect, points: u32, inner_ratio: f64) -> Self {
        Self::new(
            bounds,
            ShapeKind::AutoShape(AutoShape::new(ShapeGeometry::Star {
                points,
                inner_ratio,
            })),
        )
    }

    /// Create an auto shape from a custom path.
    pub fn from_path(bounds: Rect, path: Path) -> Self {
        Self::new(
            bounds,
            ShapeKind::AutoShape(AutoShape::new(ShapeGeometry::Path(path))),
        )
    }

    /// Create an empty text frame.
    pub fn text_frame(bounds: Rect) -> Self {
        Self::new(bounds, ShapeKind::TextFrame(TextFrameData::new()))
    }

    /// Create a text frame with initial text.
    pub fn text_frame_with_text(bounds: Rect, text: &str) -> Self {
        let mut data = TextFrameData::new();
        data.content = StyledText::plain(text);
        Self::new(bounds, ShapeKind::TextFrame(data))
    }

    /// Create a picture placeholder.
    pub fn picture(bounds: Rect) -> Self {
        Self::new(bounds, ShapeKind::Picture(PictureData::new()))
    }

    /// Create a table with the given dimensions.
    pub fn table(bounds: Rect, rows: u32, cols: u32) -> Self {
        Self::new(bounds, ShapeKind::Table(TableData::new(bounds, rows, cols)))
    }

    /// Create a text effect (WordArt / text on a path).
    pub fn text_effect(bounds: Rect, text: impl Into<String>) -> Self {
        Self::new(bounds, ShapeKind::TextEffect(TextEffectData::new(text)))
    }

    /// Create a straight line between two points (with a default stroke).
    pub fn line(start: Point, end: Point) -> Self {
        let min_x = start.x.min(end.x);
        let min_y = start.y.min(end.y);
        let max_x = start.x.max(end.x);
        let max_y = start.y.max(end.y);
        let bounds = Rect::new(min_x, min_y, max_x - min_x, max_y - min_y);
        let mut shape = Self::new(bounds, ShapeKind::Line(LineData::new(start, end)));
        shape.line = Some(Stroke::default());
        shape
    }

    /// Create an empty group.
    pub fn group() -> Self {
        Self::new(Rect::default(), ShapeKind::Group(GroupData::new()))
    }

    /// Create a group from a list of shapes.
    pub fn group_from(children: Vec<Shape>) -> Self {
        let data = GroupData::from_shapes(children);
        let bounds = data.bounds;
        Self::new(bounds, ShapeKind::Group(data))
    }

    /// Set the fill.
    pub fn with_fill(mut self, fill: Fill) -> Self {
        self.fill = fill;
        self
    }

    /// Set the outline.
    pub fn with_line(mut self, line: Stroke) -> Self {
        self.line = Some(line);
        self
    }

    /// Set the effects.
    pub fn with_effects(mut self, effects: Effects) -> Self {
        self.effects = effects;
        self
    }

    /// Set the opacity (clamped to 0.0..=1.0).
    pub fn with_opacity(mut self, opacity: f64) -> Self {
        self.opacity = opacity.clamp(0.0, 1.0);
        self
    }

    /// The unique ID of this shape.
    pub fn id(&self) -> ObjectId {
        self.common.id
    }

    /// The bounding rectangle of this shape.
    pub fn bounds(&self) -> Rect {
        self.common.bounds
    }

    /// The position (top-left corner) of this shape.
    pub fn position(&self) -> Point {
        self.common.bounds.origin
    }

    /// The centre point of this shape.
    pub fn center(&self) -> Point {
        self.common.bounds.center()
    }

    /// The full transform (flips and rotation about the centre, then the
    /// extra transform), delegated to [`ShapeCommon::full_transform`].
    pub fn full_transform(&self) -> Transform2D {
        self.common.full_transform()
    }

    /// The lightweight discriminator for this shape.
    pub fn shape_type(&self) -> ShapeType {
        self.kind.shape_type()
    }

    /// Set the position (top-left corner); the geometry moves with it.
    pub fn set_position(&mut self, pos: Point) {
        let b = self.bounds();
        self.set_bounds(Rect::new(pos.x, pos.y, b.size.width, b.size.height));
    }

    /// Set the position by x,y coordinates.
    pub fn set_position_xy(&mut self, x: f64, y: f64) {
        self.set_position(Point::new(x, y));
    }

    /// Set the size, keeping the top-left; the geometry scales with it.
    pub fn set_size(&mut self, width: f64, height: f64) {
        let b = self.bounds();
        self.set_bounds(Rect::new(b.origin.x, b.origin.y, width, height));
    }

    /// Move by the given offset (the geometry moves with it).
    pub fn translate(&mut self, dx: f64, dy: f64) {
        let b = self.bounds();
        self.set_bounds(Rect::new(
            b.origin.x + dx,
            b.origin.y + dy,
            b.size.width,
            b.size.height,
        ));
    }

    /// Move and resize the shape to `new`. Geometry kept in page coordinates
    /// moves and scales along: line end and control points, custom path
    /// points, text-effect paths, and the members of a group.
    pub fn set_bounds(&mut self, new: Rect) {
        let old = self.bounds();
        // A zero extent (e.g. a horizontal line's height) keeps its offset
        let sx = if old.size.width.abs() > 1e-9 {
            new.size.width / old.size.width
        } else {
            1.0
        };
        let sy = if old.size.height.abs() > 1e-9 {
            new.size.height / old.size.height
        } else {
            1.0
        };
        let map = |p: Point| {
            Point::new(
                new.origin.x + (p.x - old.origin.x) * sx,
                new.origin.y + (p.y - old.origin.y) * sy,
            )
        };
        let map_path = |path: &mut Path| {
            for cmd in &mut path.commands {
                match cmd {
                    PathCommand::MoveTo(p) | PathCommand::LineTo(p) => *p = map(*p),
                    PathCommand::QuadTo { control, end } => {
                        *control = map(*control);
                        *end = map(*end);
                    }
                    PathCommand::CubicTo {
                        control1,
                        control2,
                        end,
                    } => {
                        *control1 = map(*control1);
                        *control2 = map(*control2);
                        *end = map(*end);
                    }
                    PathCommand::Close => {}
                }
            }
        };
        match &mut self.kind {
            ShapeKind::Line(line) => {
                line.start = map(line.start);
                line.end = map(line.end);
                for p in &mut line.control_points {
                    *p = map(*p);
                }
            }
            ShapeKind::AutoShape(auto) => {
                if let ShapeGeometry::Path(path) = &mut auto.geometry {
                    map_path(path);
                }
            }
            ShapeKind::TextEffect(effect) => {
                if let TextTransform::Path(path) = &mut effect.transform {
                    map_path(path);
                }
            }
            ShapeKind::Group(group) => {
                for child in &mut group.children {
                    let b = child.bounds();
                    let tl = map(b.origin);
                    let br = map(Point::new(
                        b.origin.x + b.size.width,
                        b.origin.y + b.size.height,
                    ));
                    child.set_bounds(Rect::new(tl.x, tl.y, br.x - tl.x, br.y - tl.y));
                }
            }
            _ => {}
        }
        self.common.bounds = new;
        if let ShapeKind::Group(group) = &mut self.kind {
            group.bounds = new;
        }
    }

    /// Whether [`Self::flip`] mirrors this shape. Text stays readable, so text
    /// frames, tables and text effects don't flip (as in Publisher). Auto
    /// shapes, pictures and lines do; a group flips if any member does.
    pub fn can_flip(&self) -> bool {
        match &self.kind {
            ShapeKind::AutoShape(_) | ShapeKind::Picture(_) | ShapeKind::Line(_) => true,
            ShapeKind::Group(group) => group.children.iter().any(|c| c.can_flip()),
            _ => false,
        }
    }

    /// Mirror the shape about its centre, left to right (`horizontal`) or top
    /// to bottom. Its rotation is mirrored too, so it looks reflected in
    /// place. Lines mirror their points; groups mirror their members'
    /// positions and flip each member.
    pub fn flip(&mut self, horizontal: bool) {
        let c = self.bounds().center();
        self.flip_about(c, horizontal);
    }

    fn flip_about(&mut self, c: Point, horizontal: bool) {
        let mirror = |p: &mut Point| {
            if horizontal {
                p.x = 2.0 * c.x - p.x;
            } else {
                p.y = 2.0 * c.y - p.y;
            }
        };
        let bounds = self.bounds();
        let mut centre = bounds.center();
        mirror(&mut centre);
        let (dx, dy) = (centre.x - bounds.center().x, centre.y - bounds.center().y);
        match &mut self.kind {
            ShapeKind::Line(line) => {
                for p in std::iter::once(&mut line.start)
                    .chain(std::iter::once(&mut line.end))
                    .chain(line.control_points.iter_mut())
                {
                    mirror(p);
                }
                self.common.bounds.origin.x += dx;
                self.common.bounds.origin.y += dy;
            }
            ShapeKind::Group(group) => {
                for child in &mut group.children {
                    child.flip_about(c, horizontal);
                }
                group.recalculate_bounds();
                self.common.bounds = group.bounds;
            }
            _ => {
                let can_flip = self.can_flip();
                if can_flip {
                    if horizontal {
                        self.common.flip_h = !self.common.flip_h;
                    } else {
                        self.common.flip_v = !self.common.flip_v;
                    }
                    self.common.rotation = (-self.common.rotation).rem_euclid(360.0);
                }
                let mut moved = bounds;
                moved.origin.x += dx;
                moved.origin.y += dy;
                self.set_bounds(moved);
            }
        }
    }

    /// This shape, or a member of it (groups, at any depth), with ID `id`.
    pub fn find(&self, id: ObjectId) -> Option<&Shape> {
        if self.id() == id {
            return Some(self);
        }
        match &self.kind {
            ShapeKind::Group(group) => group.children.iter().find_map(|c| c.find(id)),
            _ => None,
        }
    }

    /// Mutable [`Self::find`].
    pub fn find_mut(&mut self, id: ObjectId) -> Option<&mut Shape> {
        if self.id() == id {
            return Some(self);
        }
        match &mut self.kind {
            ShapeKind::Group(group) => group.children.iter_mut().find_map(|c| c.find_mut(id)),
            _ => None,
        }
    }

    /// Remove the member `id` from inside this group (at any depth).
    pub fn remove_member(&mut self, id: ObjectId) -> Option<Shape> {
        let ShapeKind::Group(group) = &mut self.kind else {
            return None;
        };
        if let Some(index) = group.children.iter().position(|c| c.id() == id) {
            let removed = group.children.remove(index);
            group.recalculate_bounds();
            self.common.bounds = group.bounds;
            return Some(removed);
        }
        let removed = group.children.iter_mut().find_map(|c| c.remove_member(id));
        if removed.is_some() {
            group.recalculate_bounds();
            self.common.bounds = group.bounds;
        }
        removed
    }

    /// Make every group's bounds (at any depth) enclose its members again,
    /// after a member was moved or resized on its own.
    pub fn refresh_group_bounds(&mut self) {
        if let ShapeKind::Group(group) = &mut self.kind {
            for child in &mut group.children {
                child.refresh_group_bounds();
            }
            if !group.children.is_empty() {
                group.recalculate_bounds();
                self.common.bounds = group.bounds;
            }
        }
    }
}

#[cfg(test)]
mod flip_tests {
    use super::*;
    use repub_geometry::{Point, Rect};

    #[test]
    fn rotation_is_about_the_centre() {
        // A 100x100 square at the origin, centre (50,50), rotated 90° CW.
        let mut shape = Shape::rectangle(Rect::new(0.0, 0.0, 100.0, 100.0));
        shape.common.rotation = 90.0;
        let t = shape.common.full_transform();
        // The centre must stay fixed under a centred rotation.
        let c = t.apply(Point::new(50.0, 50.0));
        assert!(
            (c.x - 50.0).abs() < 1e-6 && (c.y - 50.0).abs() < 1e-6,
            "centre fixed, got {c:?}"
        );
        // The top-left corner (0,0) rotates 90° CW about (50,50) to (100,0).
        let tl = t.apply(Point::new(0.0, 0.0));
        assert!(
            (tl.x - 100.0).abs() < 1e-6 && (tl.y - 0.0).abs() < 1e-6,
            "TL -> (100,0), got {tl:?}"
        );
    }

    #[test]
    fn flipping_mirrors_in_place() {
        let mut shape = Shape::rectangle(Rect::new(10.0, 10.0, 40.0, 20.0));
        shape.common.rotation = 30.0;
        shape.flip(true);
        assert!(shape.common.flip_h && !shape.common.flip_v);
        assert_eq!(shape.common.rotation, 330.0, "rotation mirrored");
        assert_eq!(
            shape.bounds(),
            Rect::new(10.0, 10.0, 40.0, 20.0),
            "stays put"
        );
        shape.flip(true);
        assert!(
            !shape.common.flip_h && shape.common.rotation == 30.0,
            "twice is the original"
        );

        let mut line = Shape::line(Point::new(0.0, 0.0), Point::new(100.0, 50.0));
        line.flip(false);
        match &line.kind {
            ShapeKind::Line(l) => assert_eq!(
                (l.start, l.end),
                (Point::new(0.0, 50.0), Point::new(100.0, 0.0))
            ),
            _ => unreachable!(),
        }
        assert!(!line.common.flip_v, "lines mirror their points, not a flag");
    }

    #[test]
    fn groups_mirror_their_members() {
        let a = Shape::rectangle(Rect::new(0.0, 0.0, 10.0, 10.0));
        let b = Shape::text_frame(Rect::new(90.0, 0.0, 10.0, 10.0));
        let mut group = Shape::group_from(vec![a, b]);
        assert!(group.can_flip());
        group.flip(true);
        let ShapeKind::Group(g) = &group.kind else {
            unreachable!()
        };
        assert_eq!(g.children[0].bounds().origin.x, 90.0, "shape moved across");
        assert!(g.children[0].common.flip_h);
        assert_eq!(
            g.children[1].bounds().origin.x,
            0.0,
            "text box moved across"
        );
        assert!(!g.children[1].common.flip_h, "but its text isn't mirrored");
        assert_eq!(group.bounds(), Rect::new(0.0, 0.0, 100.0, 10.0));
    }

    #[test]
    fn moving_and_resizing_carries_geometry_along() {
        let mut line = Shape::line(Point::new(10.0, 10.0), Point::new(30.0, 10.0));
        line.set_position(Point::new(110.0, 60.0));
        match &line.kind {
            ShapeKind::Line(l) => {
                assert_eq!(l.start, Point::new(110.0, 60.0));
                assert_eq!(l.end, Point::new(130.0, 60.0));
            }
            _ => unreachable!(),
        }

        let a = Shape::rectangle(Rect::new(0.0, 0.0, 10.0, 10.0));
        let b = Shape::rectangle(Rect::new(30.0, 30.0, 10.0, 10.0));
        let mut group = Shape::group_from(vec![a, b]);
        group.set_bounds(Rect::new(100.0, 100.0, 80.0, 80.0)); // twice the size
        let ShapeKind::Group(g) = &group.kind else {
            unreachable!()
        };
        assert_eq!(g.children[0].bounds(), Rect::new(100.0, 100.0, 20.0, 20.0));
        assert_eq!(g.children[1].bounds(), Rect::new(160.0, 160.0, 20.0, 20.0));
    }

    #[test]
    fn table_rows_and_columns_go_in_and_out() {
        let mut shape = Shape::table(Rect::new(0.0, 0.0, 100.0, 40.0), 2, 2);
        let ShapeKind::Table(t) = &mut shape.kind else {
            unreachable!()
        };
        for (r, c, s) in [(0, 0, "a"), (0, 1, "b"), (1, 0, "c"), (1, 1, "d")] {
            t.set_cell_text(r, c, s);
        }
        t.insert_row_at(1);
        assert_eq!(t.rows, 3);
        assert_eq!(t.cell(1, 0).unwrap().plain_text(), "");
        assert!(t.remove_row(1));
        assert_eq!(t.rows, 2);
        assert_eq!(t.cell(1, 0).unwrap().plain_text(), "c");
    }
}
