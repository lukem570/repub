//! Block type coverage tracking.
//!
//! Tracks which block types (id, type pairs) are encountered during parsing
//! and whether they are correctly handled.

use std::collections::HashMap;

/// Statistics for a single block type
#[derive(Debug, Clone, Default)]
pub struct BlockStats {
    /// Number of times this block was encountered
    pub count: usize,
    /// Number of times it was successfully parsed
    pub parsed: usize,
    /// Number of times it was skipped/ignored
    pub skipped: usize,
    /// Number of times parsing failed
    pub failed: usize,
    /// Sample values seen (for debugging)
    pub sample_values: Vec<u32>,
}

impl BlockStats {
    pub fn success_rate(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.parsed as f64 / self.count as f64 * 100.0
        }
    }
}

/// Tracks coverage of all block types
#[derive(Debug, Clone, Default)]
pub struct BlockCoverage {
    /// Stats by (id, type) pair
    blocks: HashMap<(u8, u8), BlockStats>,
    /// Stats by chunk type
    chunks: HashMap<u8, BlockStats>,
    /// Total blocks seen
    pub total_blocks: usize,
    /// Total blocks successfully parsed
    pub total_parsed: usize,
}

impl BlockCoverage {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a block encounter
    pub fn record_block(&mut self, id: u8, block_type: u8, parsed: bool, value: Option<u32>) {
        self.total_blocks += 1;
        if parsed {
            self.total_parsed += 1;
        }

        let stats = self.blocks.entry((id, block_type)).or_default();
        stats.count += 1;
        if parsed {
            stats.parsed += 1;
        } else {
            stats.skipped += 1;
        }
        if let Some(v) = value {
            if stats.sample_values.len() < 5 {
                stats.sample_values.push(v);
            }
        }
    }

    /// Record a block parsing failure
    pub fn record_failure(&mut self, id: u8, block_type: u8) {
        self.total_blocks += 1;
        let stats = self.blocks.entry((id, block_type)).or_default();
        stats.count += 1;
        stats.failed += 1;
    }

    /// Record a chunk type encounter
    pub fn record_chunk(&mut self, chunk_type: u8, parsed: bool) {
        let stats = self.chunks.entry(chunk_type).or_default();
        stats.count += 1;
        if parsed {
            stats.parsed += 1;
        } else {
            stats.skipped += 1;
        }
    }

    /// Get overall coverage percentage
    pub fn overall_coverage(&self) -> f64 {
        if self.total_blocks == 0 {
            0.0
        } else {
            self.total_parsed as f64 / self.total_blocks as f64 * 100.0
        }
    }

    /// Get stats for a specific block type
    pub fn get_block_stats(&self, id: u8, block_type: u8) -> Option<&BlockStats> {
        self.blocks.get(&(id, block_type))
    }

    /// Get all block stats
    pub fn all_blocks(&self) -> &HashMap<(u8, u8), BlockStats> {
        &self.blocks
    }

    /// Get all chunk stats
    pub fn all_chunks(&self) -> &HashMap<u8, BlockStats> {
        &self.chunks
    }

    /// Get blocks sorted by frequency (most common first)
    pub fn blocks_by_frequency(&self) -> Vec<((u8, u8), &BlockStats)> {
        let mut blocks: Vec<_> = self.blocks.iter().map(|(&k, v)| (k, v)).collect();
        blocks.sort_by_key(|b| std::cmp::Reverse(b.1.count));
        blocks
    }

    /// Get unhandled blocks (skipped or failed)
    pub fn unhandled_blocks(&self) -> Vec<((u8, u8), &BlockStats)> {
        self.blocks
            .iter()
            .filter(|(_, stats)| stats.skipped > 0 || stats.failed > 0)
            .map(|(&k, v)| (k, v))
            .collect()
    }

    /// Merge another coverage report into this one
    pub fn merge(&mut self, other: &BlockCoverage) {
        self.total_blocks += other.total_blocks;
        self.total_parsed += other.total_parsed;

        for (&key, stats) in &other.blocks {
            let entry = self.blocks.entry(key).or_default();
            entry.count += stats.count;
            entry.parsed += stats.parsed;
            entry.skipped += stats.skipped;
            entry.failed += stats.failed;
        }

        for (&key, stats) in &other.chunks {
            let entry = self.chunks.entry(key).or_default();
            entry.count += stats.count;
            entry.parsed += stats.parsed;
            entry.skipped += stats.skipped;
        }
    }

    /// Generate a summary string
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!(
            "Block Coverage: {:.1}% ({}/{} blocks)\n",
            self.overall_coverage(),
            self.total_parsed,
            self.total_blocks
        ));
        s.push_str(&format!("Unique block types: {}\n", self.blocks.len()));
        s.push_str(&format!("Chunk types: {}\n", self.chunks.len()));

        let unhandled = self.unhandled_blocks();
        if !unhandled.is_empty() {
            s.push_str(&format!("Unhandled block types: {}\n", unhandled.len()));
        }

        s
    }
}

/// Known block type names for better reporting
pub fn block_type_name(block_type: u8) -> &'static str {
    match block_type {
        0x05 | 0x08 | 0x0A | 0x78 => "0-byte",
        0x07 | 0x10 | 0x12 | 0x18 | 0x1A => "2-byte",
        0x20 | 0x22 | 0x58 | 0x68 | 0x70 | 0xB8 => "4-byte",
        0x28 => "8-byte",
        0x38 => "16-byte",
        0x48 => "24-byte",
        0x80 | 0x82 | 0x88 | 0x8A | 0x90 | 0x98 | 0xA0 | 0xC0 => "variable",
        _ => "unknown",
    }
}

/// Known block ID names
pub fn block_id_name(id: u8) -> &'static str {
    match id {
        0x01 => "WIDTH/OFFSET",
        0x02 => "HEIGHT/TYPE/SHAPES",
        0x04 => "CHUNK_OFFSET",
        0x05 => "PARENT_SEQNUM",
        0x09 => "BORDER_IMAGE_ID",
        0x0a => "PAGE_BG_SHAPE",
        0x0c => "TEXT_SIZE/FIRST_INDENT",
        0x0d => "LEFT_INDENT",
        0x0e => "RIGHT_INDENT",
        0x12 => "DOCUMENT_SIZE/SPACE_BEFORE",
        0x13 => "SPACE_AFTER",
        0x19 => "DEFAULT_CHAR_STYLE",
        0x24 => "FONT_INDEX_CONTAINER",
        0x27 => "TEXT_ID",
        0x2e => "BARE_COLOR_INDEX",
        0x34 => "LINE_SPACING",
        0x35 => "VALIGN",
        0x37 => "BOLD_2",
        0x38 => "ITALIC_2",
        0x39 => "TEXT_SIZE_2",
        0x44 => "COLOR_INDEX_CONTAINER",
        0x57 => "LIST_INFO",
        0x66 => "NUM_ROWS",
        0x67 => "NUM_COLS",
        0x6b => "CELLS_SEQNUM",
        0x6d => "ROWCOL_ARRAY",
        0xaa => "SHAPE_WIDTH",
        0xab => "SHAPE_HEIGHT",
        0xb7 => "CROP",
        _ => "unknown",
    }
}

/// Known chunk type names
pub fn chunk_type_name(chunk_type: u8) -> &'static str {
    match chunk_type {
        0x01 => "SHAPE",
        0x10 => "TABLE",
        0x20 => "ALTSHAPE",
        0x30 => "GROUP",
        0x31 => "LOGO",
        0x43 => "PAGE",
        0x44 => "DOCUMENT",
        0x46 => "BORDER_ART",
        0x5C => "PALETTE",
        0x63 => "CELLS",
        0x6C => "FONT",
        _ => "unknown",
    }
}
