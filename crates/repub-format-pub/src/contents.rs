//! Microsoft Publisher Contents stream parser.
//!
//! This module parses the Contents stream from Microsoft Publisher 2002+ (2k2) files.
//! The Contents stream contains the document structure: pages, shapes, tables, fonts,
//! and their relationships.
//!
//! # Format Overview
//!
//! The Contents stream uses a block-based structure:
//! - Offset 0x1A: 4-byte trailer offset
//! - Trailer: length + 3 sub-blocks, one being the TRAILER_DIRECTORY (type 0x8A)
//! - Directory: contains content chunk references
//!
//! Each block has a type that determines its data length:
//! - 0x05, 0x08, 0x0A: 0-byte data
//! - 0x10, 0x12, 0x18, 0x1A, 0x07: 2-byte data
//! - 0x20, 0x22, 0x58, 0x68, 0x70, 0xB8: 4-byte data
//! - 0x28: 8-byte data
//! - 0x38: 16-byte data
//! - 0x48: 24-byte data
//! - 0x80, 0x82, 0x8A, 0x90, 0x98, 0xA0, 0xC0, 0x88: variable length (first DWORD is length)
//!
//! # Reference
//!
//! Based on libmspub's MSPUBParser.cpp implementation.

use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};

use crate::error::{FormatError, FormatResult};
use crate::pub_import::emus_to_points;

// Block types that determine data length
mod block_type {
    #![allow(dead_code)]
    pub const TRAILER_DIRECTORY: u8 = 0x90;
    pub const GENERAL_CONTAINER: u8 = 0x88;
    pub const STRING_CONTAINER: u8 = 0xC0;
    pub const DUMMY: u8 = 0x78;
    pub const SHAPE_SEQNUM: u8 = 0x70;
}

// Block IDs (context-dependent meanings)
mod block_id {
    #![allow(dead_code)]
    // Document chunk
    pub const DOCUMENT_SIZE: u32 = 0x12;
    pub const DOCUMENT_WIDTH: u32 = 0x01;
    pub const DOCUMENT_HEIGHT: u32 = 0x02;
    pub const DOCUMENT_PAGE_LIST: u32 = 0x02;

    // Page chunk
    pub const PAGE_BG_SHAPE: u32 = 0x0A;
    pub const PAGE_SHAPES: u32 = 0x02;
    pub const THIS_MASTER_NAME: u32 = 0x0E;
    pub const APPLIED_MASTER_NAME: u32 = 0x0D;

    // Shape chunk
    pub const SHAPE_WIDTH: u32 = 0xAA;
    pub const SHAPE_HEIGHT: u32 = 0xAB;
    pub const SHAPE_TEXT_ID: u32 = 0x27;
    pub const SHAPE_VALIGN: u32 = 0x35;
    pub const SHAPE_CROP: u32 = 0xB7;
    pub const SHAPE_BORDER_IMAGE_ID: u32 = 0x09;
    pub const SHAPE_DONT_STRETCH_BA: u32 = 0x07;

    // Table chunk
    pub const TABLE_CELLS_SEQNUM: u32 = 0x6B;
    pub const TABLE_NUM_ROWS: u32 = 0x66;
    pub const TABLE_NUM_COLS: u32 = 0x67;
    pub const TABLE_ROWCOL_ARRAY: u32 = 0x6D;

    // Chunk reference
    pub const CHUNK_TYPE: u32 = 0x02;
    pub const CHUNK_OFFSET: u32 = 0x04;
    pub const CHUNK_PARENT_SEQNUM: u32 = 0x05;
}

// Content chunk types (from MSPUBContentChunkType.h)
mod chunk_type {
    #![allow(dead_code)]
    pub const UNKNOWN: u32 = 0x00;
    pub const SHAPE: u32 = 0x01;
    pub const TABLE: u32 = 0x10;
    pub const ALTSHAPE: u32 = 0x20;
    pub const GROUP: u32 = 0x30;
    pub const LOGO: u32 = 0x31;
    pub const PAGE: u32 = 0x43;
    pub const DOCUMENT: u32 = 0x44;
    pub const BORDER_ART: u32 = 0x46;
    pub const PALETTE: u32 = 0x5C;
    pub const CELLS: u32 = 0x63;
    pub const FONT: u32 = 0x6C;
}

/// Parsed document from Contents stream.
#[derive(Debug, Clone)]
pub struct PubDocument {
    /// Document width in points.
    pub width: f64,
    /// Document height in points.
    pub height: f64,
    /// Pages in document order.
    pub pages: Vec<PubPage>,
    /// All shapes indexed by sequence number.
    pub shapes: HashMap<u32, PubShape>,
}

/// A page in the document.
#[derive(Debug, Clone)]
pub struct PubPage {
    /// Sequence number for this page.
    pub seq_num: u32,
    /// Whether this is a master page.
    pub is_master: bool,
    /// Master page name (if this is a master).
    pub master_name: Option<String>,
    /// Applied master page sequence number.
    pub applied_master: Option<u32>,
    /// Background shape sequence number.
    pub bg_shape: Option<u32>,
    /// Shape sequence numbers on this page.
    pub shape_refs: Vec<u32>,
}

/// A shape in the document.
#[derive(Debug, Clone)]
pub struct PubShape {
    /// Sequence number for this shape.
    pub seq_num: u32,
    /// Shape width in points.
    pub width: f64,
    /// Shape height in points.
    pub height: f64,
    /// Text ID (reference to Quill text block).
    pub text_id: Option<u32>,
    /// Vertical text alignment.
    pub valign: Option<VerticalAlign>,
    /// Whether this is a table shape.
    pub is_table: bool,
    /// Table information (if is_table is true).
    pub table_info: Option<TableInfo>,
}

/// Table structure information.
#[derive(Debug, Clone)]
pub struct TableInfo {
    /// Number of rows.
    pub rows: u32,
    /// Number of columns.
    pub cols: u32,
    /// Sequence number of cells chunk.
    pub cells_seq_num: u32,
}

/// Vertical text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerticalAlign {
    /// Align to top.
    Top,
    /// Align to middle.
    Middle,
    /// Align to bottom.
    Bottom,
}

impl From<u32> for VerticalAlign {
    fn from(value: u32) -> Self {
        match value {
            1 => VerticalAlign::Middle,
            2 => VerticalAlign::Bottom,
            _ => VerticalAlign::Top,
        }
    }
}

/// Internal block info from parsing.
#[derive(Debug, Clone)]
struct BlockInfo {
    id: u32,
    block_type: u8,
    data_offset: u64,
    data_length: u64,
    data: u32, // For simple types, the data value
    string_data: Vec<u8>,
}

/// Internal content chunk reference.
#[derive(Debug, Clone)]
struct ContentChunkRef {
    chunk_type: u32,
    offset: u64,
    end: u64,
    seq_num: u32,
    parent_seq_num: u32,
}

/// Parser for the Contents stream.
pub struct ContentsParser<R> {
    reader: R,
    #[allow(dead_code)]
    length: u64,
}

impl<R: Read + Seek> ContentsParser<R> {
    /// Create a new parser from a reader containing the Contents stream.
    pub fn new(mut reader: R) -> FormatResult<Self> {
        let length = reader.seek(SeekFrom::End(0))?;
        reader.seek(SeekFrom::Start(0))?;
        Ok(Self { reader, length })
    }

    /// Parse the Contents stream and return the document structure.
    pub fn parse(&mut self) -> FormatResult<PubDocument> {
        // Read trailer offset at 0x1A
        self.reader.seek(SeekFrom::Start(0x1A))?;
        let trailer_offset = self.read_u32()? as u64;

        log::debug!("Trailer offset: 0x{:X}", trailer_offset);

        // Parse trailer
        self.reader.seek(SeekFrom::Start(trailer_offset))?;
        let trailer_length = self.read_u32()? as u64;

        log::debug!("Trailer length: {}", trailer_length);

        // Parse the 3 trailer sub-blocks
        let mut content_chunks: Vec<ContentChunkRef> = Vec::new();
        let mut document_chunk_idx: Option<usize> = None;
        let mut page_chunk_indices: Vec<usize> = Vec::new();
        let mut shape_chunk_indices: Vec<usize> = Vec::new();

        let trailer_end = trailer_offset + trailer_length;
        let mut block_count = 0;

        while self.reader.stream_position()? < trailer_end && block_count < 10 {
            let block = self.parse_block()?;
            block_count += 1;
            log::debug!(
                "Trailer sub-block {}: id={}, type=0x{:02X}, data_offset=0x{:X}, data_len={}",
                block_count,
                block.id,
                block.block_type,
                block.data_offset,
                block.data_length
            );

            // Check for TRAILER_DIRECTORY (type 0x90 is variable-length container)
            if block.block_type == 0x90 || block.block_type == 0x8A {
                // Parse directory contents
                let dir_end = block.data_offset + block.data_length;
                let mut seq_num: i32 = -1;

                while self.reader.stream_position()? < dir_end {
                    let entry_block = self.parse_block()?;
                    seq_num += 1;

                    if entry_block.block_type == block_type::GENERAL_CONTAINER {
                        // Parse chunk reference
                        if let Some(chunk_ref) =
                            self.parse_content_chunk_reference(&entry_block, seq_num as u32)?
                        {
                            let idx = content_chunks.len();

                            // Update previous chunk's end offset
                            if !content_chunks.is_empty() {
                                content_chunks.last_mut().unwrap().end = chunk_ref.offset;
                            }

                            // Track chunk by type
                            match chunk_ref.chunk_type {
                                chunk_type::DOCUMENT => {
                                    log::trace!("Found DOCUMENT chunk at idx {}", idx);
                                    document_chunk_idx = Some(idx);
                                }
                                chunk_type::PAGE => {
                                    log::trace!("Found PAGE chunk at idx {}", idx);
                                    page_chunk_indices.push(idx);
                                }
                                chunk_type::SHAPE => {
                                    log::trace!(
                                        "Found SHAPE chunk at idx {} (seq={})",
                                        idx,
                                        chunk_ref.seq_num
                                    );
                                    shape_chunk_indices.push(idx);
                                }
                                chunk_type::TABLE => {
                                    log::trace!(
                                        "Found TABLE chunk at idx {} (seq={})",
                                        idx,
                                        chunk_ref.seq_num
                                    );
                                    shape_chunk_indices.push(idx);
                                }
                                chunk_type::GROUP | chunk_type::LOGO => {
                                    log::trace!(
                                        "Found GROUP/LOGO chunk at idx {} (seq={})",
                                        idx,
                                        chunk_ref.seq_num
                                    );
                                    shape_chunk_indices.push(idx);
                                }
                                chunk_type::ALTSHAPE => {
                                    log::trace!(
                                        "Found ALTSHAPE chunk at idx {} (seq={})",
                                        idx,
                                        chunk_ref.seq_num
                                    );
                                    shape_chunk_indices.push(idx);
                                }
                                _ => {
                                    log::trace!(
                                        "Found UNKNOWN chunk type 0x{:02X} at idx {} (seq={})",
                                        chunk_ref.chunk_type,
                                        idx,
                                        chunk_ref.seq_num
                                    );
                                }
                            }

                            content_chunks.push(chunk_ref);
                        }
                    } else {
                        self.skip_block(&entry_block)?;
                    }
                }

                // Set last chunk's end
                if !content_chunks.is_empty() {
                    content_chunks.last_mut().unwrap().end = dir_end;
                }
            }
        }

        // Parse document chunk for width/height and page list
        let (width, height, page_seq_nums) = if let Some(doc_idx) = document_chunk_idx {
            let doc_chunk = &content_chunks[doc_idx];
            self.parse_document_chunk(doc_chunk)?
        } else {
            return Err(FormatError::InvalidStructure(
                "No document chunk found in Contents stream".to_string(),
            ));
        };

        log::debug!(
            "Document size: {}x{} points, {} pages",
            width,
            height,
            page_seq_nums.len()
        );

        // Parse all shape chunks
        let mut shapes: HashMap<u32, PubShape> = HashMap::new();
        for &idx in &shape_chunk_indices {
            let chunk = &content_chunks[idx];
            if let Some(shape) = self.parse_shape_chunk(chunk)? {
                shapes.insert(shape.seq_num, shape);
            }
        }

        // Parse page chunks
        let mut pages: Vec<PubPage> = Vec::new();
        let mut page_by_seq: HashMap<u32, PubPage> = HashMap::new();

        for &idx in &page_chunk_indices {
            let chunk = &content_chunks[idx];
            if let Some(page) = self.parse_page_chunk(chunk)? {
                page_by_seq.insert(page.seq_num, page);
            }
        }

        // Order pages by the sequence from document chunk
        for seq_num in page_seq_nums {
            if let Some(page) = page_by_seq.remove(&seq_num) {
                pages.push(page);
            }
        }

        // Add any remaining pages not in the list
        for (_, page) in page_by_seq {
            pages.push(page);
        }

        Ok(PubDocument {
            width,
            height,
            pages,
            shapes,
        })
    }

    /// Parse a content chunk reference from a GENERAL_CONTAINER block.
    fn parse_content_chunk_reference(
        &mut self,
        block: &BlockInfo,
        seq_num: u32,
    ) -> FormatResult<Option<ContentChunkRef>> {
        let end = block.data_offset + block.data_length;
        self.reader.seek(SeekFrom::Start(block.data_offset + 4))?; // Skip container length

        let mut chunk_type: Option<u32> = None;
        let mut offset: Option<u64> = None;
        let mut parent_seq_num: u32 = 0;

        while self.reader.stream_position()? < end {
            let sub_block = self.parse_block()?;

            match sub_block.id {
                block_id::CHUNK_TYPE => {
                    chunk_type = Some(sub_block.data);
                }
                block_id::CHUNK_OFFSET => {
                    offset = Some(sub_block.data as u64);
                }
                block_id::CHUNK_PARENT_SEQNUM => {
                    parent_seq_num = sub_block.data;
                }
                _ => {}
            }
        }

        if let (Some(ct), Some(off)) = (chunk_type, offset) {
            log::trace!(
                "Chunk ref: type=0x{:02X}, offset=0x{:X}, seq={}, parent={}",
                ct,
                off,
                seq_num,
                parent_seq_num
            );
            Ok(Some(ContentChunkRef {
                chunk_type: ct,
                offset: off,
                end: 0, // Will be set later
                seq_num,
                parent_seq_num,
            }))
        } else {
            Ok(None)
        }
    }

    /// Parse the document chunk to get dimensions and page list.
    fn parse_document_chunk(
        &mut self,
        chunk: &ContentChunkRef,
    ) -> FormatResult<(f64, f64, Vec<u32>)> {
        self.reader.seek(SeekFrom::Start(chunk.offset))?;
        let len = self.read_u32()? as u64;
        let chunk_end = chunk.offset + len;

        log::debug!(
            "Document chunk at 0x{:X}, len={}, end=0x{:X}",
            chunk.offset,
            len,
            chunk_end
        );

        let mut width_emu: u32 = 0;
        let mut height_emu: u32 = 0;
        let mut page_seq_nums: Vec<u32> = Vec::new();

        while self.reader.stream_position()? < chunk_end {
            let block = self.parse_block()?;

            log::trace!(
                "Doc block: id=0x{:02X}, type=0x{:02X}, data={}",
                block.id,
                block.block_type,
                block.data
            );

            if block.id == block_id::DOCUMENT_SIZE {
                // Parse size container
                let size_end = block.data_offset + block.data_length;
                self.reader.seek(SeekFrom::Start(block.data_offset + 4))?;

                while self.reader.stream_position()? < size_end {
                    let sub_block = self.parse_block()?;
                    log::trace!(
                        "  Size sub-block: id=0x{:02X}, type=0x{:02X}, data={}",
                        sub_block.id,
                        sub_block.block_type,
                        sub_block.data
                    );
                    match sub_block.id {
                        block_id::DOCUMENT_WIDTH => {
                            width_emu = sub_block.data;
                            log::debug!(
                                "Width EMU: {} ({} points)",
                                width_emu,
                                emus_to_points(width_emu as i64)
                            );
                        }
                        block_id::DOCUMENT_HEIGHT => {
                            height_emu = sub_block.data;
                            log::debug!(
                                "Height EMU: {} ({} points)",
                                height_emu,
                                emus_to_points(height_emu as i64)
                            );
                        }
                        _ => {}
                    }
                }
            } else if block.id == block_id::DOCUMENT_PAGE_LIST {
                // Parse page list
                let list_end = block.data_offset + block.data_length;
                self.reader.seek(SeekFrom::Start(block.data_offset + 4))?;

                while self.reader.stream_position()? < list_end {
                    let sub_block = self.parse_block()?;
                    if sub_block.id == 0 {
                        page_seq_nums.push(sub_block.data);
                    }
                }
            } else {
                self.skip_block(&block)?;
            }
        }

        let width = emus_to_points(width_emu as i64);
        let height = emus_to_points(height_emu as i64);

        Ok((width, height, page_seq_nums))
    }

    /// Parse a page chunk.
    fn parse_page_chunk(&mut self, chunk: &ContentChunkRef) -> FormatResult<Option<PubPage>> {
        self.reader.seek(SeekFrom::Start(chunk.offset))?;
        let len = self.read_u32()? as u64;
        let chunk_end = chunk.offset + len;

        let mut is_master = false;
        let mut master_name: Option<String> = None;
        let mut applied_master: Option<u32> = None;
        let mut bg_shape: Option<u32> = None;
        let mut shape_refs: Vec<u32> = Vec::new();

        while self.reader.stream_position()? < chunk_end {
            let block = self.parse_block()?;

            match block.id {
                block_id::PAGE_BG_SHAPE => {
                    bg_shape = Some(block.data);
                }
                block_id::PAGE_SHAPES => {
                    // Parse shape list
                    let list_end = block.data_offset + block.data_length;
                    while self.reader.stream_position()? < list_end {
                        let sub_block = self.parse_block()?;
                        if sub_block.block_type == block_type::SHAPE_SEQNUM {
                            shape_refs.push(sub_block.data);
                        }
                    }
                }
                block_id::THIS_MASTER_NAME => {
                    // Check if any non-null bytes in string data
                    if block.string_data.iter().any(|&b| b != 0) {
                        is_master = true;
                        master_name = Some(decode_utf16le(&block.string_data));
                    }
                }
                block_id::APPLIED_MASTER_NAME => {
                    applied_master = Some(block.data);
                }
                _ => {
                    self.skip_block(&block)?;
                }
            }
        }

        log::debug!(
            "Page seq={}: is_master={}, shapes={:?}",
            chunk.seq_num,
            is_master,
            shape_refs
        );

        Ok(Some(PubPage {
            seq_num: chunk.seq_num,
            is_master,
            master_name,
            applied_master,
            bg_shape,
            shape_refs,
        }))
    }

    /// Parse a shape or table chunk.
    fn parse_shape_chunk(&mut self, chunk: &ContentChunkRef) -> FormatResult<Option<PubShape>> {
        self.reader.seek(SeekFrom::Start(chunk.offset))?;
        let len = self.read_u32()? as u64;
        let chunk_end = chunk.offset + len;

        let is_table = chunk.chunk_type == chunk_type::TABLE;

        let mut width_emu: u32 = 0;
        let mut height_emu: u32 = 0;
        let mut text_id: Option<u32> = None;
        let mut valign: Option<VerticalAlign> = None;
        let mut table_info: Option<TableInfo> = None;

        // For tables, track these
        let mut cells_seq_num: Option<u32> = None;
        let mut num_rows: Option<u32> = None;
        let mut num_cols: Option<u32> = None;

        while self.reader.stream_position()? < chunk_end {
            let block = self.parse_block()?;

            match block.id {
                block_id::SHAPE_WIDTH => {
                    width_emu = block.data;
                }
                block_id::SHAPE_HEIGHT => {
                    height_emu = block.data;
                }
                block_id::SHAPE_TEXT_ID => {
                    text_id = Some(block.data);
                }
                block_id::SHAPE_VALIGN => {
                    valign = Some(VerticalAlign::from(block.data));
                }
                block_id::TABLE_CELLS_SEQNUM if is_table => {
                    cells_seq_num = Some(block.data);
                }
                block_id::TABLE_NUM_ROWS if is_table => {
                    num_rows = Some(block.data);
                }
                block_id::TABLE_NUM_COLS if is_table => {
                    num_cols = Some(block.data);
                }
                _ => {
                    self.skip_block(&block)?;
                }
            }
        }

        // Build table info if this is a table
        if is_table {
            if let (Some(csn), Some(r), Some(c)) = (cells_seq_num, num_rows, num_cols) {
                table_info = Some(TableInfo {
                    rows: r,
                    cols: c,
                    cells_seq_num: csn,
                });
            }
        }

        let width = emus_to_points(width_emu as i64);
        let height = emus_to_points(height_emu as i64);

        log::trace!(
            "Shape seq={}: {}x{}, text_id={:?}, is_table={}",
            chunk.seq_num,
            width,
            height,
            text_id,
            is_table
        );

        Ok(Some(PubShape {
            seq_num: chunk.seq_num,
            width,
            height,
            text_id,
            valign,
            is_table,
            table_info,
        }))
    }

    /// Parse a block header and determine its data.
    ///
    /// Block format (from libmspub):
    /// - 1 byte: id
    /// - 1 byte: type
    /// - data (length determined by type)
    fn parse_block(&mut self) -> FormatResult<BlockInfo> {
        let _start_pos = self.reader.stream_position()?;

        let id = self.read_u8()? as u32;
        let block_type = self.read_u8()?;
        let data_offset = self.reader.stream_position()?;

        let data_length = get_block_data_length(block_type);

        let (actual_length, data, string_data) = if data_length < 0 {
            // Variable length: first DWORD is length (includes the 4 bytes for the length field itself)
            let len = self.read_u32()? as u64;
            // For string containers, read the string data (length - 4 bytes for the length field)
            let str_data = if is_block_data_string(block_type) && len > 4 {
                let mut buf = vec![0u8; (len - 4) as usize];
                self.reader.read_exact(&mut buf)?;
                buf
            } else {
                vec![]
            };
            (len, 0u32, str_data)
        } else {
            let len = data_length as u64;
            let d = match len {
                0 => 0,
                2 => self.read_u16()? as u32,
                4 => self.read_u32()?,
                8 => {
                    // For 8-byte data, just read first 4 bytes as the value
                    let v = self.read_u32()?;
                    self.reader.seek(SeekFrom::Current(4))?;
                    v
                }
                16 | 24 => {
                    // For larger fixed sizes, skip for now
                    self.reader.seek(SeekFrom::Current(len as i64))?;
                    0
                }
                _ => 0,
            };
            (len, d, vec![])
        };

        Ok(BlockInfo {
            id,
            block_type,
            data_offset,
            data_length: actual_length,
            data,
            string_data,
        })
    }

    /// Skip past a block's data.
    fn skip_block(&mut self, block: &BlockInfo) -> FormatResult<()> {
        let end = block.data_offset + block.data_length;
        let current = self.reader.stream_position()?;
        if end > current {
            self.reader.seek(SeekFrom::Start(end))?;
        }
        Ok(())
    }

    // Reading helpers
    fn read_u8(&mut self) -> FormatResult<u8> {
        let mut buf = [0u8; 1];
        self.reader.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16(&mut self) -> FormatResult<u16> {
        let mut buf = [0u8; 2];
        self.reader.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_u32(&mut self) -> FormatResult<u32> {
        let mut buf = [0u8; 4];
        self.reader.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }
}

/// Get the data length for a block type. Returns -1 for variable length.
fn get_block_data_length(block_type: u8) -> i16 {
    match block_type {
        // 0-byte data
        0x05 | 0x08 | 0x0A | 0x78 => 0,

        // 2-byte data
        0x07 | 0x10 | 0x12 | 0x18 | 0x1A => 2,

        // 4-byte data
        0x20 | 0x22 | 0x58 | 0x68 | 0x70 | 0xB8 => 4,

        // 8-byte data
        0x28 => 8,

        // 16-byte data
        0x38 => 16,

        // 24-byte data
        0x48 => 24,

        // Variable length (first DWORD is length)
        0x80 | 0x82 | 0x88 | 0x8A | 0x90 | 0x98 | 0xA0 | 0xC0 => -1,

        // Unknown type - treat as 0 length
        _ => {
            log::warn!("Unknown block type: 0x{:02X}", block_type);
            0
        }
    }
}

/// Check if a block type contains string data.
fn is_block_data_string(block_type: u8) -> bool {
    block_type == block_type::STRING_CONTAINER
}

/// Decode UTF-16LE bytes to a String.
fn decode_utf16le(data: &[u8]) -> String {
    let u16_iter = data
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]));

    char::decode_utf16(u16_iter)
        .filter_map(|r| r.ok())
        .filter(|c| *c != '\0')
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_data_lengths() {
        assert_eq!(get_block_data_length(0x05), 0);
        assert_eq!(get_block_data_length(0x10), 2);
        assert_eq!(get_block_data_length(0x20), 4);
        assert_eq!(get_block_data_length(0x28), 8);
        assert_eq!(get_block_data_length(0x38), 16);
        assert_eq!(get_block_data_length(0x48), 24);
        assert_eq!(get_block_data_length(0x88), -1);
        assert_eq!(get_block_data_length(0x90), -1);
        assert_eq!(get_block_data_length(0xC0), -1);
    }

    #[test]
    fn test_vertical_align() {
        assert_eq!(VerticalAlign::from(0), VerticalAlign::Top);
        assert_eq!(VerticalAlign::from(1), VerticalAlign::Middle);
        assert_eq!(VerticalAlign::from(2), VerticalAlign::Bottom);
        assert_eq!(VerticalAlign::from(99), VerticalAlign::Top);
    }

    #[test]
    fn test_decode_utf16le() {
        // "Hello" in UTF-16LE
        let data = [0x48, 0x00, 0x65, 0x00, 0x6C, 0x00, 0x6C, 0x00, 0x6F, 0x00];
        assert_eq!(decode_utf16le(&data), "Hello");

        // With null terminator
        let data_with_null = [0x48, 0x00, 0x69, 0x00, 0x00, 0x00];
        assert_eq!(decode_utf16le(&data_with_null), "Hi");
    }

    /// Test parsing the ramble-strip.pub file's Contents stream.
    /// The Contents stream is 10306 bytes.
    #[test]
    #[ignore] // Requires the test file at a specific path
    fn test_parse_ramble_strip_contents() {
        use std::fs::File;
        use std::io::Read;

        let test_file = "/Users/lukem/Downloads/ramble-strip.pub";
        if !std::path::Path::new(test_file).exists() {
            eprintln!("Test file not found: {}", test_file);
            return;
        }

        // Open as OLE compound file
        let file = File::open(test_file).expect("Failed to open test file");
        let mut cfb = cfb::CompoundFile::open(file).expect("Failed to open as OLE");

        // Read Contents stream
        let mut contents_data = Vec::new();
        {
            let mut stream = cfb.open_stream("/Contents").expect("No Contents stream");
            stream
                .read_to_end(&mut contents_data)
                .expect("Failed to read Contents");
        }

        println!("Contents stream size: {} bytes", contents_data.len());
        assert_eq!(
            contents_data.len(),
            10306,
            "Expected Contents stream to be 10306 bytes"
        );

        // Parse it
        let cursor = std::io::Cursor::new(contents_data);
        let mut parser = ContentsParser::new(cursor).expect("Failed to create parser");
        let doc = parser.parse().expect("Failed to parse Contents");

        println!("Document size: {}x{} points", doc.width, doc.height);
        println!("Number of pages: {}", doc.pages.len());
        println!("Number of shapes: {}", doc.shapes.len());

        for (i, page) in doc.pages.iter().enumerate() {
            println!(
                "  Page {}: seq={}, is_master={}, shapes={:?}",
                i, page.seq_num, page.is_master, page.shape_refs
            );
        }

        println!("\nShapes:");
        for (seq, shape) in &doc.shapes {
            println!(
                "  Shape {}: {}x{} pts, text_id={:?}, is_table={}",
                seq, shape.width, shape.height, shape.text_id, shape.is_table
            );
        }

        // Basic assertions
        assert!(doc.width > 0.0, "Document width should be positive");
        assert!(doc.height > 0.0, "Document height should be positive");
        assert!(!doc.pages.is_empty(), "Should have at least one page");
    }
}
