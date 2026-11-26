//! Microsoft Publisher (.pub) file export.
//!
//! This module writes Publisher files using the OLE Compound File Binary (CFB)
//! format, mirroring the structure used by Microsoft Publisher.
//!
//! ## Format Overview
//!
//! Publisher files are OLE Compound Documents containing several streams:
//! - `Contents` - Main document data with page/shape structure
//! - `Escher/EscherStm` - Drawing/shape data (Microsoft's Escher format)
//! - `Escher/EscherDelayStm` - Embedded images
//! - `Quill/QuillSub/CONTENTS` - Text content with formatting
//!
//! ## Units
//!
//! Publisher internally uses EMUs (English Metric Units) where:
//! - 914,400 EMUs = 1 inch
//! - 12,700 EMUs = 1 point

use std::io::{Read, Seek, Write};
use std::path::Path;

use cfb::CompoundFile;
use repub_model::document::Document;

use crate::error::{FormatError, FormatResult};
use crate::escher;
use crate::model::{ImageSource, LayerObjectExt, Object, ShapeType, Table};

// Constants

/// EMUs per point
const EMUS_PER_POINT: i32 = 12700;

/// Convert points to EMUs
fn points_to_emus(points: f64) -> i32 {
    (points * EMUS_PER_POINT as f64).round() as i32
}

// Block types (matching pub_import.rs)

mod block_type {
    pub const ZERO_BYTE_1: u8 = 0x08;
    pub const ZERO_BYTE_2: u8 = 0x78;
    pub const TWO_BYTE_1: u8 = 0x18;
    pub const FOUR_BYTE_1: u8 = 0x20;
    pub const FOUR_BYTE_2: u8 = 0x70;
    pub const CONTAINER: u8 = 0x88;
    pub const TRAILER: u8 = 0x90;
    pub const VARIABLE: u8 = 0xA0;
    pub const STRING: u8 = 0xC0;
}

mod block_id {
    pub const DOCUMENT_WIDTH: u8 = 0x01;
    pub const DOCUMENT_HEIGHT: u8 = 0x02;
    pub const CHUNK_TYPE: u8 = 0x02;
    pub const CHUNK_OFFSET: u8 = 0x04;
    pub const CHUNK_PARENT_SEQNUM: u8 = 0x05;
    pub const SHAPE_WIDTH: u8 = 0xAA;
    pub const SHAPE_HEIGHT: u8 = 0xAB;
    pub const TEXT_ID: u8 = 0x27;
    pub const TABLE_NUM_ROWS: u8 = 0x66;
    pub const TABLE_NUM_COLS: u8 = 0x67;
    pub const TABLE_CELLS_SEQNUM: u8 = 0x6B;
    pub const TABLE_ROWCOL_ARRAY: u8 = 0x6D;
    pub const TABLE_ROWCOL_SIZE: u8 = 0x02;
}

// Chunk type markers
mod chunk_type {
    pub const DOCUMENT: u8 = 0x44;
    pub const PAGE: u8 = 0x43;
    pub const SHAPE: u8 = 0x01;
    pub const GROUP: u8 = 0x30;
    pub const TABLE: u8 = 0x10;
    pub const CELLS: u8 = 0x63;
}

// Export Options

/// Options for exporting Publisher files.
#[derive(Debug, Clone)]
pub struct PubExportOptions {
    /// Publisher version to target
    pub version: u16,
}

impl Default for PubExportOptions {
    fn default() -> Self {
        Self { version: 2010 }
    }
}

// Main Export Function

/// Export a document to Publisher format.
pub fn export<P: AsRef<Path>>(
    document: &Document,
    path: P,
    options: &PubExportOptions,
) -> FormatResult<()> {
    let file = std::fs::File::create(path.as_ref())?;
    export_to_writer(document, file, options)
}

/// Export a document to a writer.
pub fn export_to_writer<W: Read + Write + Seek>(
    document: &Document,
    writer: W,
    options: &PubExportOptions,
) -> FormatResult<()> {
    let mut exporter = PubExporter::new(document, options);
    exporter.export(writer)
}

/// Export a document to Publisher format as in-memory bytes.
pub fn export_bytes(document: &Document, options: &PubExportOptions) -> FormatResult<Vec<u8>> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    export_to_writer(document, &mut buffer, options)?;
    Ok(buffer.into_inner())
}

// Block Writer Helpers

/// Write a block with 0 bytes of data
fn write_zero_block(data: &mut Vec<u8>, id: u8) {
    data.push(id);
    data.push(block_type::ZERO_BYTE_1);
}

/// Write a block with 2-byte value
fn write_u16_block(data: &mut Vec<u8>, id: u8, value: u16) {
    data.push(id);
    data.push(block_type::TWO_BYTE_1);
    data.extend_from_slice(&value.to_le_bytes());
}

/// Write a block with 4-byte value
fn write_u32_block(data: &mut Vec<u8>, id: u8, value: u32) {
    data.push(id);
    data.push(block_type::FOUR_BYTE_1);
    data.extend_from_slice(&value.to_le_bytes());
}

/// Write a container block start
fn write_container_start(data: &mut Vec<u8>, id: u8) -> usize {
    data.push(id);
    data.push(block_type::CONTAINER);
    let len_pos = data.len();
    data.extend_from_slice(&0u32.to_le_bytes());
    len_pos
}

/// Patch container length (length includes the 4-byte length field)
fn patch_container_length(data: &mut [u8], len_pos: usize) {
    let content_len = (data.len() - len_pos) as u32;
    let bytes = content_len.to_le_bytes();
    data[len_pos..len_pos + 4].copy_from_slice(&bytes);
}

/// Write an 8-byte Escher record header: `verAndInstance`, `recordType`,
/// `length`. `version` is 0 for atoms (the caller passes the instance); for
/// containers use [`escher_container_start`].
fn write_escher_header(data: &mut Vec<u8>, record_type: u16, instance: u16, length: u32) {
    let ver_inst = instance << 4; // version 0 (atom), instance in the high 12 bits
    data.extend_from_slice(&ver_inst.to_le_bytes());
    data.extend_from_slice(&record_type.to_le_bytes());
    data.extend_from_slice(&length.to_le_bytes());
}

/// Begin an Escher container record (version nibble 0xF). Returns the offset of
/// its length field for [`escher_patch_length`].
fn escher_container_start(data: &mut Vec<u8>, record_type: u16) -> usize {
    data.extend_from_slice(&0x000Fu16.to_le_bytes()); // version 0xF, instance 0
    data.extend_from_slice(&record_type.to_le_bytes());
    let len_pos = data.len();
    data.extend_from_slice(&0u32.to_le_bytes()); // length placeholder
    len_pos
}

/// Patch an Escher container's length to cover everything written after its
/// 8-byte header.
fn escher_patch_length(data: &mut [u8], len_pos: usize) {
    let end = data.len();
    let len = (end - (len_pos + 4)) as u32;
    data[len_pos..len_pos + 4].copy_from_slice(&len.to_le_bytes());
}

/// Encode an RGB colour as the little-endian u32 the importer's `bgr_to_color`
/// expects: byte 0 = red, byte 1 = green, byte 2 = blue.
fn rgb_to_bgr(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

impl ImageBlip {
    /// Build a blip from embedded image bytes, choosing the Escher record type
    /// from the MIME type. Returns `None` for formats we do not write.
    fn from_bytes(mime_type: &str, bytes: Vec<u8>) -> Option<Self> {
        match mime_type {
            "image/png" => Some(ImageBlip {
                blip_type: escher::ESCHER_BLIP_PNG,
                instance: 0x6E0,
                bytes,
            }),
            "image/jpeg" => Some(ImageBlip {
                blip_type: escher::ESCHER_BLIP_JPEG,
                instance: 0x46A,
                bytes,
            }),
            _ => None,
        }
    }
}

/// Total byte length of an image's BLIP record in EscherDelayStm: the 8-byte
/// Escher header plus the 16-byte UID, 1 tag byte, and the image bytes.
fn blip_record_len(image: &ImageBlip) -> usize {
    8 + 16 + 1 + image.bytes.len()
}

/// Write a 36-byte FBSE (blip store entry) record for `image`. `fo_delay` is
/// the offset of the image's BLIP record in EscherDelayStm; `blip_total` is
/// that record's full byte length.
fn write_fbse(data: &mut Vec<u8>, image: &ImageBlip, fo_delay: u32, blip_total: u32) {
    // The FBSE header instance is the Windows blip type; the importer does not
    // rely on it, but Publisher sets it.
    write_escher_header(data, escher::ESCHER_BSE, image.blip_type & 0xFF, 36);
    let win_type = match image.blip_type {
        t if t == escher::ESCHER_BLIP_PNG => 6u8,  // msoblipPNG
        t if t == escher::ESCHER_BLIP_JPEG => 5u8, // msoblipJPEG
        _ => 0,
    };
    data.push(win_type); // btWin
    data.push(win_type); // btMacOS
    data.extend_from_slice(&[0u8; 16]); // rgbUid
    data.extend_from_slice(&0u16.to_le_bytes()); // tag
    data.extend_from_slice(&blip_total.to_le_bytes()); // size
    data.extend_from_slice(&1u32.to_le_bytes()); // cRef
    data.extend_from_slice(&fo_delay.to_le_bytes()); // foDelay
    data.push(0); // usage
    data.push(0); // cbName
    data.extend_from_slice(&0u16.to_le_bytes()); // unused
}

/// Flatten a `StyledText` into the CR-joined text story (one CR after each
/// paragraph), the per-run character-formatting list used for FDPC, and the
/// per-paragraph style list used for FDPP.
fn styled_text_to_block(
    content: &repub_model::text::StyledText,
) -> (String, Vec<StyleRun>, Vec<ParaStyleSpec>) {
    use repub_model::styles::{FontStyle, FontWeight, TextAlignment, UnderlineStyle};
    use repub_model::text::InlineElement;

    let mut text = String::new();
    let mut runs: Vec<StyleRun> = Vec::new();
    let mut paras: Vec<ParaStyleSpec> = Vec::new();

    for para in &content.paragraphs {
        for el in &para.elements {
            let (run_text, style) = match el {
                InlineElement::Text(r) => (r.text.as_str(), &r.style),
                InlineElement::Link(h) => (h.text.as_str(), &h.style),
                // Fields and other inline elements have no plain text to emit.
                _ => continue,
            };
            if run_text.is_empty() {
                continue;
            }
            let len = run_text.encode_utf16().count();
            text.push_str(run_text);
            runs.push(StyleRun {
                len,
                bold: style.font.weight == FontWeight::Bold,
                italic: style.font.style == FontStyle::Italic,
                underline: match style.underline {
                    UnderlineStyle::None => 0,
                    UnderlineStyle::Double => 3,
                    _ => 1,
                },
                size_pt: style.font_size,
                font: style.font.family.clone(),
                color: run_color(style),
            });
        }
        // Paragraph mark (CR), carried as an unstyled run.
        text.push('\r');
        runs.push(StyleRun {
            len: 1,
            bold: false,
            italic: false,
            underline: 0,
            size_pt: 0.0,
            font: String::new(),
            color: None,
        });
        paras.push(ParaStyleSpec {
            alignment: match para.style.alignment {
                TextAlignment::Right => 1,
                TextAlignment::Center => 2,
                TextAlignment::Justify => 6,
                _ => 0,
            },
            space_before: para.style.space_before,
            space_after: para.style.space_after,
        });
    }

    (text, runs, paras)
}

/// Build an FDPP paragraph-style record: a 4-byte length followed by the
/// paragraph formatting blocks the importer reads (alignment, space before/
/// after).
fn paragraph_style_record(style: &ParaStyleSpec) -> Vec<u8> {
    const ALIGNMENT: u8 = 0x04;
    const SPACE_BEFORE: u8 = 0x12;
    const SPACE_AFTER: u8 = 0x13;
    const TWO_BYTE: u8 = 0x10;
    const FOUR_BYTE: u8 = 0x20;

    let mut body = Vec::new();
    if style.alignment != 0 {
        body.push(ALIGNMENT);
        body.push(TWO_BYTE);
        body.extend_from_slice(&(style.alignment as u16).to_le_bytes());
    }
    if style.space_before != 0.0 {
        body.push(SPACE_BEFORE);
        body.push(FOUR_BYTE);
        body.extend_from_slice(&(points_to_emus(style.space_before) as u32).to_le_bytes());
    }
    if style.space_after != 0.0 {
        body.push(SPACE_AFTER);
        body.push(FOUR_BYTE);
        body.extend_from_slice(&(points_to_emus(style.space_after) as u32).to_le_bytes());
    }

    let mut rec = Vec::new();
    rec.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
    rec.extend_from_slice(&body);
    rec
}

/// A run's colour as an RGB triple, or `None` when it is the default black.
fn run_color(style: &repub_model::styles::CharacterStyle) -> Option<Rgb> {
    let (r, g, b, _) = style.color.to_rgba8();
    if (r, g, b) == (0, 0, 0) {
        None
    } else {
        Some((r, g, b))
    }
}

/// Flatten a table cell's content into inline text and character-formatting
/// runs, with no paragraph mark (the caller adds the single terminating CR).
fn cell_text_runs(content: &repub_model::text::StyledText) -> (String, Vec<StyleRun>) {
    use repub_model::styles::{FontStyle, FontWeight, UnderlineStyle};
    use repub_model::text::InlineElement;

    let mut text = String::new();
    let mut runs: Vec<StyleRun> = Vec::new();
    for para in &content.paragraphs {
        for el in &para.elements {
            let (run_text, style) = match el {
                InlineElement::Text(r) => (r.text.as_str(), &r.style),
                InlineElement::Link(h) => (h.text.as_str(), &h.style),
                _ => continue,
            };
            if run_text.is_empty() {
                continue;
            }
            text.push_str(run_text);
            runs.push(StyleRun {
                len: run_text.encode_utf16().count(),
                bold: style.font.weight == FontWeight::Bold,
                italic: style.font.style == FontStyle::Italic,
                underline: match style.underline {
                    UnderlineStyle::None => 0,
                    UnderlineStyle::Double => 3,
                    _ => 1,
                },
                size_pt: style.font_size,
                font: style.font.family.clone(),
                color: run_color(style),
            });
        }
    }
    (text, runs)
}

/// Encode a model `Path` (page-space coordinates) as Escher freeform geometry:
/// vertices in 0-21600 shape space and matching segment codes. Returns `None`
/// when the path or bounds are degenerate.
fn encode_custom_path(
    path: &repub_model::geometry::Path,
    bounds: repub_model::Rect,
) -> Option<CustomPath> {
    use repub_model::geometry::PathCommand;

    let w = bounds.width();
    let h = bounds.height();
    if w <= 0.0 || h <= 0.0 || path.commands.is_empty() {
        return None;
    }

    // Map a page-space point into 0-21600 shape space.
    let to_shape = |p: repub_model::Point| -> (i32, i32) {
        let x = ((p.x - bounds.origin.x) / w * 21600.0).round() as i32;
        let y = ((p.y - bounds.origin.y) / h * 21600.0).round() as i32;
        (x, y)
    };

    let mut vertices: Vec<(i32, i32)> = Vec::new();
    let mut segments: Vec<u16> = Vec::new();

    // Track the current point so a QuadTo can be promoted to a cubic.
    let mut current = repub_model::Point::new(bounds.origin.x, bounds.origin.y);
    for cmd in &path.commands {
        match cmd {
            PathCommand::MoveTo(p) => {
                vertices.push(to_shape(*p));
                segments.push(0x0001); // msopathMoveTo
                current = *p;
            }
            PathCommand::LineTo(p) => {
                vertices.push(to_shape(*p));
                segments.push(0x0000); // msopathLineTo
                current = *p;
            }
            PathCommand::CubicTo { control1, control2, end } => {
                vertices.push(to_shape(*control1));
                vertices.push(to_shape(*control2));
                vertices.push(to_shape(*end));
                segments.push(0x0020); // msopathCurveTo (consumes 3 vertices)
                current = *end;
            }
            PathCommand::QuadTo { control, end } => {
                // Promote the quadratic to a cubic: c1 = p0 + 2/3(ctrl - p0),
                // c2 = end + 2/3(ctrl - end).
                let c1 = repub_model::Point::new(
                    current.x + 2.0 / 3.0 * (control.x - current.x),
                    current.y + 2.0 / 3.0 * (control.y - current.y),
                );
                let c2 = repub_model::Point::new(
                    end.x + 2.0 / 3.0 * (control.x - end.x),
                    end.y + 2.0 / 3.0 * (control.y - end.y),
                );
                vertices.push(to_shape(c1));
                vertices.push(to_shape(c2));
                vertices.push(to_shape(*end));
                segments.push(0x0020);
                current = *end;
            }
            PathCommand::Close => {
                segments.push(0x0080); // msopathClose
            }
        }
    }

    if vertices.is_empty() {
        return None;
    }
    Some(CustomPath { vertices, segments })
}

/// Count the shape chunks an object produces: itself plus, for a group, every
/// descendant, and for a table its extra CELLS chunk (each gets its own
/// Contents chunk and sequence number).
fn count_shape_chunks(obj: &Object) -> usize {
    match obj {
        Object::Group(group) => {
            1 + group.children.iter().map(count_shape_chunks).sum::<usize>()
        }
        // A table is its own chunk plus a following CELLS chunk.
        Object::Table(_) => 2,
        _ => 1,
    }
}

/// Map a line-ending style to the MSOLINEEND arrow code the importer decodes
/// (the inverse of its `convert_arrow_style`).
fn arrow_code(style: repub_model::shape::LineEndingStyle) -> u32 {
    use repub_model::shape::LineEndingStyle as S;
    match style {
        S::None => 0,
        S::Arrow => 1,
        S::StealthArrow => 2,
        S::Diamond => 3,
        S::Circle => 4,
        S::OpenArrow => 5,
        // The importer has no distinct square arrow head; approximate with
        // the diamond head, which is the closest filled marker.
        S::Square => 3,
    }
}

// Exporter Implementation

struct PubExporter<'a> {
    document: &'a Document,
    options: &'a PubExportOptions,
    next_seq_num: u32,
    text_blocks: Vec<TextBlock>,
    /// Embedded raster images to write as Escher BLIP records, in blip-index
    /// order (the blip index is one-based: entry `i` has index `i + 1`).
    images: Vec<ImageBlip>,
    /// Per-shape Escher geometry/style, keyed by Contents seq_num, built during
    /// the Contents pass and serialized into `EscherStm`.
    escher_shapes: Vec<EscherShapeInfo>,
}

/// A raster image to embed as an Escher BLIP.
struct ImageBlip {
    /// Blip record type (0xF01E PNG, 0xF01D JPEG).
    blip_type: u16,
    /// Record instance (one-UID flavour, e.g. 0x6E0 for PNG).
    instance: u16,
    /// Raw image bytes.
    bytes: Vec<u8>,
}

struct TextBlock {
    id: u32,
    text: String,
    /// Character-formatting runs over `text`, in order. Each covers a half-open
    /// range of UTF-16 code units within this block.
    runs: Vec<StyleRun>,
    /// Paragraph styles in order, one per CR in `text`.
    paragraphs: Vec<ParaStyleSpec>,
}

/// Paragraph-level formatting written into an FDPP style record.
#[derive(Clone, Copy)]
struct ParaStyleSpec {
    /// Alignment code: 0 left, 1 right, 2 center, 6 justify.
    alignment: u8,
    /// Space before / after the paragraph, in points.
    space_before: f64,
    space_after: f64,
}

impl ParaStyleSpec {
    fn is_default(&self) -> bool {
        self.alignment == 0 && self.space_before == 0.0 && self.space_after == 0.0
    }
}

/// A run of uniformly-formatted characters, used to build FDPC spans.
struct StyleRun {
    /// Length of this run in UTF-16 code units.
    len: usize,
    bold: bool,
    italic: bool,
    /// Underline style code (1 = single), or 0 for none.
    underline: u8,
    /// Font size in points, or 0 to leave the default.
    size_pt: f64,
    /// Font family name (empty for the default font).
    font: String,
    /// Text colour, or `None` for the default.
    color: Option<Rgb>,
}

/// A merged character-formatting span, ending at a global character offset.
struct CharSpan {
    end_char: usize,
    bold: bool,
    italic: bool,
    underline: u8,
    size_pt: f64,
    /// Index into the font table, or `None` for the default font.
    font_index: Option<u32>,
    /// Index into the colour palette, or `None` for the default colour.
    color_index: Option<u32>,
}

/// Build an FDPC character-style record: a 4-byte length followed by the
/// formatting blocks the importer reads (bold, italic, underline, size, colour,
/// font index).
fn character_style_record(span: &CharSpan) -> Vec<u8> {
    // block_id / block_type values mirror the importer's parser.
    const BOLD_1: u8 = 0x02;
    const ITALIC_1: u8 = 0x03;
    const UNDERLINE: u8 = 0x1E;
    const TEXT_SIZE_1: u8 = 0x0C;
    const BARE_COLOR_INDEX: u8 = 0x2E;
    const FONT_INDEX_CONTAINER: u8 = 0x24;
    const TWO_BYTE: u8 = 0x10; // a 2-byte value block
    const FOUR_BYTE: u8 = 0x20; // a 4-byte value block
    const CONTAINER: u8 = 0x8A; // a length-prefixed container the reader accepts

    let mut body = Vec::new();
    if span.bold {
        body.push(BOLD_1);
        body.push(TWO_BYTE);
        body.extend_from_slice(&1u16.to_le_bytes());
    }
    if span.italic {
        body.push(ITALIC_1);
        body.push(TWO_BYTE);
        body.extend_from_slice(&1u16.to_le_bytes());
    }
    if span.underline != 0 {
        body.push(UNDERLINE);
        body.push(TWO_BYTE);
        body.extend_from_slice(&(span.underline as u16).to_le_bytes());
    }
    if span.size_pt > 0.0 {
        let size_emu = (span.size_pt * 914_400.0 / 72.0).round() as u32;
        body.push(TEXT_SIZE_1);
        body.push(FOUR_BYTE);
        body.extend_from_slice(&size_emu.to_le_bytes());
    }
    if let Some(ci) = span.color_index {
        body.push(BARE_COLOR_INDEX);
        body.push(FOUR_BYTE);
        body.extend_from_slice(&ci.to_le_bytes());
    }
    if let Some(fi) = span.font_index {
        // Nested container: FONT_INDEX_CONTAINER > sub-container > inner block.
        // The sub-container's content must be at least 6 bytes for the reader,
        // so the inner block is a 4-byte value holding the font index.
        let mut inner = Vec::new();
        inner.push(0x00); // inner id
        inner.push(FOUR_BYTE);
        inner.extend_from_slice(&fi.to_le_bytes());
        // Sub-container wrapping the inner block. Its length word is the
        // content length only (the importer adds back the 4-byte field).
        let mut sub = Vec::new();
        sub.push(0x00); // sub id
        sub.push(CONTAINER);
        sub.extend_from_slice(&(inner.len() as u32).to_le_bytes());
        sub.extend_from_slice(&inner);
        // Outer FONT_INDEX_CONTAINER. Its length word includes the 4-byte field
        // itself, matching parse_font_index_container.
        body.push(FONT_INDEX_CONTAINER);
        body.push(CONTAINER);
        body.extend_from_slice(&((sub.len() + 4) as u32).to_le_bytes());
        body.extend_from_slice(&sub);
    }

    let mut rec = Vec::new();
    // The length field covers itself plus the body.
    rec.extend_from_slice(&((body.len() + 4) as u32).to_le_bytes());
    rec.extend_from_slice(&body);
    rec
}

/// An 8-bit-per-channel RGB colour.
type Rgb = (u8, u8, u8);

/// A shape's fill as it will be written to Escher.
enum FillSpec {
    Solid(Rgb),
    /// A two-colour linear gradient: start colour, end colour, angle in degrees.
    Linear(Rgb, Rgb, f64),
    /// A two-colour radial gradient: centre colour, edge colour.
    Radial(Rgb, Rgb),
}

/// The Escher record a shape needs so that re-import recovers its kind, fill,
/// and stroke. `spid` equals the Contents seq_num, which is how the importer
/// joins the two streams.
struct EscherShapeInfo {
    spid: u32,
    /// MSOSPT shape type (1 = rectangle, 3 = ellipse, …; 202 = text box).
    spt: u16,
    /// Fill (solid or gradient), or `None` for no fill.
    fill: Option<FillSpec>,
    /// Stroke color, or `None` for no line.
    line: Option<Rgb>,
    /// Stroke width in EMUs (only meaningful when `line` is set).
    line_width_emu: u32,
    /// Clockwise rotation in degrees, written as the FOPT ROTATION property
    /// (0 when the shape is upright).
    rotation_deg: f64,
    /// Horizontal / vertical mirror, written into the FSP flags.
    flip_h: bool,
    flip_v: bool,
    /// Line-ending arrow codes for a line/connector (start, end), written as
    /// the FOPT start/end arrow-head properties. `None` for non-line shapes or
    /// a plain line with no arrow heads.
    arrows: Option<(u32, u32)>,
    /// Quill text id for a text-bearing shape, written as the FOPT TEXT_ID
    /// property so the importer joins this record to its text frame by text id
    /// (not by a size/index fallback that could grab a neighbour's record).
    text_id: Option<u32>,
    /// One-based blip index for a picture, written as the FOPT `pib` property
    /// so the importer links the shape to its image in the blip store.
    blip_index: Option<u32>,
    /// WordArt (text-effect) string, written as the GTEXT_UNICODE complex FOPT
    /// property so re-import recovers the effect text.
    wordart_text: Option<String>,
    /// First adjust value in 21600-unit shape space (e.g. a rounded
    /// rectangle's corner radius), written as FOPT ADJUST1.
    adjust1: Option<u32>,
    /// Drop shadow: colour and (x, y) offset in points. Written as the shadow
    /// FOPT properties so re-import restores the effect.
    shadow: Option<(Rgb, f64, f64)>,
    /// Text-frame column count and gap (points), when more than one column.
    columns: Option<(u32, f64)>,
    /// Text-frame internal margins (left, top, right, bottom) in points.
    insets: Option<(f64, f64, f64, f64)>,
    /// Freeform custom-path geometry: vertices in 21600-unit shape space and
    /// per-segment type codes, written as the pVertices/pSegments complex
    /// properties.
    custom_path: Option<CustomPath>,
}

/// A freeform path encoded for Escher: vertices in 0-21600 shape space and the
/// segment type codes that consume them.
struct CustomPath {
    vertices: Vec<(i32, i32)>,
    /// Segment type codes (low byte read by the importer): 0x01 moveto,
    /// 0x00 lineto, 0x20 curveto, 0x80 close.
    segments: Vec<u16>,
}

impl<'a> PubExporter<'a> {
    fn new(document: &'a Document, options: &'a PubExportOptions) -> Self {
        Self {
            document,
            options,
            next_seq_num: 0,
            text_blocks: Vec::new(),
            images: Vec::new(),
            escher_shapes: Vec::new(),
        }
    }

    fn alloc_seq_num(&mut self) -> u32 {
        let num = self.next_seq_num;
        self.next_seq_num += 1;
        num
    }

    fn export<W: Read + Write + Seek>(&mut self, writer: W) -> FormatResult<()> {
        let mut cfb = CompoundFile::create(writer)
            .map_err(|e| FormatError::Io(std::io::Error::other(e.to_string())))?;

        // Build streams
        let contents_data = self.build_contents()?;
        let quill_data = self.build_quill()?;
        let escher_stm_data = self.build_escher_stm()?;
        let escher_delay_data = self.build_escher_delay()?;

        // Write Contents
        let mut stream = cfb.create_stream("/Contents")
            .map_err(|e| FormatError::Io(std::io::Error::other(e.to_string())))?;
        stream.write_all(&contents_data)?;
        stream.flush()?;
        drop(stream);

        // Write Quill
        cfb.create_storage("/Quill")
            .map_err(|e| FormatError::Io(std::io::Error::other(e.to_string())))?;
        cfb.create_storage("/Quill/QuillSub")
            .map_err(|e| FormatError::Io(std::io::Error::other(e.to_string())))?;
        let mut stream = cfb.create_stream("/Quill/QuillSub/CONTENTS")
            .map_err(|e| FormatError::Io(std::io::Error::other(e.to_string())))?;
        stream.write_all(&quill_data)?;
        drop(stream);

        // Write Escher
        cfb.create_storage("/Escher")
            .map_err(|e| FormatError::Io(std::io::Error::other(e.to_string())))?;
        let mut stream = cfb.create_stream("/Escher/EscherStm")
            .map_err(|e| FormatError::Io(std::io::Error::other(e.to_string())))?;
        stream.write_all(&escher_stm_data)?;
        drop(stream);

        if !escher_delay_data.is_empty() {
            let mut stream = cfb.create_stream("/Escher/EscherDelayStm")
                .map_err(|e| FormatError::Io(std::io::Error::other(e.to_string())))?;
            stream.write_all(&escher_delay_data)?;
            drop(stream);
        }

        // Write other required streams
        let mut stream = cfb.create_stream("/Internal")
            .map_err(|e| FormatError::Io(std::io::Error::other(e.to_string())))?;
        stream.write_all(&[0u8; 12])?;
        drop(stream);

        let stream = cfb.create_stream("/Envelope")
            .map_err(|e| FormatError::Io(std::io::Error::other(e.to_string())))?;
        drop(stream);

        cfb.flush()
            .map_err(|e| FormatError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    fn build_contents(&mut self) -> FormatResult<Vec<u8>> {
        let mut data = Vec::new();
        
        // Header - 0x1E bytes before the trailer offset field
        // Based on analysis of real Publisher files:
        // Bytes 0x00-0x03: Some kind of magic/version
        // Bytes 0x04-0x07: Version info
        // etc.
        // Trailer offset at 0x1A
        
        // Write a header similar to real Publisher files
        let first_page = self.document.pages().first();
        let (width, height) = if let Some(page) = first_page {
            (points_to_emus(page.size.width) as u32, points_to_emus(page.size.height) as u32)
        } else {
            (points_to_emus(612.0) as u32, points_to_emus(792.0) as u32)
        };
        
        // Header bytes (based on observed patterns)
        data.extend_from_slice(&width.to_le_bytes());           // 0x00: width
        data.extend_from_slice(&0x03E8u16.to_le_bytes());       // 0x04: version marker?
        data.extend_from_slice(&0x000Au16.to_le_bytes());       // 0x06: some flag
        data.extend_from_slice(&height.to_le_bytes());          // 0x08: height?
        data.extend_from_slice(&0x001Au16.to_le_bytes());       // 0x0C: header size?
        data.extend_from_slice(&0x0001u16.to_le_bytes());       // 0x0E: flag
        data.extend_from_slice(&[0u8; 8]);                      // 0x10: padding
        data.extend_from_slice(&0x0000u16.to_le_bytes());       // 0x18: padding
        
        // Trailer offset placeholder at 0x1A
        let trailer_offset_pos = data.len();
        data.extend_from_slice(&0u32.to_le_bytes());            // Will be patched
        
        // Build chunk directory entries
        // Note: chunk offsets are ABSOLUTE offsets from the start of the Contents stream
        let mut chunk_entries: Vec<ChunkEntry> = Vec::new();
        
        // First, calculate what seq nums pages will get
        // Doc gets seq 0, then each page followed by its shapes
        let doc_seq = 0u32;
        let mut page_seq_nums: Vec<u32> = Vec::new();
        let mut next_seq = 1u32; // Start after doc
        
        for page in self.document.pages() {
            page_seq_nums.push(next_seq);
            next_seq += 1;
            
            // Add seq nums for shapes on this page, counting every group
            // descendant (each gets its own chunk and seq num).
            let shape_count: usize = page
                .layers()
                .iter()
                .flat_map(|l| l.objects())
                .map(|obj| count_shape_chunks(&obj))
                .sum();
            next_seq += shape_count as u32;
        }
        
        // Reset our seq counter
        self.next_seq_num = 0;
        
        // Now write document chunk with correct page seq nums
        let doc_offset = data.len();
        self.write_document_chunk(&mut data, &page_seq_nums)?;
        let _ = self.alloc_seq_num(); // seq 0 for doc
        chunk_entries.push(ChunkEntry {
            chunk_type: chunk_type::DOCUMENT,
            offset: doc_offset as u32,
            parent_seq: 0,
            seq: doc_seq,
        });
        
        // Page chunks
        for (page_idx, page) in self.document.pages().iter().enumerate() {
            let page_seq = self.alloc_seq_num();
            
            // Collect shape seqnums for this page, including every group
            // descendant so the importer can nest children back into groups.
            let mut page_shape_seqs: Vec<u32> = Vec::new();
            let shape_start_seq = self.next_seq_num;

            let shape_count: usize = page
                .layers()
                .iter()
                .flat_map(|l| l.objects())
                .map(|obj| count_shape_chunks(&obj))
                .sum();

            for i in 0..shape_count {
                page_shape_seqs.push(shape_start_seq + i as u32);
            }
            
            // Write page chunk
            let page_offset = data.len();
            self.write_page_chunk(&mut data, page, &page_shape_seqs)?;
            chunk_entries.push(ChunkEntry {
                chunk_type: chunk_type::PAGE,
                offset: page_offset as u32,
                parent_seq: doc_seq,
                seq: page_seq,
            });
            
            // Write shapes
            for layer in page.layers() {
                for obj in layer.objects() {
                    self.write_shape_entries(&mut data, &mut chunk_entries, &obj, page_seq)?;
                }
            }
        }
        
        // Write trailer
        let trailer_offset = data.len();
        self.write_trailer(&mut data, &chunk_entries)?;
        
        // Patch trailer offset at 0x1A
        let trailer_bytes = (trailer_offset as u32).to_le_bytes();
        data[trailer_offset_pos..trailer_offset_pos + 4].copy_from_slice(&trailer_bytes);
        
        Ok(data)
    }

    fn write_document_chunk(&mut self, data: &mut Vec<u8>, page_seq_nums: &[u32]) -> FormatResult<()> {
        // Document chunk format:
        // - 4 bytes: total chunk length (includes these 4 bytes)
        // - Then blocks with id+type format
        
        let chunk_start = data.len();
        data.extend_from_slice(&0u32.to_le_bytes()); // Placeholder for length
        
        let first_page = self.document.pages().first();
        let (width, height) = if let Some(page) = first_page {
            (points_to_emus(page.size.width) as u32, points_to_emus(page.size.height) as u32)
        } else {
            (points_to_emus(612.0) as u32, points_to_emus(792.0) as u32)
        };
        
        // Document size container (block_id::DOCUMENT_SIZE = 0x12)
        let size_start = write_container_start(data, 0x12);
        write_u32_block(data, block_id::DOCUMENT_WIDTH, width);
        write_u32_block(data, block_id::DOCUMENT_HEIGHT, height);
        patch_container_length(data, size_start);
        
        // Page list container (block_id::DOCUMENT_PAGE_LIST = 0x02)
        // Contains sequence numbers of pages (actual seq nums, not indices)
        if !page_seq_nums.is_empty() {
            let list_start = write_container_start(data, 0x02);
            for &page_seq in page_seq_nums {
                write_u32_block(data, 0x00, page_seq);
            }
            patch_container_length(data, list_start);
        }
        
        // Patch chunk length (includes the 4-byte length field itself)
        let chunk_len = (data.len() - chunk_start) as u32;
        data[chunk_start..chunk_start + 4].copy_from_slice(&chunk_len.to_le_bytes());
        
        Ok(())
    }

    fn write_page_chunk(&self, data: &mut Vec<u8>, page: &repub_model::document::Page, shape_seqs: &[u32]) -> FormatResult<()> {
        // Page chunk format:
        // - 4 bytes: total chunk length
        // - Blocks with id+type format
        
        let chunk_start = data.len();
        data.extend_from_slice(&0u32.to_le_bytes()); // Placeholder for length
        
        // Page dimensions (optional, using same IDs as document)
        let _width = points_to_emus(page.size.width) as u32;
        let _height = points_to_emus(page.size.height) as u32;
        
        // Shape list container (PAGE_SHAPES = 0x02)
        // Contains sequence numbers of shapes on this page
        if !shape_seqs.is_empty() {
            let list_start = write_container_start(data, 0x02);
            for &seq in shape_seqs {
                // Shape seqnum: id=anything (usually increments), type=0x70 (SHAPE_SEQNUM)
                data.push(0x00);
                data.push(block_type::FOUR_BYTE_2); // 0x70 = SHAPE_SEQNUM  
                data.extend_from_slice(&seq.to_le_bytes());
            }
            patch_container_length(data, list_start);
        }
        
        // Patch chunk length
        let chunk_len = (data.len() - chunk_start) as u32;
        data[chunk_start..chunk_start + 4].copy_from_slice(&chunk_len.to_le_bytes());
        
        Ok(())
    }

    fn write_shape_entries(
        &mut self,
        data: &mut Vec<u8>,
        entries: &mut Vec<ChunkEntry>,
        obj: &Object,
        parent_seq: u32,
    ) -> FormatResult<()> {
        let seq = self.alloc_seq_num();
        let offset = data.len(); // Absolute offset
        
        // Shape chunk format:
        // - 4 bytes: total chunk length
        // - Blocks with id+type format
        
        let chunk_start = data.len();
        data.extend_from_slice(&0u32.to_le_bytes()); // Placeholder for length
        
        let bounds = obj.bounds();
        let width_emu = points_to_emus(bounds.width()) as u32;
        let height_emu = points_to_emus(bounds.height()) as u32;
        
        // Shape dimensions (required)
        write_u32_block(data, block_id::SHAPE_WIDTH, width_emu);
        write_u32_block(data, block_id::SHAPE_HEIGHT, height_emu);

        // Parent group sequence number, so re-import can nest this shape back
        // into its group. Omitted for top-level shapes (parent 0).
        if parent_seq != 0 {
            write_u32_block(data, block_id::CHUNK_PARENT_SEQNUM, parent_seq);
        }

        // Object-specific data. A group only records that it is a group here;
        // its children are written as separate chunks after this one is closed.
        let mut text_id: Option<u32> = None;
        let mut blip_index: Option<u32> = None;
        let is_group = matches!(obj, Object::Group(_));
        let is_table = matches!(obj, Object::Table(_));
        // A table reserves the next sequence number for its CELLS chunk.
        let cells_seq = if is_table {
            Some(self.alloc_seq_num())
        } else {
            None
        };
        match obj {
            Object::TextFrame(tf) => {
                // Join paragraphs with CR (Publisher's paragraph mark) so
                // re-import, via the FDPP boundaries, restores each paragraph.
                // Each paragraph is terminated by a CR, including the last, so
                // every paragraph (not just those followed by another) gets an
                // FDPP boundary and survives re-import. Character-formatting runs
                // are collected for the FDPC chunk.
                let (text, runs, paragraphs) = styled_text_to_block(&tf.content);
                if !text.is_empty() {
                    let id = self.text_blocks.len() as u32;
                    self.text_blocks.push(TextBlock {
                        id,
                        text,
                        runs,
                        paragraphs,
                    });
                    write_u32_block(data, block_id::TEXT_ID, id);
                    text_id = Some(id);
                }
            }
            Object::ImageFrame(img) => {
                // The picture's bytes go into the Escher BLIP store; the Contents
                // chunk only carries its position/size. Record the blip so the
                // shape's FOPT can reference it by one-based index.
                if let ImageSource::Embedded { data: img_data, mime_type } = &img.source {
                    if let Ok(bytes) = base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        img_data,
                    ) {
                        if let Some(blip) = ImageBlip::from_bytes(mime_type, bytes) {
                            self.images.push(blip);
                            blip_index = Some(self.images.len() as u32);
                        }
                    }
                }
            }
            Object::Table(table) => {
                // Row/column counts and the CELLS chunk this table owns.
                write_u32_block(data, block_id::TABLE_NUM_ROWS, table.rows);
                write_u32_block(data, block_id::TABLE_NUM_COLS, table.cols);
                write_u32_block(data, block_id::TABLE_CELLS_SEQNUM, cells_seq.unwrap());

                // Row/column size array: a variable-length block holding one
                // inner container whose sub-blocks give each column width then
                // each row height (in EMUs).
                self.write_table_rowcol_array(data, table);

                // One text paragraph per cell, row-major, carried as this
                // table's text story so re-import can fill the cells.
                // Each cell is exactly one CR-terminated paragraph in the story,
                // in row-major order, so the importer maps paragraphs to cells
                // one-to-one. Cell text and its character runs are collected for
                // FDPC; the terminating CR is an unstyled run.
                let mut story = String::new();
                let mut runs: Vec<StyleRun> = Vec::new();
                let mut paragraphs: Vec<ParaStyleSpec> = Vec::new();
                for cell in &table.cells {
                    let (ctext, cruns) = cell_text_runs(&cell.content);
                    story.push_str(&ctext);
                    runs.extend(cruns);
                    story.push('\r');
                    runs.push(StyleRun {
                        len: 1,
                        bold: false,
                        italic: false,
                        underline: 0,
                        size_pt: 0.0,
                        font: String::new(),
                        color: None,
                    });
                    paragraphs.push(ParaStyleSpec {
                        alignment: 0,
                        space_before: 0.0,
                        space_after: 0.0,
                    });
                }
                let id = self.text_blocks.len() as u32;
                self.text_blocks.push(TextBlock {
                    id,
                    text: story,
                    runs,
                    paragraphs,
                });
                write_u32_block(data, block_id::TEXT_ID, id);
                text_id = Some(id);
            }
            _ => {}
        }

        // Record the Escher geometry/style for this shape so re-import recovers
        // its kind, fill and stroke (joined to this chunk by seq == spid, and by
        // text id for text frames). Groups carry no geometry of their own.
        self.record_escher_shape(seq, obj, text_id, blip_index);

        // Patch chunk length
        let chunk_len = (data.len() - chunk_start) as u32;
        data[chunk_start..chunk_start + 4].copy_from_slice(&chunk_len.to_le_bytes());

        entries.push(ChunkEntry {
            chunk_type: if is_group {
                chunk_type::GROUP
            } else if is_table {
                chunk_type::TABLE
            } else {
                chunk_type::SHAPE
            },
            offset: offset as u32,
            parent_seq,
            seq,
        });

        // A table's CELLS chunk follows immediately, describing each cell's
        // row/column span in row-major order.
        if let Object::Table(table) = obj {
            let cells_seq = cells_seq.unwrap();
            let cells_offset = data.len();
            self.write_cells_chunk(data, table);
            entries.push(ChunkEntry {
                chunk_type: chunk_type::CELLS,
                offset: cells_offset as u32,
                parent_seq: seq,
                seq: cells_seq,
            });
        }

        // Now that the group's own chunk is closed, write its children as
        // separate chunks that point back at it via parent_seq.
        if let Object::Group(group) = obj {
            for child in &group.children {
                self.write_shape_entries(data, entries, child, seq)?;
            }
        }

        Ok(())
    }

    /// Write a table's ROWCOL_ARRAY block: a variable-length block whose
    /// content is a length-prefixed inner container of per-column widths then
    /// per-row heights (each a `TABLE_ROWCOL_SIZE` u32 block, in EMUs).
    fn write_table_rowcol_array(&self, data: &mut Vec<u8>, table: &Table) {
        // Build the inner payload first so we can length-prefix it.
        let mut inner = Vec::new();
        // Inner container is itself an `id == 0` block the importer descends
        // into (skipping a 4-byte header before the sub-blocks).
        let container_start = write_container_start(&mut inner, 0x00);
        inner.extend_from_slice(&0u32.to_le_bytes()); // 4-byte inner header the importer skips
        for &w in &table.col_widths {
            write_u32_block(&mut inner, block_id::TABLE_ROWCOL_SIZE, points_to_emus(w) as u32);
        }
        for &h in &table.row_heights {
            write_u32_block(&mut inner, block_id::TABLE_ROWCOL_SIZE, points_to_emus(h) as u32);
        }
        patch_container_length(&mut inner, container_start);

        // The ROWCOL_ARRAY block: id + type(0xA0 variable) + total length (incl.
        // the 4-byte length field), with the array payload prefixed by its own
        // length word (which is what the importer reads first).
        data.push(block_id::TABLE_ROWCOL_ARRAY);
        data.push(block_type::VARIABLE);
        let array_len = 4 + 4 + inner.len(); // outer length word + inner length word + inner
        data.extend_from_slice(&(array_len as u32).to_le_bytes());
        // Payload: inner length word the importer reads as `array_length`.
        data.extend_from_slice(&((4 + inner.len()) as u32).to_le_bytes());
        data.extend_from_slice(&inner);
    }

    /// Write a table's CELLS chunk: a cell count followed by a container of
    /// per-cell start/end row and column (row-major, single-cell spans).
    fn write_cells_chunk(&self, data: &mut Vec<u8>, table: &Table) {
        let chunk_start = data.len();
        data.extend_from_slice(&0u32.to_le_bytes()); // chunk length placeholder

        let total = table.rows * table.cols;
        write_u32_block(data, 0x01, total); // cell count

        // Cell array container (id 0x02).
        let array = write_container_start(data, 0x02);
        data.extend_from_slice(&0u32.to_le_bytes()); // 4-byte header the importer skips
        for idx in 0..total {
            let row = idx / table.cols;
            let col = idx % table.cols;
            // Each cell is an id==0 container holding start/end row and column.
            let cell = write_container_start(data, 0x00);
            data.extend_from_slice(&0u32.to_le_bytes()); // 4-byte header the importer skips
            write_u32_block(data, 0x01, row); // start_row
            write_u32_block(data, 0x02, row); // end_row
            write_u32_block(data, 0x03, col); // start_column
            write_u32_block(data, 0x04, col); // end_column
            patch_container_length(data, cell);
        }
        patch_container_length(data, array);

        let chunk_len = (data.len() - chunk_start) as u32;
        data[chunk_start..chunk_start + 4].copy_from_slice(&chunk_len.to_le_bytes());
    }

    /// Capture `obj`'s shape type and fill/stroke colours as an Escher record.
    fn record_escher_shape(
        &mut self,
        seq: u32,
        obj: &Object,
        text_id: Option<u32>,
        blip_index: Option<u32>,
    ) {
        use repub_model::styles::Fill;

        let rgb = |c: &repub_color::Color| {
            let (r, g, b, _) = c.to_rgba8();
            (r, g, b)
        };
        let fill_of = |fill: &Fill| -> Option<FillSpec> {
            match fill {
                Fill::None => None,
                Fill::LinearGradient(g) => {
                    let start = rgb(&g.stops.first().map(|s| s.color).unwrap_or(repub_color::Color::WHITE));
                    let end = rgb(&g.stops.last().map(|s| s.color).unwrap_or(repub_color::Color::WHITE));
                    // Angle from the gradient's start→end direction.
                    let dx = g.end.x - g.start.x;
                    let dy = g.end.y - g.start.y;
                    let angle = dy.atan2(dx).to_degrees();
                    Some(FillSpec::Linear(start, end, angle))
                }
                Fill::RadialGradient(g) => {
                    let start = rgb(&g.stops.first().map(|s| s.color).unwrap_or(repub_color::Color::WHITE));
                    let end = rgb(&g.stops.last().map(|s| s.color).unwrap_or(repub_color::Color::WHITE));
                    Some(FillSpec::Radial(start, end))
                }
                _ => Some(FillSpec::Solid(rgb(&fill.primary_color()))),
            }
        };

        let shadow_of = |sh: &Option<repub_model::styles::DropShadow>| {
            sh.as_ref().map(|s| (rgb(&s.color), s.offset_x, s.offset_y))
        };
        let mut adjust1: Option<u32> = None;
        let mut custom_path: Option<CustomPath> = None;
        let (spt, fill, line, line_w): (u16, Option<FillSpec>, Option<Rgb>, u32) =
            match obj {
                Object::Shape(s) => {
                    let bounds = s.common.bounds;
                    let spt = match &s.shape_type {
                        ShapeType::Rectangle { corner_radius } if *corner_radius > 0.0 => {
                            // The corner radius as a fraction of the smaller side,
                            // in Publisher's 21600-unit shape space.
                            let min_side = bounds.width().min(bounds.height());
                            if min_side > 0.0 {
                                let frac = (*corner_radius / min_side).clamp(0.0, 0.5);
                                adjust1 = Some((frac * 21600.0).round() as u32);
                            }
                            escher::SHAPE_TYPE_ROUND_RECT as u16
                        }
                        ShapeType::Rectangle { .. } => escher::SHAPE_TYPE_RECTANGLE as u16,
                        ShapeType::Ellipse => escher::SHAPE_TYPE_ELLIPSE as u16,
                        ShapeType::Polygon { sides } => match sides {
                            3 => escher::SHAPE_TYPE_ISOSCELES_TRIANGLE as u16,
                            5 => escher::SHAPE_TYPE_PENTAGON as u16,
                            6 => escher::SHAPE_TYPE_HEXAGON as u16,
                            _ => escher::SHAPE_TYPE_RECTANGLE as u16,
                        },
                        ShapeType::Star { .. } => escher::SHAPE_TYPE_RECTANGLE as u16,
                        ShapeType::Path(path) => {
                            // A freeform shape: encode its outline as pVertices /
                            // pSegments in 21600-unit shape space (spt 0).
                            custom_path = encode_custom_path(path, bounds);
                            0
                        }
                    };
                    let line = s.style.stroke.as_ref().map(|st| rgb(&st.color));
                    let lw = s
                        .style
                        .stroke
                        .as_ref()
                        .map(|st| points_to_emus(st.width) as u32)
                        .unwrap_or(0);
                    (spt, fill_of(&s.style.fill), line, lw)
                }
                Object::TextFrame(tf) => {
                    let line = tf.style.stroke.as_ref().map(|st| rgb(&st.color));
                    let lw = tf
                        .style
                        .stroke
                        .as_ref()
                        .map(|st| points_to_emus(st.width) as u32)
                        .unwrap_or(0);
                    (
                        escher::SHAPE_TYPE_TEXT_BOX as u16,
                        fill_of(&tf.style.fill),
                        line,
                        lw,
                    )
                }
                Object::ImageFrame(_) => (escher::SHAPE_TYPE_PICTURE_FRAME as u16, None, None, 0),
                Object::Line(l) => {
                    let line = Some(rgb(&l.stroke.color));
                    let lw = points_to_emus(l.stroke.width) as u32;
                    let arrows = (
                        arrow_code(l.start_ending.style),
                        arrow_code(l.end_ending.style),
                    );
                    self.escher_shapes.push(EscherShapeInfo {
                        spid: seq,
                        spt: escher::SHAPE_TYPE_LINE as u16,
                        fill: None,
                        line,
                        line_width_emu: lw,
                        rotation_deg: obj.common().rotation,
                        flip_h: obj.common().flip_h,
                        flip_v: obj.common().flip_v,
                        arrows: Some(arrows),
                        text_id,
                        blip_index: None,
                        wordart_text: None,
                        adjust1: None,
                        shadow: None,
                        columns: None,
                        insets: None,
                        custom_path: None,
                    });
                    return;
                }
                Object::WordArt(wa) => {
                    // A text effect: a WordArt shape type carrying its string as
                    // the GTEXT_UNICODE complex property.
                    self.escher_shapes.push(EscherShapeInfo {
                        spid: seq,
                        spt: escher::SHAPE_TYPE_TEXT_PLAIN_TEXT as u16,
                        fill: fill_of(&wa.fill),
                        line: wa.outline.as_ref().map(|st| rgb(&st.color)),
                        line_width_emu: wa
                            .outline
                            .as_ref()
                            .map(|st| points_to_emus(st.width) as u32)
                            .unwrap_or(0),
                        rotation_deg: obj.common().rotation,
                        flip_h: obj.common().flip_h,
                        flip_v: obj.common().flip_v,
                        arrows: None,
                        text_id,
                        blip_index: None,
                        wordart_text: Some(wa.text.clone()),
                        adjust1: None,
                        shadow: None,
                        columns: None,
                        insets: None,
                        custom_path: None,
                    });
                    return;
                }
                // Tables and groups are not given Escher geometry here; they
                // round-trip via Contents only.
                _ => return,
            };

        // Drop shadow, if the shape carries one.
        let shadow = match obj {
            Object::Shape(s) => shadow_of(&s.style.shadow),
            Object::TextFrame(tf) => shadow_of(&tf.style.shadow),
            Object::ImageFrame(img) => shadow_of(&img.style.shadow),
            _ => None,
        };

        // Text-frame columns and internal margins.
        let (columns, insets) = match obj {
            Object::TextFrame(tf) => {
                let cols = if tf.columns.count > 1 {
                    Some((tf.columns.count, tf.columns.gap))
                } else {
                    None
                };
                let m = &tf.margins;
                let insets = if m.left != 0.0 || m.top != 0.0 || m.right != 0.0 || m.bottom != 0.0
                {
                    Some((m.left, m.top, m.right, m.bottom))
                } else {
                    None
                };
                (cols, insets)
            }
            _ => (None, None),
        };

        self.escher_shapes.push(EscherShapeInfo {
            spid: seq,
            spt,
            fill,
            line,
            line_width_emu: line_w,
            rotation_deg: obj.common().rotation,
            flip_h: obj.common().flip_h,
            flip_v: obj.common().flip_v,
            arrows: None,
            text_id,
            blip_index,
            wordart_text: None,
            adjust1,
            shadow,
            columns,
            insets,
            custom_path,
        });
    }

    fn write_trailer(&self, data: &mut Vec<u8>, entries: &[ChunkEntry]) -> FormatResult<()> {
        // Trailer format:
        // - 4 bytes: total trailer length (including this field)
        // - Block id=0x01, type=0x20: entry count?
        // - Block id=0x02, type=0x20: entry count?
        // - Block id=0x03, type=0x90 (TRAILER_DIRECTORY): directory entries
        
        let trailer_start = data.len();
        data.extend_from_slice(&0u32.to_le_bytes()); // Placeholder for total length
        
        // Version/sequence info blocks
        write_u32_block(data, 0x01, entries.len() as u32);
        write_u32_block(data, 0x02, entries.len() as u32);
        
        // Trailer directory block (id=0x03, type=0x90)
        // The 0x90 type is a variable-length block like containers
        data.push(0x03); // id
        data.push(block_type::TRAILER); // 0x90
        
        let dir_len_pos = data.len();
        data.extend_from_slice(&0u32.to_le_bytes()); // Placeholder for directory length
        
        // Write directory entries (each is a container with chunk info)
        for entry in entries {
            let entry_container = write_container_start(data, 0x00);
            write_u16_block(data, block_id::CHUNK_TYPE, entry.chunk_type as u16);
            write_u32_block(data, block_id::CHUNK_OFFSET, entry.offset);
            write_u32_block(data, block_id::CHUNK_PARENT_SEQNUM, entry.parent_seq);
            patch_container_length(data, entry_container);
        }
        
        // Patch directory length (includes the 4-byte length field)
        let dir_len = (data.len() - dir_len_pos) as u32;
        data[dir_len_pos..dir_len_pos + 4].copy_from_slice(&dir_len.to_le_bytes());
        
        // Add padding (some zero-blocks)
        for _ in 0..20 {
            write_zero_block(data, 0x00);
        }
        
        // Patch total trailer length
        let trailer_len = (data.len() - trailer_start) as u32;
        data[trailer_start..trailer_start + 4].copy_from_slice(&trailer_len.to_le_bytes());
        
        Ok(())
    }

    fn build_quill(&self) -> FormatResult<Vec<u8>> {
        let mut data = Vec::new();
        
        // Quill signature: "CHNKINK " (8 bytes)
        data.extend_from_slice(b"CHNKINK ");
        
        // Header bytes (based on observed real Publisher files)
        // 0x08: 04 00 - version?
        data.extend_from_slice(&0x0004u16.to_le_bytes()); // 0x08
        // 0x0A: 07 00 
        data.extend_from_slice(&0x0007u16.to_le_bytes()); // 0x0A
        // 0x0C: 14 00
        data.extend_from_slice(&0x0014u16.to_le_bytes()); // 0x0C
        // 0x0E: 00 03 (0x0300)
        data.extend_from_slice(&0x0300u16.to_le_bytes()); // 0x0E
        // 0x10: 00 04 (0x0400)
        data.extend_from_slice(&0x0400u16.to_le_bytes()); // 0x10
        // 0x12: 00 00 
        data.extend_from_slice(&0x0000u16.to_le_bytes()); // 0x12
        // 0x14-0x17: some offset value
        data.extend_from_slice(&0x00000022u32.to_le_bytes()); // 0x14
        
        // 0x18-0x19: Initial value (looks like an offset or count - 0x01f8 in original)
        // For simple files, use a smaller value
        // Paragraph boundaries (one entry per CR) across all text blocks, as
        // global character offsets of each paragraph end. Written as an FDPP
        // chunk so re-import splits each block into paragraphs (needed for
        // table cells and multi-paragraph text frames).
        let para_end_chars = self.paragraph_end_chars();

        // Character-formatting spans (bold/italic/underline/size) across all
        // text blocks, with global character offsets. Empty when every run is
        // plain.
        let (char_spans, font_names, palette) = self.character_spans();

        let chunk_refs_start = 0x20usize;
        // TEXT, STRS, SYID, FONT, then optionally FDPP (paragraph marks), FDPC
        // (character formatting) and PL (colour palette).
        let num_chunks = if self.text_blocks.is_empty() {
            0u16
        } else {
            4u16
                + if para_end_chars.is_empty() { 0 } else { 1 }
                + if char_spans.is_empty() { 0 } else { 1 }
                + if palette.is_empty() { 0 } else { 1 }
        };
        let chunk_refs_size = num_chunks as usize * 0x18;
        let data_start = chunk_refs_start + chunk_refs_size;
        
        // Initial value at 0x18 - seems to be related to data start
        data.extend_from_slice(&(data_start as u16).to_le_bytes()); // 0x18
        
        // 0x1A: Number of chunks
        data.extend_from_slice(&num_chunks.to_le_bytes()); // 0x1A
        
        // 0x1C-0x1F: Next chunk list offset (0xFFFFFFFF = none, or actual offset if more)
        // For simple files with one list, use 0xFFFFFFFF
        data.extend_from_slice(&0xFFFFFFFFu32.to_le_bytes()); // 0x1C
        
        // Now at 0x20 - chunk refs start here
        
        // Build text content first (we need to know sizes)
        let mut text_data: Vec<u8> = Vec::new();
        for block in &self.text_blocks {
            for ch in block.text.encode_utf16() {
                text_data.extend_from_slice(&ch.to_le_bytes());
            }
        }
        
        // Build chunk directory and data
        let mut chunks: Vec<QuillChunkInfo> = Vec::new();
        let mut chunk_data: Vec<u8> = Vec::new();
        
        if !self.text_blocks.is_empty() {
            // Data starts right after chunk refs
            let mut current_offset = data_start as u32;
            
            // TEXT chunk - actual text content in UTF-16
            chunks.push(QuillChunkInfo {
                name: *b"TEXT",
                id: 0,
                name2: *b"TEXT",
                offset: current_offset,
                length: text_data.len() as u32,
            });
            chunk_data.extend_from_slice(&text_data);
            current_offset += text_data.len() as u32;
            
            // STRS chunk - string table with character counts per text block
            // Format: num_entries (4) + header_size (4) + padding[header_size] + counts[num_entries] (4 each)
            // Import reads entries at offset: 4 + header_size
            // So with header_size=4, entries start at byte 8 (after the two u32 header fields)
            let mut strs_data = Vec::new();
            strs_data.extend_from_slice(&(self.text_blocks.len() as u32).to_le_bytes()); // num entries
            strs_data.extend_from_slice(&4u32.to_le_bytes()); // header_size=4
            for block in &self.text_blocks {
                let char_count = block.text.encode_utf16().count() as u32;
                strs_data.extend_from_slice(&char_count.to_le_bytes());
            }
            chunks.push(QuillChunkInfo {
                name: *b"STRS",
                id: 0,
                name2: *b"PLC ",
                offset: current_offset,
                length: strs_data.len() as u32,
            });
            chunk_data.extend_from_slice(&strs_data);
            current_offset += strs_data.len() as u32;
            
            // SYID chunk - text IDs mapping
            let mut syid_data = Vec::new();
            syid_data.extend_from_slice(&0u32.to_le_bytes()); // Unknown
            syid_data.extend_from_slice(&(self.text_blocks.len() as u32).to_le_bytes());
            for block in &self.text_blocks {
                syid_data.extend_from_slice(&block.id.to_le_bytes());
            }
            chunks.push(QuillChunkInfo {
                name: *b"SYID",
                id: 0,
                name2: *b"SYID",
                offset: current_offset,
                length: syid_data.len() as u32,
            });
            chunk_data.extend_from_slice(&syid_data);
            current_offset += syid_data.len() as u32;
            
            // FONT chunk - font table. Layout the importer reads: a DWORD, the
            // font count, a 12-byte area, a per-font u32 offset table, then each
            // font as [u16 name_len, UTF-16 name, u32 trailing].
            let font_table: Vec<String> = if font_names.is_empty() {
                vec!["Calibri".to_string()]
            } else {
                font_names.clone()
            };
            let mut font_data = Vec::new();
            font_data.extend_from_slice(&0u32.to_le_bytes()); // leading DWORD
            font_data.extend_from_slice(&(font_table.len() as u32).to_le_bytes());
            font_data.extend_from_slice(&[0u8; 12]); // header area
            for i in 0..font_table.len() {
                font_data.extend_from_slice(&(i as u32).to_le_bytes()); // offset table (unused by reader)
            }
            for name in &font_table {
                let utf16: Vec<u16> = name.encode_utf16().collect();
                font_data.extend_from_slice(&(utf16.len() as u16).to_le_bytes());
                for ch in utf16 {
                    font_data.extend_from_slice(&ch.to_le_bytes());
                }
                font_data.extend_from_slice(&0u32.to_le_bytes()); // trailing DWORD
            }
            chunks.push(QuillChunkInfo {
                name: *b"FONT",
                id: 0,
                name2: *b"FONT",
                offset: current_offset,
                length: font_data.len() as u32,
            });
            chunk_data.extend_from_slice(&font_data);
            current_offset += font_data.len() as u32;

            // FDPP chunk - paragraph boundaries. `text_offset[i]` is the
            // absolute byte offset (in this stream) of paragraph i's end; the
            // TEXT data starts at `data_start`, two bytes per UTF-16 char.
            if !para_end_chars.is_empty() {
                let n = para_end_chars.len();
                let header_len = 8 + n * 4 + n * 2;

                // Build a style record per paragraph (default paragraphs share a
                // single empty record to stay compact).
                let mut records: Vec<u8> = Vec::new();
                let mut record_offsets: Vec<u16> = Vec::new();
                let mut default_off: Option<u16> = None;
                for (_, style) in &para_end_chars {
                    if style.is_default() {
                        let off = *default_off.get_or_insert_with(|| {
                            let off = (header_len + records.len()) as u16;
                            records.extend_from_slice(&0u16.to_le_bytes());
                            off
                        });
                        record_offsets.push(off);
                    } else {
                        record_offsets.push((header_len + records.len()) as u16);
                        records.extend_from_slice(&paragraph_style_record(style));
                    }
                }

                let mut fdpp = Vec::new();
                fdpp.extend_from_slice(&(n as u16).to_le_bytes()); // num_entries
                fdpp.extend_from_slice(&[0u8; 6]); // header padding to offset 8
                for (end_char, _) in &para_end_chars {
                    let byte_off = data_start as u32 + (*end_char as u32) * 2;
                    fdpp.extend_from_slice(&byte_off.to_le_bytes());
                }
                for &off in &record_offsets {
                    fdpp.extend_from_slice(&off.to_le_bytes());
                }
                fdpp.extend_from_slice(&records);

                chunks.push(QuillChunkInfo {
                    name: *b"FDPP",
                    id: 0,
                    name2: *b"FDPP",
                    offset: current_offset,
                    length: fdpp.len() as u32,
                });
                chunk_data.extend_from_slice(&fdpp);
                current_offset += fdpp.len() as u32;
            }

            // FDPC chunk - character formatting spans. Each entry's text_offset
            // is the absolute byte offset of the span's end; chunk_offset points
            // at the span's style record within this chunk.
            if !char_spans.is_empty() {
                let n = char_spans.len();
                let header_len = 8 + n * 4 + n * 2;

                // Build the style records and record each one's offset.
                let mut records = Vec::new();
                let mut record_offsets = Vec::new();
                for span in &char_spans {
                    record_offsets.push((header_len + records.len()) as u16);
                    let rec = character_style_record(span);
                    records.extend_from_slice(&rec);
                }

                let mut fdpc = Vec::new();
                fdpc.extend_from_slice(&(n as u16).to_le_bytes());
                fdpc.extend_from_slice(&[0u8; 6]);
                for span in &char_spans {
                    let byte_off = data_start as u32 + (span.end_char as u32) * 2;
                    fdpc.extend_from_slice(&byte_off.to_le_bytes());
                }
                for &off in &record_offsets {
                    fdpc.extend_from_slice(&off.to_le_bytes());
                }
                fdpc.extend_from_slice(&records);

                chunks.push(QuillChunkInfo {
                    name: *b"FDPC",
                    id: 0,
                    name2: *b"FDPC",
                    offset: current_offset,
                    length: fdpc.len() as u32,
                });
                chunk_data.extend_from_slice(&fdpc);
                current_offset += fdpc.len() as u32;
            }

            // PL chunk - colour palette. Each entry is a length-prefixed block
            // holding a (0x01, 0x20) colour value.
            if !palette.is_empty() {
                let mut pl = Vec::new();
                pl.extend_from_slice(&(palette.len() as u32).to_le_bytes());
                pl.extend_from_slice(&[0u8; 8]); // header area
                for (r, g, b) in &palette {
                    // block: u32 length (covers itself + the 6-byte colour block)
                    pl.extend_from_slice(&10u32.to_le_bytes());
                    pl.push(0x01);
                    pl.push(0x20);
                    pl.extend_from_slice(&rgb_to_bgr(*r, *g, *b).to_le_bytes());
                }
                chunks.push(QuillChunkInfo {
                    name: *b"PL  ",
                    id: 0,
                    name2: *b"PL  ",
                    offset: current_offset,
                    length: pl.len() as u32,
                });
                chunk_data.extend_from_slice(&pl);
            }
        }
        
        // Write chunk references (each is 0x18 = 24 bytes)
        // Format: marker (2) + name (4) + id (2) + unknown (4) + name2 (4) + offset (4) + length (4) = 24
        for chunk in &chunks {
            data.extend_from_slice(&0x0018u16.to_le_bytes()); // Marker = 0x18
            data.extend_from_slice(&chunk.name);               // 4-char name
            data.extend_from_slice(&chunk.id.to_le_bytes());   // ID
            data.extend_from_slice(&0x00000001u32.to_le_bytes()); // Unknown = 1
            data.extend_from_slice(&chunk.name2);              // 4-char name2
            data.extend_from_slice(&chunk.offset.to_le_bytes()); // Data offset
            data.extend_from_slice(&chunk.length.to_le_bytes()); // Data length
        }
        
        // Append actual chunk data
        data.extend_from_slice(&chunk_data);
        
        Ok(data)
    }

    /// Character-formatting spans across all text blocks concatenated, as a PLC
    /// keyed by the global character offset of each span's end, together with
    /// the font-name and colour tables the spans reference. Returns empty spans
    /// when every run is plain (default font/colour/size, no emphasis), so the
    /// FDPC chunk is only written when it carries information.
    fn character_spans(&self) -> (Vec<CharSpan>, Vec<String>, Vec<Rgb>) {
        let mut spans: Vec<CharSpan> = Vec::new();
        let mut fonts: Vec<String> = Vec::new();
        let mut colors: Vec<Rgb> = Vec::new();
        let mut base = 0usize;
        let mut any_formatting = false;

        let mut font_index = |name: &str| -> Option<u32> {
            if name.is_empty() {
                return None;
            }
            if let Some(i) = fonts.iter().position(|f| f == name) {
                return Some(i as u32);
            }
            fonts.push(name.to_string());
            Some((fonts.len() - 1) as u32)
        };
        let mut color_index = |c: Option<Rgb>| -> Option<u32> {
            let c = c?;
            if let Some(i) = colors.iter().position(|&x| x == c) {
                return Some(i as u32);
            }
            colors.push(c);
            Some((colors.len() - 1) as u32)
        };

        for block in &self.text_blocks {
            let mut pos = base;
            for run in &block.runs {
                let end = pos + run.len;
                let fi = font_index(&run.font);
                let ci = color_index(run.color);
                if run.bold
                    || run.italic
                    || run.underline != 0
                    || run.size_pt != 12.0
                    || fi.is_some()
                    || ci.is_some()
                {
                    any_formatting = true;
                }
                // Merge with the previous span when everything matches and it is
                // contiguous, to keep the PLC compact.
                if let Some(last) = spans.last_mut() {
                    if last.end_char == pos
                        && last.bold == run.bold
                        && last.italic == run.italic
                        && last.underline == run.underline
                        && (last.size_pt - run.size_pt).abs() < f64::EPSILON
                        && last.font_index == fi
                        && last.color_index == ci
                    {
                        last.end_char = end;
                        pos = end;
                        continue;
                    }
                }
                spans.push(CharSpan {
                    end_char: end,
                    bold: run.bold,
                    italic: run.italic,
                    underline: run.underline,
                    size_pt: run.size_pt,
                    font_index: fi,
                    color_index: ci,
                });
                pos = end;
            }
            base = pos;
        }

        if any_formatting {
            (spans, fonts, colors)
        } else {
            (Vec::new(), Vec::new(), Vec::new())
        }
    }

    /// Global character offsets, across all text blocks concatenated, of each
    /// paragraph end (the position just past each CR), paired with that
    /// paragraph's style. Empty when no text block contains a paragraph mark.
    fn paragraph_end_chars(&self) -> Vec<(usize, ParaStyleSpec)> {
        let default = ParaStyleSpec {
            alignment: 0,
            space_before: 0.0,
            space_after: 0.0,
        };
        let mut ends = Vec::new();
        let mut base = 0usize;
        for block in &self.text_blocks {
            let chars: Vec<u16> = block.text.encode_utf16().collect();
            let mut para_idx = 0;
            for (i, &c) in chars.iter().enumerate() {
                if c == b'\r' as u16 {
                    // Paragraph ends at the character just after the CR.
                    let style = block.paragraphs.get(para_idx).copied().unwrap_or(default);
                    ends.push((base + i + 1, style));
                    para_idx += 1;
                }
            }
            base += chars.len();
        }
        ends
    }

    fn build_escher_stm(&self) -> FormatResult<Vec<u8>> {
        let mut data = Vec::new();

        // DggContainer (0xF000) { [BStoreContainer] DgContainer (0xF002) { SpContainer per shape } }
        let dgg = escher_container_start(&mut data, escher::ESCHER_DGG_CONTAINER);

        // Blip store: one FBSE per embedded image, each pointing at its BLIP
        // record's offset in EscherDelayStm. foDelay offsets are computed with
        // the same layout build_escher_delay writes.
        if !self.images.is_empty() {
            let bstore = escher_container_start(&mut data, escher::ESCHER_BSTORE_CONTAINER);
            // Instance of the BStore container = number of blips.
            let mut delay_offset = 0u32;
            for image in &self.images {
                let blip_total = blip_record_len(image) as u32;
                write_fbse(&mut data, image, delay_offset, blip_total);
                delay_offset += blip_total;
            }
            escher_patch_length(&mut data, bstore);
            // Patch the BStore header instance to the blip count.
            let inst = self.images.len() as u16;
            let ver_inst = (inst << 4) | 0x0F;
            data[bstore - 4..bstore - 2].copy_from_slice(&ver_inst.to_le_bytes());
        }

        let dg = escher_container_start(&mut data, escher::ESCHER_DG_CONTAINER);

        for shape in &self.escher_shapes {
            let sp = escher_container_start(&mut data, escher::ESCHER_SP_CONTAINER);

            // FSP (0xF00A): instance = shape type (spt); body = spid + flags.
            const FSP_FLAG_FLIP_H: u32 = 0x0040;
            const FSP_FLAG_FLIP_V: u32 = 0x0080;
            let mut flags = 0u32;
            if shape.flip_h {
                flags |= FSP_FLAG_FLIP_H;
            }
            if shape.flip_v {
                flags |= FSP_FLAG_FLIP_V;
            }
            write_escher_header(&mut data, escher::ESCHER_FSP, shape.spt, 8);
            data.extend_from_slice(&shape.spid.to_le_bytes());
            data.extend_from_slice(&flags.to_le_bytes());

            // FOPT (0xF00B): property array; instance = property count.
            // FOPT properties are emitted in ascending opid order, as Publisher
            // writes them.
            let mut props: Vec<(u16, u32)> = Vec::new();
            // ROTATION (opid 0x0004): clockwise degrees in signed 16.16 fixed point.
            if shape.rotation_deg != 0.0 {
                let fixed = (shape.rotation_deg * 65536.0).round() as i32;
                props.push((escher::FOPT_ROTATION, fixed as u32));
            }
            // TEXT_ID (opid 0x0080) joins a text frame's Escher record to its
            // Quill story and to its Contents shape by text id.
            if let Some(tid) = shape.text_id {
                props.push((0x0080, tid));
            }
            // Text insets (opids 0x0081 left, 0x0082 top, 0x0083 right, 0x0084
            // bottom) in EMUs.
            if let Some((l, t, r, b)) = shape.insets {
                props.push((0x0081, points_to_emus(l) as u32));
                props.push((0x0082, points_to_emus(t) as u32));
                props.push((0x0083, points_to_emus(r) as u32));
                props.push((0x0084, points_to_emus(b) as u32));
            }
            // Text columns (opids 0x008C count, 0x008D gap in EMUs).
            if let Some((count, gap)) = shape.columns {
                props.push((0x008C, count));
                props.push((0x008D, points_to_emus(gap) as u32));
            }
            // pib (opid 0x0104): one-based blip index linking a picture to its
            // image in the blip store.
            if let Some(idx) = shape.blip_index {
                props.push((0x0104, idx));
            }
            // Complex-property data, appended after the fixed-size entry array
            // in property-id order (matching the entry order).
            let mut complex: Vec<u8> = Vec::new();
            // Freeform geometry: geoRight/geoBottom bounds plus the pVertices
            // and pSegments complex arrays (opids 0x0142, 0x0143, 0x0145, 0x0146).
            if let Some(cp) = &shape.custom_path {
                props.push((escher::FOPT_GEO_RIGHT, 21600));
                props.push((escher::FOPT_GEO_BOTTOM, 21600));

                let mut verts = Vec::new();
                verts.extend_from_slice(&(cp.vertices.len() as u16).to_le_bytes());
                verts.extend_from_slice(&(cp.vertices.len() as u16).to_le_bytes()); // total/size hint
                verts.extend_from_slice(&8u16.to_le_bytes()); // 8 bytes per vertex (two i32)
                for (x, y) in &cp.vertices {
                    verts.extend_from_slice(&x.to_le_bytes());
                    verts.extend_from_slice(&y.to_le_bytes());
                }
                props.push((escher::FOPT_P_VERTICES, verts.len() as u32));
                complex.extend_from_slice(&verts);

                let mut segs = Vec::new();
                segs.extend_from_slice(&(cp.segments.len() as u16).to_le_bytes());
                segs.extend_from_slice(&(cp.segments.len() as u16).to_le_bytes());
                segs.extend_from_slice(&2u16.to_le_bytes()); // 2 bytes per segment
                for s in &cp.segments {
                    segs.extend_from_slice(&s.to_le_bytes());
                }
                props.push((escher::FOPT_P_SEGMENTS, segs.len() as u32));
                complex.extend_from_slice(&segs);
            }
            // adjustValue (opid 0x0147): first adjust handle, e.g. a rounded
            // rectangle's corner radius in 21600-unit shape space.
            if let Some(adj) = shape.adjust1 {
                props.push((escher::FOPT_ADJUST1, adj));
            }
            match &shape.fill {
                Some(FillSpec::Solid((r, g, b))) => {
                    props.push((escher::FOPT_FILL_TYPE, escher::FILL_TYPE_SOLID));
                    props.push((escher::FOPT_FILL_COLOR, rgb_to_bgr(*r, *g, *b)));
                }
                Some(FillSpec::Linear((sr, sg, sb), (er, eg, eb), angle)) => {
                    // opid order: FILL_TYPE, FILL_COLOR, FILL_BACK_COLOR, FILL_ANGLE.
                    props.push((escher::FOPT_FILL_TYPE, escher::FILL_TYPE_SHADE));
                    props.push((escher::FOPT_FILL_COLOR, rgb_to_bgr(*sr, *sg, *sb)));
                    props.push((escher::FOPT_FILL_BACK_COLOR, rgb_to_bgr(*er, *eg, *eb)));
                    // fillAngle is signed 16.16 fixed-point degrees.
                    let fixed = (*angle * 65536.0).round() as i32;
                    props.push((escher::FOPT_FILL_ANGLE, fixed as u32));
                }
                Some(FillSpec::Radial((cr, cg, cb), (er, eg, eb))) => {
                    props.push((escher::FOPT_FILL_TYPE, escher::FILL_TYPE_SHADE_CENTER));
                    props.push((escher::FOPT_FILL_COLOR, rgb_to_bgr(*cr, *cg, *cb)));
                    props.push((escher::FOPT_FILL_BACK_COLOR, rgb_to_bgr(*er, *eg, *eb)));
                }
                None => {}
            }
            if let Some((r, g, b)) = shape.line {
                props.push((escher::FOPT_LINE_COLOR, rgb_to_bgr(r, g, b)));
                if shape.line_width_emu > 0 {
                    props.push((escher::FOPT_LINE_WIDTH, shape.line_width_emu));
                }
            }
            // Line-ending arrow heads (opids 0x01CF start, 0x01D0 end).
            if let Some((start, end)) = shape.arrows {
                if start != 0 {
                    props.push((escher::FOPT_LINE_START_ARROW_HEAD, start));
                }
                if end != 0 {
                    props.push((escher::FOPT_LINE_END_ARROW_HEAD, end));
                }
            }
            // Drop shadow (opids 0x0200 type, 0x0201 colour, 0x0205/6 offsets).
            if let Some(((r, g, b), ox, oy)) = shape.shadow {
                props.push((escher::FOPT_SHADOW_TYPE, 0)); // msoshadowOffset
                props.push((escher::FOPT_SHADOW_COLOR, rgb_to_bgr(r, g, b)));
                props.push((escher::FOPT_SHADOW_OFFSET_X, points_to_emus(ox) as u32));
                props.push((escher::FOPT_SHADOW_OFFSET_Y, points_to_emus(oy) as u32));
            }
            // WordArt text is another complex property; its data follows the
            // geometry complex data in the same block.
            if let Some(text) = &shape.wordart_text {
                let mut bytes: Vec<u8> = Vec::new();
                for ch in text.encode_utf16() {
                    bytes.extend_from_slice(&ch.to_le_bytes());
                }
                bytes.extend_from_slice(&0u16.to_le_bytes()); // null terminator
                props.push((escher::FOPT_GTEXT_UNICODE | 0x8000, bytes.len() as u32));
                complex.extend_from_slice(&bytes);
            }
            if !props.is_empty() {
                let body_len = props.len() * 6 + complex.len();
                write_escher_header(
                    &mut data,
                    escher::ESCHER_FOPT,
                    props.len() as u16,
                    body_len as u32,
                );
                for (opid, value) in props {
                    data.extend_from_slice(&opid.to_le_bytes());
                    data.extend_from_slice(&value.to_le_bytes());
                }
                data.extend_from_slice(&complex);
            }

            escher_patch_length(&mut data, sp);
        }

        escher_patch_length(&mut data, dg);
        escher_patch_length(&mut data, dgg);

        Ok(data)
    }

    fn build_escher_delay(&self) -> FormatResult<Vec<u8>> {
        let mut data = Vec::new();

        // One BLIP record per image: an 8-byte Escher header, a 16-byte UID,
        // a 1-byte tag, then the raw image bytes. Offsets here must match the
        // foDelay values written into the blip store (build_escher_stm).
        for image in &self.images {
            let contents_len = 16 + 1 + image.bytes.len();
            write_escher_header(
                &mut data,
                image.blip_type,
                image.instance,
                contents_len as u32,
            );
            data.extend_from_slice(&[0u8; 16]); // rgbUid (unused by our importer)
            data.push(0xFF); // tag
            data.extend_from_slice(&image.bytes);
        }

        Ok(data)
    }
}

struct ChunkEntry {
    chunk_type: u8,
    offset: u32,
    parent_seq: u32,
    seq: u32,
}

struct QuillChunkInfo {
    name: [u8; 4],
    id: u16,
    name2: [u8; 4],
    offset: u32,
    length: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_model::{PageOrientation, PageSize};
    use std::io::Cursor;

    #[test]
    fn test_export_empty_document() {
        let doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        let mut buffer = Cursor::new(Vec::new());
        
        let result = export_to_writer(&doc, &mut buffer, &PubExportOptions::default());
        assert!(result.is_ok());
        assert!(!buffer.into_inner().is_empty());
    }

    #[test]
    fn test_points_to_emus() {
        assert_eq!(points_to_emus(1.0), 12700);
        assert_eq!(points_to_emus(72.0), 914400); // 1 inch
    }

    /// A document with mixed shapes and a text frame must survive an
    /// export → import round trip with each shape's kind, fill and stroke,
    /// and without a text frame inheriting a neighbouring shape's fill.
    #[test]
    fn roundtrip_preserves_shape_kind_fill_and_stroke() {
        use repub_color::Color;
        use repub_model::geometry::ShapeGeometry;
        use repub_model::shape::{Shape, ShapeKind};
        use repub_model::styles::{Fill, Stroke};
        use repub_model::Rect;

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();

            let mut rect = Shape::rectangle(Rect::new(72.0, 72.0, 200.0, 100.0));
            rect.fill = Fill::Solid(Color::from_rgb8(220, 40, 40));
            rect.line = Some(Stroke::solid(Color::from_rgb8(0, 0, 0), 2.0));
            rect.common.rotation = 45.0;
            rect.common.flip_h = true;
            page.add_shape(rect);

            // A text frame placed between two shapes: it must keep its own fill.
            let mut text = Shape::text_frame_with_text(Rect::new(72.0, 300.0, 400.0, 80.0), "Hello");
            text.fill = Fill::Solid(Color::from_rgb8(250, 250, 180));
            page.add_shape(text);

            let mut ellipse = Shape::ellipse(Rect::new(300.0, 72.0, 120.0, 120.0));
            ellipse.fill = Fill::Solid(Color::from_rgb8(40, 160, 40));
            page.add_shape(ellipse);
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        let shapes: Vec<&Shape> = back
            .page(0)
            .unwrap()
            .layers()
            .iter()
            .flat_map(|l| l.shapes())
            .collect();
        assert_eq!(shapes.len(), 3, "all three shapes round-trip");

        let rect = shapes[0];
        assert!(
            matches!(
                &rect.kind,
                ShapeKind::AutoShape(a)
                    if matches!(a.geometry, ShapeGeometry::Rectangle { .. })
                        || a.preset.as_ref().map(|p| p.spt) == Some(1)
            ),
            "first shape is a rectangle"
        );
        assert_eq!(rect.fill.primary_color().to_rgba8(), (220, 40, 40, 255));
        assert_eq!(
            rect.line.as_ref().map(|l| l.color.to_rgba8()),
            Some((0, 0, 0, 255)),
            "rectangle keeps its stroke colour"
        );
        assert!(
            (rect.common.rotation - 45.0).abs() < 0.01,
            "rectangle keeps its 45° rotation, got {}",
            rect.common.rotation
        );
        assert!(rect.common.flip_h, "rectangle keeps its horizontal flip");
        assert!(!rect.common.flip_v);

        let text = shapes[1];
        assert!(matches!(text.kind, ShapeKind::TextFrame(_)), "second shape is text");
        assert_eq!(
            text.fill.primary_color().to_rgba8(),
            (250, 250, 180, 255),
            "text frame keeps its own fill, not a neighbour's"
        );

        let ellipse = shapes[2];
        assert!(
            matches!(
                &ellipse.kind,
                ShapeKind::AutoShape(a) if a.preset.as_ref().map(|p| p.spt) == Some(3)
            ),
            "third shape is an ellipse (spt 3)"
        );
        assert_eq!(ellipse.fill.primary_color().to_rgba8(), (40, 160, 40, 255));
    }

    /// Borderless, unfilled shapes are still real objects: they must survive a
    /// round trip rather than being mistaken for Publisher's invisible helper
    /// shapes (which set an explicit no-fill flag).
    #[test]
    fn roundtrip_keeps_borderless_unfilled_shapes() {
        use repub_model::shape::Shape;
        use repub_model::Rect;

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();
            page.add_shape(Shape::rectangle(Rect::new(72.0, 72.0, 100.0, 80.0)));
            page.add_shape(Shape::ellipse(Rect::new(300.0, 72.0, 100.0, 80.0)));
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        assert_eq!(
            back.total_shape_count(),
            2,
            "both borderless shapes survive the round trip"
        );
    }

    /// A line round-trips its stroke colour and width and its end arrow head.
    #[test]
    fn roundtrip_preserves_line_stroke_and_arrow() {
        use repub_color::Color;
        use repub_model::shape::{LineEndingStyle, Shape, ShapeKind};
        use repub_model::styles::{Fill, Stroke};
        use repub_model::{Point, Rect};

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();
            // A filled shape so the line is not the only object on the page.
            let mut rect = Shape::rectangle(Rect::new(72.0, 72.0, 100.0, 80.0));
            rect.fill = Fill::Solid(Color::from_rgb8(200, 0, 0));
            page.add_shape(rect);

            let mut line = Shape::line(Point::new(100.0, 400.0), Point::new(400.0, 500.0));
            line.line = Some(Stroke::solid(Color::from_rgb8(0, 0, 200), 2.5));
            if let ShapeKind::Line(l) = &mut line.kind {
                l.end_ending.style = LineEndingStyle::Arrow;
            }
            page.add_shape(line);
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        let line = back
            .page(0)
            .unwrap()
            .layers()
            .iter()
            .flat_map(|l| l.shapes())
            .find(|s| matches!(s.kind, ShapeKind::Line(_)))
            .expect("the line round-trips");

        let stroke = line.line.as_ref().expect("line keeps its stroke");
        assert_eq!(stroke.color.to_rgba8(), (0, 0, 200, 255));
        assert!((stroke.width - 2.5).abs() < 0.1, "stroke width, got {}", stroke.width);
        if let ShapeKind::Line(l) = &line.kind {
            assert_eq!(l.end_ending.style, LineEndingStyle::Arrow);
        }
    }

    /// A group round-trips as a group holding its children, and a standalone
    /// shape on the same page stays at the top level. Nested groups are
    /// preserved too.
    #[test]
    fn roundtrip_reconstructs_groups() {
        use repub_color::Color;
        use repub_model::shape::{Shape, ShapeKind};
        use repub_model::styles::Fill;
        use repub_model::Rect;

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();

            let mut red = Shape::rectangle(Rect::new(72.0, 72.0, 100.0, 80.0));
            red.fill = Fill::Solid(Color::from_rgb8(200, 0, 0));
            let mut blue = Shape::ellipse(Rect::new(200.0, 72.0, 100.0, 80.0));
            blue.fill = Fill::Solid(Color::from_rgb8(0, 0, 200));
            let inner = Shape::group_from(vec![red, blue]);

            let mut green = Shape::rectangle(Rect::new(72.0, 200.0, 100.0, 80.0));
            green.fill = Fill::Solid(Color::from_rgb8(0, 200, 0));
            page.add_shape(Shape::group_from(vec![inner, green]));

            // A standalone shape that must not be absorbed into the group.
            let mut solo = Shape::rectangle(Rect::new(300.0, 300.0, 120.0, 90.0));
            solo.fill = Fill::Solid(Color::from_rgb8(100, 100, 100));
            page.add_shape(solo);
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        let top: Vec<&Shape> = back
            .page(0)
            .unwrap()
            .layers()
            .iter()
            .flat_map(|l| l.shapes())
            .collect();
        assert_eq!(top.len(), 2, "the outer group and the standalone shape");

        let group = top
            .iter()
            .find_map(|s| match &s.kind {
                ShapeKind::Group(g) => Some(g),
                _ => None,
            })
            .expect("the outer group round-trips");
        assert_eq!(group.children.len(), 2, "outer group keeps its two children");

        let nested = group
            .children
            .iter()
            .find_map(|c| match &c.kind {
                ShapeKind::Group(g) => Some(g),
                _ => None,
            })
            .expect("the inner group is still nested");
        assert_eq!(nested.children.len(), 2, "inner group keeps both shapes");
    }

    /// An embedded PNG picture round-trips as a picture whose image bytes are
    /// recovered intact from the Escher blip store.
    #[test]
    fn roundtrip_preserves_embedded_picture() {
        use base64::Engine;
        use repub_model::shape::{ImageSource, Shape, ShapeKind};
        use repub_model::Rect;

        // A minimal 1x1 PNG.
        const PNG_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAC0lEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";
        let png = base64::engine::general_purpose::STANDARD
            .decode(PNG_B64)
            .unwrap();

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();
            let mut pic = Shape::picture(Rect::new(72.0, 72.0, 200.0, 150.0));
            if let ShapeKind::Picture(data) = &mut pic.kind {
                data.source = ImageSource::Embedded {
                    data: base64::engine::general_purpose::STANDARD.encode(&png),
                    mime_type: "image/png".to_string(),
                };
            }
            page.add_shape(pic);
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        let pic = back
            .page(0)
            .unwrap()
            .layers()
            .iter()
            .flat_map(|l| l.shapes())
            .find_map(|s| match &s.kind {
                ShapeKind::Picture(d) => Some(d.clone()),
                _ => None,
            })
            .expect("the picture round-trips");

        match pic.source {
            ImageSource::Embedded { data, mime_type } => {
                assert_eq!(mime_type, "image/png");
                let got = base64::engine::general_purpose::STANDARD
                    .decode(&data)
                    .unwrap();
                assert_eq!(got, png, "image bytes are recovered intact");
            }
            other => panic!("picture source is not embedded: {other:?}"),
        }
    }

    /// A table round-trips its row/column structure and per-cell text.
    #[test]
    fn roundtrip_reconstructs_table() {
        use repub_model::shape::{Shape, ShapeKind};
        use repub_model::Rect;

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();
            let mut t = Shape::table(Rect::new(72.0, 72.0, 300.0, 200.0), 2, 3);
            if let ShapeKind::Table(tbl) = &mut t.kind {
                tbl.set_cell_text(0, 0, "A1");
                tbl.set_cell_text(0, 1, "B1");
                tbl.set_cell_text(1, 2, "C2");
            }
            page.add_shape(t);
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        let table = back
            .page(0)
            .unwrap()
            .layers()
            .iter()
            .flat_map(|l| l.shapes())
            .find_map(|s| match &s.kind {
                ShapeKind::Table(t) => Some(t.clone()),
                _ => None,
            })
            .expect("the table round-trips");

        assert_eq!((table.rows, table.cols), (2, 3), "table keeps its grid");
        let cell_text = |r: u32, c: u32| {
            table
                .cells
                .get((r * table.cols + c) as usize)
                .map(|cell| cell.content.plain_text())
                .unwrap_or_default()
        };
        assert_eq!(cell_text(0, 0), "A1");
        assert_eq!(cell_text(0, 1), "B1");
        assert_eq!(cell_text(1, 2), "C2");
    }

    /// A text frame with several paragraphs keeps them all on re-import.
    #[test]
    fn roundtrip_preserves_text_frame_paragraphs() {
        use repub_model::shape::{Shape, ShapeKind};
        use repub_model::text::{Paragraph, StyledText};
        use repub_model::Rect;

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();
            let mut tf = Shape::text_frame_with_text(Rect::new(72.0, 72.0, 400.0, 200.0), "");
            if let ShapeKind::TextFrame(frame) = &mut tf.kind {
                frame.content = StyledText {
                    paragraphs: vec![
                        Paragraph::plain("First paragraph"),
                        Paragraph::plain("Second paragraph"),
                        Paragraph::plain("Third"),
                    ],
                };
            }
            page.add_shape(tf);
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        let frame = back
            .page(0)
            .unwrap()
            .layers()
            .iter()
            .flat_map(|l| l.shapes())
            .find_map(|s| match &s.kind {
                ShapeKind::TextFrame(f) => Some(f.clone()),
                _ => None,
            })
            .expect("the text frame round-trips");

        let texts: Vec<String> = frame
            .content
            .paragraphs
            .iter()
            .map(|p| p.plain_text())
            .collect();
        assert_eq!(
            texts,
            vec!["First paragraph", "Second paragraph", "Third"],
            "all three paragraphs survive"
        );
    }

    /// A WordArt / text-effect shape round-trips as a text effect carrying its
    /// string.
    #[test]
    fn roundtrip_preserves_text_effect() {
        use repub_model::shape::{Shape, ShapeKind};
        use repub_model::Rect;

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();
            page.add_shape(Shape::text_effect(
                Rect::new(72.0, 72.0, 300.0, 100.0),
                "Fancy Text",
            ));
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        let text = back
            .page(0)
            .unwrap()
            .layers()
            .iter()
            .flat_map(|l| l.shapes())
            .find_map(|s| match &s.kind {
                ShapeKind::TextEffect(te) => Some(te.text.clone()),
                _ => None,
            })
            .expect("the text effect round-trips");
        assert_eq!(text, "Fancy Text");
    }

    /// A rounded rectangle round-trips its corner radius as the ROUND_RECT
    /// preset's adjust value.
    #[test]
    fn roundtrip_preserves_rounded_rectangle_adjust() {
        use repub_model::shape::{Shape, ShapeKind};
        use repub_model::Rect;

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();
            // 20pt radius on a 200x100 rectangle: 20 / min(200,100) = 0.2.
            page.add_shape(Shape::rounded_rectangle(
                Rect::new(72.0, 72.0, 200.0, 100.0),
                20.0,
            ));
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        let preset = back
            .page(0)
            .unwrap()
            .layers()
            .iter()
            .flat_map(|l| l.shapes())
            .find_map(|s| match &s.kind {
                ShapeKind::AutoShape(a) => a.preset.clone(),
                _ => None,
            })
            .expect("the rounded rectangle keeps its preset");

        assert_eq!(preset.spt, 2, "ROUND_RECT preset type");
        assert_eq!(
            preset.adjust.first().copied().flatten(),
            Some(4320),
            "corner radius 0.2 * 21600"
        );
    }

    /// A shape's drop shadow round-trips its colour and offset.
    #[test]
    fn roundtrip_preserves_drop_shadow() {
        use repub_color::Color;
        use repub_model::shape::Shape;
        use repub_model::styles::{DropShadow, Fill};
        use repub_model::Rect;

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();
            let mut s = Shape::rectangle(Rect::new(72.0, 72.0, 200.0, 100.0));
            s.fill = Fill::Solid(Color::from_rgb8(200, 0, 0));
            s.effects.drop_shadow = Some(DropShadow {
                color: Color::from_rgb8(50, 50, 50),
                offset_x: 4.0,
                offset_y: 6.0,
                blur_radius: 4.0,
                spread: 0.0,
            });
            page.add_shape(s);
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        let shadow = back
            .page(0)
            .unwrap()
            .layers()
            .iter()
            .flat_map(|l| l.shapes())
            .find_map(|s| s.effects.drop_shadow.clone())
            .expect("the drop shadow round-trips");

        assert_eq!(shadow.color.to_rgba8().0, 50);
        assert_eq!(shadow.color.to_rgba8().1, 50);
        assert_eq!(shadow.color.to_rgba8().2, 50);
        assert!((shadow.offset_x - 4.0).abs() < 0.5, "x offset, got {}", shadow.offset_x);
        assert!((shadow.offset_y - 6.0).abs() < 0.5, "y offset, got {}", shadow.offset_y);
    }

    /// Linear and radial gradient fills round-trip their end-stop colours.
    #[test]
    fn roundtrip_preserves_gradient_fills() {
        use repub_color::Color;
        use repub_model::shape::{Shape, ShapeKind};
        use repub_model::styles::{Fill, GradientStop, LinearGradient, RadialGradient};
        use repub_model::{Point, Rect};

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();

            let mut linear = Shape::rectangle(Rect::new(72.0, 72.0, 200.0, 100.0));
            linear.fill = Fill::LinearGradient(LinearGradient {
                start: Point::new(0.0, 0.0),
                end: Point::new(1.0, 0.0),
                stops: vec![
                    GradientStop::new(0.0, Color::from_rgb8(255, 0, 0)),
                    GradientStop::new(1.0, Color::from_rgb8(0, 0, 255)),
                ],
            });
            page.add_shape(linear);

            let mut radial = Shape::ellipse(Rect::new(300.0, 72.0, 120.0, 120.0));
            radial.fill = Fill::RadialGradient(RadialGradient::centered(
                Color::from_rgb8(0, 255, 0),
                Color::from_rgb8(0, 0, 0),
            ));
            page.add_shape(radial);
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        let fills: Vec<Fill> = back
            .page(0)
            .unwrap()
            .layers()
            .iter()
            .flat_map(|l| l.shapes())
            .filter_map(|s| match &s.kind {
                ShapeKind::AutoShape(_) => Some(s.fill.clone()),
                _ => None,
            })
            .collect();

        let linear = fills
            .iter()
            .find_map(|f| match f {
                Fill::LinearGradient(g) => Some(g),
                _ => None,
            })
            .expect("linear gradient round-trips");
        assert_eq!(linear.stops.first().unwrap().color.to_rgba8(), (255, 0, 0, 255));
        assert_eq!(linear.stops.last().unwrap().color.to_rgba8(), (0, 0, 255, 255));

        let radial = fills
            .iter()
            .find_map(|f| match f {
                Fill::RadialGradient(g) => Some(g),
                _ => None,
            })
            .expect("radial gradient round-trips");
        assert_eq!(radial.stops.first().unwrap().color.to_rgba8(), (0, 255, 0, 255));
        assert_eq!(radial.stops.last().unwrap().color.to_rgba8(), (0, 0, 0, 255));
    }

    /// A text frame's column layout and internal margins round-trip.
    #[test]
    fn roundtrip_preserves_text_frame_columns_and_insets() {
        use repub_model::shape::{ColumnSpec, Margins, Shape, ShapeKind};
        use repub_model::Rect;

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();
            let mut tf = Shape::text_frame_with_text(Rect::new(72.0, 72.0, 400.0, 200.0), "Hello");
            if let ShapeKind::TextFrame(f) = &mut tf.kind {
                f.columns = ColumnSpec::equal(3, 18.0);
                f.margins = Margins {
                    left: 5.0,
                    top: 3.0,
                    right: 5.0,
                    bottom: 3.0,
                };
            }
            page.add_shape(tf);
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        let frame = back
            .page(0)
            .unwrap()
            .layers()
            .iter()
            .flat_map(|l| l.shapes())
            .find_map(|s| match &s.kind {
                ShapeKind::TextFrame(f) => Some(f.clone()),
                _ => None,
            })
            .expect("the text frame round-trips");

        assert_eq!(frame.columns.count, 3, "column count");
        assert!((frame.columns.gap - 18.0).abs() < 0.5, "gap, got {}", frame.columns.gap);
        assert!((frame.margins.left - 5.0).abs() < 0.5, "left margin");
        assert!((frame.margins.top - 3.0).abs() < 0.5, "top margin");
    }

    /// A freeform custom path round-trips its command structure (move, lines,
    /// close) via the pVertices / pSegments complex properties.
    #[test]
    fn roundtrip_preserves_custom_path() {
        use repub_model::geometry::{AutoShape, Path, PathCommand, ShapeGeometry};
        use repub_model::shape::{Shape, ShapeKind};
        use repub_model::{Point, Rect};

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();
            let mut path = Path::new();
            path.commands = vec![
                PathCommand::MoveTo(Point::new(100.0, 200.0)),
                PathCommand::LineTo(Point::new(150.0, 100.0)),
                PathCommand::LineTo(Point::new(200.0, 200.0)),
                PathCommand::Close,
            ];
            page.add_shape(Shape::new(
                Rect::new(100.0, 100.0, 100.0, 100.0),
                ShapeKind::AutoShape(AutoShape {
                    geometry: ShapeGeometry::Path(path),
                    preset: None,
                }),
            ));
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        let path = back
            .page(0)
            .unwrap()
            .layers()
            .iter()
            .flat_map(|l| l.shapes())
            .find_map(|s| match &s.kind {
                ShapeKind::AutoShape(a) => match &a.geometry {
                    ShapeGeometry::Path(p) => Some(p.clone()),
                    _ => None,
                },
                _ => None,
            })
            .expect("the freeform path round-trips");

        let kinds: Vec<&str> = path
            .commands
            .iter()
            .map(|c| match c {
                PathCommand::MoveTo(_) => "M",
                PathCommand::LineTo(_) => "L",
                PathCommand::CubicTo { .. } => "C",
                PathCommand::QuadTo { .. } => "Q",
                PathCommand::Close => "Z",
            })
            .collect();
        assert_eq!(kinds, vec!["M", "L", "L", "Z"], "triangle outline survives");
    }

    /// Character formatting (bold, italic, underline, size) round-trips per run
    /// through the Quill FDPC chunk.
    #[test]
    fn roundtrip_preserves_character_formatting() {
        use repub_model::shape::{Shape, ShapeKind};
        use repub_model::styles::{CharacterStyle, FontStyle, FontWeight, UnderlineStyle};
        use repub_model::text::{InlineElement, Paragraph, StyledText, TextRun};
        use repub_model::Rect;

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();
            let mut tf = Shape::text_frame_with_text(Rect::new(72.0, 72.0, 400.0, 200.0), "");
            if let ShapeKind::TextFrame(frame) = &mut tf.kind {
                let mut bold = CharacterStyle::default();
                bold.font.weight = FontWeight::Bold;
                bold.font_size = 18.0;
                let mut ital = CharacterStyle::default();
                ital.font.style = FontStyle::Italic;
                ital.underline = UnderlineStyle::Single;
                let mut para = Paragraph::new();
                para.elements = vec![
                    InlineElement::Text(TextRun::styled("Bold18", bold)),
                    InlineElement::Text(TextRun::new("plain")),
                    InlineElement::Text(TextRun::styled("ItalUnder", ital)),
                ];
                frame.content = StyledText {
                    paragraphs: vec![para],
                };
            }
            page.add_shape(tf);
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        let frame = back
            .page(0)
            .unwrap()
            .layers()
            .iter()
            .flat_map(|l| l.shapes())
            .find_map(|s| match &s.kind {
                ShapeKind::TextFrame(f) => Some(f.clone()),
                _ => None,
            })
            .expect("the text frame round-trips");

        let runs: Vec<(String, bool, bool, bool, f64)> = frame
            .content
            .paragraphs
            .iter()
            .flat_map(|p| p.elements.iter())
            .filter_map(|el| match el {
                InlineElement::Text(r) => Some((
                    r.text.clone(),
                    r.style.font.weight == FontWeight::Bold,
                    r.style.font.style == FontStyle::Italic,
                    r.style.underline != UnderlineStyle::None,
                    r.style.font_size,
                )),
                _ => None,
            })
            .collect();

        let bold = runs.iter().find(|r| r.0 == "Bold18").expect("bold run");
        assert!(bold.1, "bold");
        assert!((bold.4 - 18.0).abs() < 0.5, "size 18, got {}", bold.4);

        let ital = runs.iter().find(|r| r.0 == "ItalUnder").expect("italic run");
        assert!(ital.2, "italic");
        assert!(ital.3, "underline");
    }

    /// Paragraph alignment round-trips per paragraph through the FDPP style
    /// records.
    #[test]
    fn roundtrip_preserves_paragraph_alignment() {
        use repub_model::shape::{Shape, ShapeKind};
        use repub_model::styles::TextAlignment;
        use repub_model::text::{Paragraph, StyledText};
        use repub_model::Rect;

        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        {
            let page = doc.page_mut(0).unwrap();
            let mut tf = Shape::text_frame_with_text(Rect::new(72.0, 72.0, 400.0, 200.0), "");
            if let ShapeKind::TextFrame(frame) = &mut tf.kind {
                let mut c = Paragraph::plain("Centered");
                c.style.alignment = TextAlignment::Center;
                let mut r = Paragraph::plain("Right");
                r.style.alignment = TextAlignment::Right;
                frame.content = StyledText {
                    paragraphs: vec![c, r, Paragraph::plain("Left default")],
                };
            }
            page.add_shape(tf);
        }

        let bytes = export_bytes(&doc, &PubExportOptions::default()).expect("export");
        let back = crate::import_bytes_lenient(&bytes).expect("reimport");

        let frame = back
            .page(0)
            .unwrap()
            .layers()
            .iter()
            .flat_map(|l| l.shapes())
            .find_map(|s| match &s.kind {
                ShapeKind::TextFrame(f) => Some(f.clone()),
                _ => None,
            })
            .expect("the text frame round-trips");

        let aligns: Vec<(String, TextAlignment)> = frame
            .content
            .paragraphs
            .iter()
            .map(|p| (p.plain_text(), p.style.alignment))
            .collect();
        assert!(aligns.iter().any(|(t, a)| t == "Centered" && *a == TextAlignment::Center));
        assert!(aligns.iter().any(|(t, a)| t == "Right" && *a == TextAlignment::Right));
        assert!(aligns.iter().any(|(t, a)| t == "Left default" && *a == TextAlignment::Left));
    }
}
