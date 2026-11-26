//! Quill text storage format parser for Microsoft Publisher files.
//!
//! The Quill format stores text content, formatting, fonts, and colors in Publisher
//! documents. It is found in the `Quill/QuillSub/CONTENTS` stream of .pub files.
//!
//! ## Format Overview
//!
//! The Quill stream starts with a header followed by chunk references:
//!
//! ```text
//! Offset 0x00: Magic "CHNKINK " (8 bytes)
//! Offset 0x08: Version/flags (8 bytes)
//! Offset 0x10: Unknown (8 bytes)
//! Offset 0x18: Chunk list metadata
//!   - Offset 0x1A: numChunks (u16)
//!   - Offset 0x1C: nextListOffset (u32, 0xFFFFFFFF if no more)
//! Offset 0x20: First chunk reference (24 bytes each)
//! ```
//!
//! ## Chunk Reference Structure (24 bytes)
//!
//! ```text
//! 0x00: marker (u16) - should be 0x0018
//! 0x02: name (4 bytes ASCII) - chunk type like "TEXT", "FDPC", etc.
//! 0x06: id (u16) - chunk instance id
//! 0x08: flags (u32) - usually 0x01000000
//! 0x0C: name2 (4 bytes ASCII) - secondary type name
//! 0x10: offset (u32) - offset to chunk data
//! 0x14: length (u32) - length of chunk data
//! ```
//!
//! ## Important Chunks
//!
//! - `TEXT`: Raw UTF-16LE text content
//! - `STRS`: Text string lengths array (maps text to shapes)
//! - `SYID`: Text string IDs (shape references)
//! - `FDPC`: Character formatting (bold, italic, font, size, color)
//! - `FDPP`: Paragraph formatting (alignment, spacing, indents)
//! - `STSH`: Default styles (id=1 is the main one)
//! - `FONT`: Font table
//! - `PL  `: Color palette

use std::collections::HashMap;

use crate::error::{FormatError, FormatResult};
use crate::pub_import::{emus_to_points, EMUS_PER_POINT};

/// Chunk reference marker value.
const CHUNK_REF_MARKER: u16 = 0x0018;

/// Chunk reference size in bytes.
const CHUNK_REF_SIZE: usize = 24;

/// Header offset where chunk list metadata begins.
const CHUNK_LIST_HEADER_OFFSET: usize = 0x18;

/// Offset where first chunk reference begins.
const FIRST_CHUNK_REF_OFFSET: usize = 0x20;

/// End-of-list marker for nextListOffset.
const END_OF_LIST: u32 = 0xFFFF_FFFF;

/// Text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    #[default]
    Left,
    Center,
    Right,
    Justify,
}

impl Alignment {
    fn from_value(value: u32) -> Self {
        match value & 0xFF {
            0 => Alignment::Left,
            1 => Alignment::Right,
            2 => Alignment::Center,
            6 | 3 => Alignment::Justify,
            _ => Alignment::Left,
        }
    }
}

/// A span of text with character formatting.
#[derive(Debug, Clone)]
pub struct QuillSpan {
    /// The text content of this span.
    pub text: String,
    /// Whether the text is bold.
    pub bold: bool,
    /// Whether the text is italic.
    pub italic: bool,
    /// Whether the text is underlined.
    pub underline: bool,
    /// Font index into the font table.
    pub font_index: Option<u32>,
    /// Font size in points.
    pub font_size: Option<f64>,
    /// Color index into the color palette.
    pub color_index: Option<u32>,
}

impl Default for QuillSpan {
    fn default() -> Self {
        Self {
            text: String::new(),
            bold: false,
            italic: false,
            underline: false,
            font_index: None,
            font_size: None,
            color_index: None,
        }
    }
}

/// A paragraph with formatting and text spans.
#[derive(Debug, Clone)]
pub struct QuillParagraph {
    /// Text spans within this paragraph.
    pub spans: Vec<QuillSpan>,
    /// Paragraph alignment.
    pub alignment: Alignment,
    /// Line spacing multiplier (1.0 = single, 2.0 = double).
    pub line_spacing: f64,
    /// Space before paragraph in points.
    pub space_before: f64,
    /// Space after paragraph in points.
    pub space_after: f64,
    /// First line indent in points (can be negative for hanging indent).
    pub first_indent: f64,
    /// Left indent in points.
    pub left_indent: f64,
    /// Right indent in points.
    pub right_indent: f64,
}

impl Default for QuillParagraph {
    fn default() -> Self {
        Self {
            spans: Vec::new(),
            alignment: Alignment::Left,
            line_spacing: 1.0,
            space_before: 0.0,
            space_after: 0.0,
            first_indent: 0.0,
            left_indent: 0.0,
            right_indent: 0.0,
        }
    }
}

/// A text string with its ID and content.
#[derive(Debug, Clone)]
pub struct QuillText {
    /// The text ID (used to link text to shapes).
    pub id: u32,
    /// Paragraphs in this text string.
    pub paragraphs: Vec<QuillParagraph>,
}

/// A font entry from the font table.
#[derive(Debug, Clone)]
pub struct QuillFont {
    /// Font family name.
    pub name: String,
}

/// A color entry from the color palette.
#[derive(Debug, Clone, Copy)]
pub struct QuillColor {
    /// Red component (0-255).
    pub r: u8,
    /// Green component (0-255).
    pub g: u8,
    /// Blue component (0-255).
    pub b: u8,
}

impl QuillColor {
    /// Create a color from an RGB value.
    pub fn from_rgb(value: u32) -> Self {
        Self {
            r: (value & 0xFF) as u8,
            g: ((value >> 8) & 0xFF) as u8,
            b: ((value >> 16) & 0xFF) as u8,
        }
    }
}

/// Reference to a chunk in the Quill stream.
#[derive(Debug, Clone)]
struct ChunkReference {
    /// Chunk type name (e.g., "TEXT", "FDPC").
    name: String,
    /// Instance ID.
    id: u16,
    /// Secondary name.
    name2: String,
    /// Offset to chunk data.
    offset: u32,
    /// Length of chunk data.
    length: u32,
}

/// Internal character style parsed from FDPC.
#[derive(Debug, Clone, Default)]
struct CharacterStyle {
    bold: bool,
    italic: bool,
    underline: bool,
    font_index: Option<u32>,
    text_size_emu: Option<u32>,
    color_index: Option<u32>,
}

/// Internal paragraph style parsed from FDPP.
#[derive(Debug, Clone, Default)]
struct ParagraphStyle {
    alignment: Alignment,
    line_spacing_raw: Option<u32>,
    space_before_emu: Option<u32>,
    space_after_emu: Option<u32>,
    first_indent_emu: Option<i32>,
    left_indent_emu: Option<u32>,
    right_indent_emu: Option<u32>,
}

/// Reference associating a text offset with a style.
#[derive(Debug, Clone)]
struct StyleReference<T> {
    /// Text offset where this style ends.
    end_offset: u32,
    /// The style.
    style: T,
}

/// Parser for Quill text storage format.
pub struct QuillParser {
    /// Raw data of the Quill stream.
    data: Vec<u8>,
    /// Parsed chunk references.
    chunks: Vec<ChunkReference>,
    /// Parsed fonts.
    fonts: Vec<QuillFont>,
    /// Parsed colors.
    colors: Vec<QuillColor>,
    /// Text strings by ID.
    texts: HashMap<u32, QuillText>,
}

impl QuillParser {
    /// Create a new parser for the given Quill stream data.
    pub fn new(data: Vec<u8>) -> FormatResult<Self> {
        let mut parser = Self {
            data,
            chunks: Vec::new(),
            fonts: Vec::new(),
            colors: Vec::new(),
            texts: HashMap::new(),
        };

        parser.parse_chunk_references()?;
        parser.parse_fonts()?;
        parser.parse_colors()?;
        parser.parse_text()?;

        Ok(parser)
    }

    /// Get the parsed fonts.
    pub fn fonts(&self) -> &[QuillFont] {
        &self.fonts
    }

    /// Get the parsed colors.
    pub fn colors(&self) -> &[QuillColor] {
        &self.colors
    }

    /// Get all parsed text strings.
    pub fn texts(&self) -> &HashMap<u32, QuillText> {
        &self.texts
    }

    /// Get a specific text string by ID.
    pub fn text(&self, id: u32) -> Option<&QuillText> {
        self.texts.get(&id)
    }

    /// Parse chunk references from the header.
    fn parse_chunk_references(&mut self) -> FormatResult<()> {
        if self.data.len() < FIRST_CHUNK_REF_OFFSET {
            return Err(FormatError::InvalidStructure(
                "Quill stream too short for header".to_string(),
            ));
        }

        // Verify magic
        let magic = &self.data[0..8];
        if magic != b"CHNKINK " {
            return Err(FormatError::InvalidStructure(format!(
                "Invalid Quill magic: {:?}",
                magic
            )));
        }

        // Parse chunk list(s)
        let mut list_offset = CHUNK_LIST_HEADER_OFFSET;
        let mut seen_offsets = std::collections::HashSet::new();

        while list_offset != END_OF_LIST as usize {
            if seen_offsets.contains(&list_offset) {
                log::warn!("Cycle detected in Quill chunk list");
                break;
            }
            seen_offsets.insert(list_offset);

            if list_offset + 8 > self.data.len() {
                break;
            }

            // Read list header
            let num_chunks = read_u16(&self.data, list_offset + 2);
            let next_list = read_u32(&self.data, list_offset + 4);

            // Parse chunk references
            let mut offset = list_offset + 8; // After the 2+2+4 header bytes
            for _ in 0..num_chunks {
                if offset + CHUNK_REF_SIZE > self.data.len() {
                    break;
                }

                let marker = read_u16(&self.data, offset);
                if marker != CHUNK_REF_MARKER {
                    log::warn!(
                        "Unexpected chunk reference marker 0x{:04x} at offset 0x{:04x}",
                        marker,
                        offset
                    );
                    break;
                }

                let name = read_ascii4(&self.data, offset + 2);
                let id = read_u16(&self.data, offset + 6);
                let name2 = read_ascii4(&self.data, offset + 12);
                let chunk_offset = read_u32(&self.data, offset + 16);
                let chunk_length = read_u32(&self.data, offset + 20);

                self.chunks.push(ChunkReference {
                    name,
                    id,
                    name2,
                    offset: chunk_offset,
                    length: chunk_length,
                });

                offset += CHUNK_REF_SIZE;
            }

            list_offset = next_list as usize;
        }

        log::debug!("Parsed {} Quill chunks", self.chunks.len());
        Ok(())
    }

    /// Find a chunk by name and optional ID.
    fn find_chunk(&self, name: &str, id: Option<u16>) -> Option<&ChunkReference> {
        self.chunks
            .iter()
            .find(|c| c.name == name && (id.is_none() || Some(c.id) == id))
    }

    /// Get chunk data by reference.
    fn chunk_data(&self, chunk: &ChunkReference) -> &[u8] {
        let start = chunk.offset as usize;
        let end = start + chunk.length as usize;
        if end <= self.data.len() {
            &self.data[start..end]
        } else {
            &[]
        }
    }

    /// Parse the FONT chunk.
    fn parse_fonts(&mut self) -> FormatResult<()> {
        let chunk = match self.find_chunk("FONT", None) {
            Some(c) => c.clone(),
            None => {
                log::debug!("No FONT chunk found");
                return Ok(());
            }
        };

        let data = self.chunk_data(&chunk).to_vec();
        if data.len() < 20 {
            return Ok(());
        }

        // Structure: unknown(4), numElements(4), skip(12), offsets[], names[]
        let num_elements = read_u32(&data, 4) as usize;

        // Parse font names sequentially after offset table
        let names_start = 20 + num_elements * 4;
        let mut pos = names_start;

        for _ in 0..num_elements {
            if pos + 2 > data.len() {
                break;
            }

            let name_len = read_u16(&data, pos) as usize;
            pos += 2;

            let name = if name_len > 0 && pos + name_len * 2 <= data.len() {
                let name_bytes = &data[pos..pos + name_len * 2];
                pos += name_len * 2;
                decode_utf16le(name_bytes)
            } else {
                String::new()
            };

            // Skip unknown u32 after name
            pos += 4;

            self.fonts.push(QuillFont { name });
        }

        log::debug!("Parsed {} fonts", self.fonts.len());
        Ok(())
    }

    /// Parse the PL (color palette) chunk.
    fn parse_colors(&mut self) -> FormatResult<()> {
        let chunk = match self.find_chunk("PL  ", None) {
            Some(c) => c.clone(),
            None => {
                log::debug!("No PL chunk found");
                return Ok(());
            }
        };

        let data = self.chunk_data(&chunk).to_vec();
        if data.len() < 12 {
            return Ok(());
        }

        let num_entries = read_u32(&data, 0) as usize;

        // Parse color entries
        let mut pos = 12; // Skip header
        for _ in 0..num_entries {
            if pos + 4 > data.len() {
                break;
            }

            let block_len = read_u32(&data, pos) as usize;
            let block_end = pos + 4 + block_len;
            pos += 4;

            // Look for color value in block (id=0x01 is typically the main color)
            while pos + 6 <= block_end && pos + 6 <= data.len() {
                let block_id = data[pos];
                let block_type = data[pos + 1];

                if block_type == 0x20 || block_type == 0x22 {
                    let value = read_u32(&data, pos + 2);
                    if block_id == 0x01 {
                        self.colors.push(QuillColor::from_rgb(value));
                    }
                    pos += 6;
                } else if block_type == 0x08 || block_type == 0x0a {
                    pos += 2;
                } else {
                    break;
                }
            }

            pos = block_end;
        }

        log::debug!("Parsed {} colors", self.colors.len());
        Ok(())
    }

    /// Parse text content from TEXT, STRS, SYID, FDPC, FDPP chunks.
    fn parse_text(&mut self) -> FormatResult<()> {
        // Get TEXT chunk
        let text_chunk = match self.find_chunk("TEXT", None) {
            Some(c) => c.clone(),
            None => {
                log::debug!("No TEXT chunk found");
                return Ok(());
            }
        };

        let text_data = self.chunk_data(&text_chunk);
        let full_text = decode_utf16le(text_data);

        // Get STRS chunk (string lengths)
        let strs_chunk = self.find_chunk("STRS", None).cloned();

        // Get SYID chunk (text IDs)
        let syid_chunk = self.find_chunk("SYID", None).cloned();

        // Get FDPC chunk (character formatting)
        let fdpc_chunk = self.find_chunk("FDPC", None).cloned();

        // Get FDPP chunk (paragraph formatting)
        let fdpp_chunk = self.find_chunk("FDPP", None).cloned();

        // Parse string lengths and IDs
        let (text_lengths, text_ids) = self.parse_strs_syid(&strs_chunk, &syid_chunk)?;

        // Parse character styles
        let char_styles = self.parse_fdpc(&fdpc_chunk, &text_chunk)?;

        // Parse paragraph styles
        let para_styles = self.parse_fdpp(&fdpp_chunk, &text_chunk)?;

        // Build text strings with formatting
        let mut char_offset: usize = 0;
        for (i, (&length, &id)) in text_lengths.iter().zip(text_ids.iter()).enumerate() {
            let text_end = char_offset + length as usize;
            let text_slice: String = full_text
                .chars()
                .skip(char_offset)
                .take(length as usize)
                .collect();

            // Find styles that apply to this text string
            let byte_start = char_offset * 2; // UTF-16 offset
            let byte_end = text_end * 2;

            let relevant_char_styles: Vec<_> = char_styles
                .iter()
                .filter(|s| {
                    let style_end = s.end_offset as usize;
                    style_end > byte_start && style_end <= byte_end + 2
                })
                .collect();

            let relevant_para_styles: Vec<_> = para_styles
                .iter()
                .filter(|s| {
                    let style_end = s.end_offset as usize;
                    style_end > byte_start && style_end <= byte_end + 2
                })
                .collect();

            let paragraphs = self.build_paragraphs(
                &text_slice,
                &relevant_char_styles,
                &relevant_para_styles,
                byte_start,
            );

            self.texts.insert(id, QuillText { id, paragraphs });

            char_offset = text_end;
            log::debug!(
                "Text string {}: id=0x{:08x}, {} paragraphs, {} chars",
                i,
                id,
                self.texts[&id].paragraphs.len(),
                length
            );
        }

        Ok(())
    }

    /// Parse STRS and SYID chunks for text lengths and IDs.
    fn parse_strs_syid(
        &self,
        strs_chunk: &Option<ChunkReference>,
        syid_chunk: &Option<ChunkReference>,
    ) -> FormatResult<(Vec<u32>, Vec<u32>)> {
        let mut text_lengths = Vec::new();
        let mut text_ids = Vec::new();

        // Parse STRS
        if let Some(chunk) = strs_chunk {
            let data = self.chunk_data(chunk);
            if data.len() >= 8 {
                let num_entries = read_u32(data, 0) as usize;
                let data_offset = read_u32(data, 4) as usize;

                // Length values start at offset data_offset + 4
                let len_start = data_offset + 4;
                for i in 0..=num_entries {
                    let pos = len_start + i * 4;
                    if pos + 4 <= data.len() {
                        text_lengths.push(read_u32(data, pos));
                    }
                }
            }
        }

        // Parse SYID
        if let Some(chunk) = syid_chunk {
            let data = self.chunk_data(chunk);
            if data.len() >= 8 {
                let num_ids = read_u32(data, 4) as usize;
                for i in 0..num_ids {
                    let pos = 8 + i * 4;
                    if pos + 4 <= data.len() {
                        text_ids.push(read_u32(data, pos));
                    }
                }
            }
        }

        // If we have more lengths than IDs, generate placeholder IDs
        while text_ids.len() < text_lengths.len() {
            text_ids.push(text_ids.len() as u32);
        }

        // If we have more IDs than lengths, truncate
        text_ids.truncate(text_lengths.len());

        Ok((text_lengths, text_ids))
    }

    /// Parse FDPC chunk for character styles.
    fn parse_fdpc(
        &self,
        chunk: &Option<ChunkReference>,
        text_chunk: &ChunkReference,
    ) -> FormatResult<Vec<StyleReference<CharacterStyle>>> {
        let mut styles = Vec::new();

        let chunk = match chunk {
            Some(c) => c,
            None => return Ok(styles),
        };

        let data = self.chunk_data(chunk);
        if data.len() < 8 {
            return Ok(styles);
        }

        // Structure: numEntries(u16), padding(6), textOffsets[], chunkOffsets[]
        let num_entries = read_u16(data, 0) as usize;

        // Text offsets (u32 each)
        let mut text_offsets = Vec::with_capacity(num_entries);
        for i in 0..num_entries {
            let pos = 8 + i * 4;
            if pos + 4 <= data.len() {
                text_offsets.push(read_u32(data, pos));
            }
        }

        // Chunk offsets (u16 each) - these are offsets within the FDPC chunk
        let chunk_offset_start = 8 + num_entries * 4;
        let mut chunk_offsets = Vec::with_capacity(num_entries);
        for i in 0..num_entries {
            let pos = chunk_offset_start + i * 2;
            if pos + 2 <= data.len() {
                chunk_offsets.push(read_u16(data, pos));
            }
        }

        // Parse each style block
        for i in 0..num_entries.min(text_offsets.len()).min(chunk_offsets.len()) {
            let style_offset = chunk_offsets[i] as usize;
            if style_offset + 4 > data.len() {
                continue;
            }

            let style = self.parse_character_style_block(&data[style_offset..]);

            // Convert text offset to be relative to TEXT chunk start
            styles.push(StyleReference {
                end_offset: text_offsets[i] + text_chunk.offset,
                style,
            });
        }

        Ok(styles)
    }

    /// Parse a character style block.
    fn parse_character_style_block(&self, data: &[u8]) -> CharacterStyle {
        let mut style = CharacterStyle::default();

        if data.len() < 4 {
            return style;
        }

        let block_len = read_u32(data, 0) as usize;
        let mut pos = 4;

        while pos + 2 <= data.len() && pos < 4 + block_len {
            let block_id = data[pos];
            let block_type = data[pos + 1];

            match block_type {
                0x08 | 0x0a => {
                    // Flag type - just the presence indicates true
                    match block_id {
                        0x01 | 0x02 => style.bold = true,   // Bold flags
                        0x03 | 0x04 => style.italic = true, // Italic flags
                        0x05 => style.underline = true,     // Underline
                        _ => {}
                    }
                    pos += 2;
                }
                0x10 | 0x12 | 0x18 | 0x1a => {
                    // 2-byte value
                    if pos + 4 <= data.len() {
                        let _value = read_u16(data, pos + 2);
                        // Handle 2-byte properties if needed
                        pos += 4;
                    } else {
                        break;
                    }
                }
                0x20 | 0x22 => {
                    // 4-byte value
                    if pos + 6 <= data.len() {
                        let value = read_u32(data, pos + 2);
                        match block_id {
                            0x06 | 0x0C | 0x39 => {
                                // Text size in EMUs (0x06, 0x0C, 0x39 are variants)
                                style.text_size_emu = Some(value);
                            }
                            0x0D | 0x24 => {
                                // Font index (0x0D direct, 0x24 in container)
                                style.font_index = Some(value);
                            }
                            0x11 | 0x2E | 0x44 => {
                                // Color index
                                style.color_index = Some(value);
                            }
                            _ => {}
                        }
                        pos += 6;
                    } else {
                        break;
                    }
                }
                0x80 | 0x82 | 0x88 | 0x8a | 0x90 | 0x98 | 0xa0 => {
                    // Variable-length container - skip by reading length
                    if pos + 6 <= data.len() {
                        let container_len = read_u32(data, pos + 2) as usize;
                        pos += 6 + container_len;
                    } else {
                        break;
                    }
                }
                _ => {
                    // Unknown type, try to skip
                    pos += 2;
                }
            }
        }

        style
    }

    /// Parse FDPP chunk for paragraph styles.
    fn parse_fdpp(
        &self,
        chunk: &Option<ChunkReference>,
        text_chunk: &ChunkReference,
    ) -> FormatResult<Vec<StyleReference<ParagraphStyle>>> {
        let mut styles = Vec::new();

        let chunk = match chunk {
            Some(c) => c,
            None => return Ok(styles),
        };

        let data = self.chunk_data(chunk);
        if data.len() < 8 {
            return Ok(styles);
        }

        // Same structure as FDPC
        let num_entries = read_u16(data, 0) as usize;

        let mut text_offsets = Vec::with_capacity(num_entries);
        for i in 0..num_entries {
            let pos = 8 + i * 4;
            if pos + 4 <= data.len() {
                text_offsets.push(read_u32(data, pos));
            }
        }

        let chunk_offset_start = 8 + num_entries * 4;
        let mut chunk_offsets = Vec::with_capacity(num_entries);
        for i in 0..num_entries {
            let pos = chunk_offset_start + i * 2;
            if pos + 2 <= data.len() {
                chunk_offsets.push(read_u16(data, pos));
            }
        }

        for i in 0..num_entries.min(text_offsets.len()).min(chunk_offsets.len()) {
            let style_offset = chunk_offsets[i] as usize;
            if style_offset + 4 > data.len() {
                continue;
            }

            let style = self.parse_paragraph_style_block(&data[style_offset..]);

            styles.push(StyleReference {
                end_offset: text_offsets[i] + text_chunk.offset,
                style,
            });
        }

        Ok(styles)
    }

    /// Parse a paragraph style block.
    fn parse_paragraph_style_block(&self, data: &[u8]) -> ParagraphStyle {
        let mut style = ParagraphStyle::default();

        if data.len() < 4 {
            return style;
        }

        let block_len = read_u32(data, 0) as usize;
        let mut pos = 4;

        while pos + 2 <= data.len() && pos < 4 + block_len {
            let block_id = data[pos];
            let block_type = data[pos + 1];

            match block_type {
                0x08 | 0x0a => {
                    // Flag type
                    pos += 2;
                }
                0x10 | 0x12 | 0x18 | 0x1a => {
                    // 2-byte value
                    if pos + 4 <= data.len() {
                        let _value = read_u16(data, pos + 2);
                        pos += 4;
                    } else {
                        break;
                    }
                }
                0x20 | 0x22 => {
                    // 4-byte value
                    if pos + 6 <= data.len() {
                        let value = read_u32(data, pos + 2);
                        match block_id {
                            0x01 | 0x04 => {
                                // Alignment (0x01 in FDPP, 0x04 is alternate ID)
                                style.alignment = Alignment::from_value(value);
                            }
                            0x34 => {
                                // Line spacing
                                style.line_spacing_raw = Some(value);
                            }
                            0x05 | 0x12 => {
                                // Space before
                                style.space_before_emu = Some(value);
                            }
                            0x06 | 0x13 => {
                                // Space after
                                style.space_after_emu = Some(value);
                            }
                            0x07 | 0x0C => {
                                // First line indent (signed)
                                style.first_indent_emu = Some(value as i32);
                            }
                            0x08 | 0x0D => {
                                // Left indent
                                style.left_indent_emu = Some(value);
                            }
                            0x09 | 0x0E => {
                                // Right indent
                                style.right_indent_emu = Some(value);
                            }
                            _ => {}
                        }
                        pos += 6;
                    } else {
                        break;
                    }
                }
                0x80 | 0x82 | 0x88 | 0x8a | 0x90 | 0x98 | 0xa0 => {
                    // Variable-length container
                    if pos + 6 <= data.len() {
                        let container_len = read_u32(data, pos + 2) as usize;
                        pos += 6 + container_len;
                    } else {
                        break;
                    }
                }
                _ => {
                    pos += 2;
                }
            }
        }

        style
    }

    /// Build paragraphs from text and styles.
    fn build_paragraphs(
        &self,
        text: &str,
        char_styles: &[&StyleReference<CharacterStyle>],
        para_styles: &[&StyleReference<ParagraphStyle>],
        base_offset: usize,
    ) -> Vec<QuillParagraph> {
        let mut paragraphs = Vec::new();

        // Split text by paragraph separators (CR = 0x0D in UTF-16)
        let para_texts: Vec<&str> = text.split('\r').collect();

        let mut char_pos: usize = 0;

        for para_text in para_texts {
            if para_text.is_empty() && paragraphs.is_empty() {
                // Skip leading empty paragraph
                char_pos += 1; // Account for CR
                continue;
            }

            let para_byte_start = base_offset + char_pos * 2;
            let para_byte_end = base_offset + (char_pos + para_text.len()) * 2;

            // Find paragraph style
            let para_style = para_styles
                .iter()
                .find(|s| {
                    let end = s.end_offset as usize;
                    end >= para_byte_start && end <= para_byte_end + 2
                })
                .map(|s| &s.style);

            // Build spans for this paragraph
            let mut spans = Vec::new();
            let mut span_start = 0;

            // Find character styles that apply to this paragraph
            for char_style_ref in char_styles.iter() {
                let style_byte_end = char_style_ref.end_offset as usize;

                // Check if this style ends within our paragraph
                if style_byte_end > para_byte_start && style_byte_end <= para_byte_end + 2 {
                    let style_char_end = (style_byte_end - para_byte_start) / 2;
                    let span_end = style_char_end.min(para_text.len());

                    if span_end > span_start {
                        let span_text: String = para_text
                            .chars()
                            .skip(span_start)
                            .take(span_end - span_start)
                            .collect();

                        if !span_text.is_empty() {
                            spans.push(self.build_span(&span_text, &char_style_ref.style));
                        }
                        span_start = span_end;
                    }
                }
            }

            // Add remaining text as default span
            if span_start < para_text.len() {
                let remaining: String = para_text.chars().skip(span_start).collect();
                if !remaining.is_empty() {
                    spans.push(QuillSpan {
                        text: remaining,
                        ..Default::default()
                    });
                }
            }

            // If no spans were created, create one with all the text
            if spans.is_empty() && !para_text.is_empty() {
                spans.push(QuillSpan {
                    text: para_text.to_string(),
                    ..Default::default()
                });
            }

            let paragraph = self.build_paragraph(spans, para_style);
            paragraphs.push(paragraph);

            char_pos += para_text.len() + 1; // +1 for CR
        }

        // Ensure at least one paragraph
        if paragraphs.is_empty() {
            paragraphs.push(QuillParagraph::default());
        }

        paragraphs
    }

    /// Build a QuillSpan from text and character style.
    fn build_span(&self, text: &str, style: &CharacterStyle) -> QuillSpan {
        QuillSpan {
            text: text.to_string(),
            bold: style.bold,
            italic: style.italic,
            underline: style.underline,
            font_index: style.font_index,
            font_size: style.text_size_emu.map(|emu| emus_to_points(emu as i64)),
            color_index: style.color_index,
        }
    }

    /// Build a QuillParagraph from spans and paragraph style.
    fn build_paragraph(
        &self,
        spans: Vec<QuillSpan>,
        style: Option<&ParagraphStyle>,
    ) -> QuillParagraph {
        let style = style.cloned().unwrap_or_default();

        // Convert line spacing from raw format
        let line_spacing = if let Some(raw) = style.line_spacing_raw {
            if raw & 1 != 0 {
                // Points expressed as eighths of an EMU (yes, really)
                (raw - 1) as f64 / 8.0 * 72.0 / crate::pub_import::EMUS_PER_INCH
            } else if raw & 2 != 0 {
                // SP (line spacing multiplier) expressed as EMUs for 96pt font
                (raw - 2) as f64 / crate::pub_import::EMUS_PER_INCH * 72.0 / 96.0
            } else {
                1.0
            }
        } else {
            1.0
        };

        QuillParagraph {
            spans,
            alignment: style.alignment,
            line_spacing,
            space_before: style
                .space_before_emu
                .map(|v| v as f64 / EMUS_PER_POINT)
                .unwrap_or(0.0),
            space_after: style
                .space_after_emu
                .map(|v| v as f64 / EMUS_PER_POINT)
                .unwrap_or(0.0),
            first_indent: style
                .first_indent_emu
                .map(|v| v as f64 / EMUS_PER_POINT)
                .unwrap_or(0.0),
            left_indent: style
                .left_indent_emu
                .map(|v| v as f64 / EMUS_PER_POINT)
                .unwrap_or(0.0),
            right_indent: style
                .right_indent_emu
                .map(|v| v as f64 / EMUS_PER_POINT)
                .unwrap_or(0.0),
        }
    }
}

// Helper functions for reading binary data

fn read_u16(data: &[u8], offset: usize) -> u16 {
    if offset + 2 <= data.len() {
        u16::from_le_bytes([data[offset], data[offset + 1]])
    } else {
        0
    }
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    if offset + 4 <= data.len() {
        u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ])
    } else {
        0
    }
}

fn read_ascii4(data: &[u8], offset: usize) -> String {
    if offset + 4 <= data.len() {
        String::from_utf8_lossy(&data[offset..offset + 4])
            .trim_end_matches('\0')
            .to_string()
    } else {
        String::new()
    }
}

fn decode_utf16le(data: &[u8]) -> String {
    let u16s: Vec<u16> = data
        .chunks_exact(2)
        .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
        .collect();
    String::from_utf16_lossy(&u16s)
        .trim_end_matches('\0')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_alignment_from_value() {
        assert_eq!(Alignment::from_value(0), Alignment::Left);
        assert_eq!(Alignment::from_value(1), Alignment::Right);
        assert_eq!(Alignment::from_value(2), Alignment::Center);
        assert_eq!(Alignment::from_value(6), Alignment::Justify);
        assert_eq!(Alignment::from_value(3), Alignment::Justify);
        assert_eq!(Alignment::from_value(99), Alignment::Left); // Unknown defaults to left
    }

    #[test]
    fn test_read_helpers() {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        assert_eq!(read_u16(&data, 0), 0x0201);
        assert_eq!(read_u32(&data, 0), 0x04030201);
        assert_eq!(read_ascii4(&data, 0), "\x01\x02\x03\x04");
    }

    #[test]
    fn test_decode_utf16le() {
        // "Hello" in UTF-16LE
        let data = [0x48, 0x00, 0x65, 0x00, 0x6C, 0x00, 0x6C, 0x00, 0x6F, 0x00];
        assert_eq!(decode_utf16le(&data), "Hello");
    }

    #[test]
    fn test_quill_color_from_rgb() {
        let color = QuillColor::from_rgb(0x00FF8040);
        assert_eq!(color.r, 0x40);
        assert_eq!(color.g, 0x80);
        assert_eq!(color.b, 0xFF);
    }

    #[test]
    fn test_quill_span_default() {
        let span = QuillSpan::default();
        assert!(span.text.is_empty());
        assert!(!span.bold);
        assert!(!span.italic);
        assert!(!span.underline);
        assert!(span.font_index.is_none());
        assert!(span.font_size.is_none());
        assert!(span.color_index.is_none());
    }

    #[test]
    fn test_quill_paragraph_default() {
        let para = QuillParagraph::default();
        assert!(para.spans.is_empty());
        assert_eq!(para.alignment, Alignment::Left);
        assert!((para.line_spacing - 1.0).abs() < 0.001);
        assert!((para.space_before - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_invalid_magic() {
        let data = vec![0x00; 32];
        let result = QuillParser::new(data);
        assert!(result.is_err());
    }

    #[test]
    fn test_too_short() {
        let data = vec![0x00; 10];
        let result = QuillParser::new(data);
        assert!(result.is_err());
    }
}
