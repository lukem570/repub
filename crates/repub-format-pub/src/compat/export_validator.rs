//! Export validation - measures how well exported files match the expected format.

use std::collections::HashMap;
use std::io::{Read, Seek};
use std::path::Path;

use crate::error::FormatResult;
use crate::model::LayerObjectExt;
use crate::pub_export::{self, PubExportOptions};
use repub_model::document::Document;

/// Result of export validation
#[derive(Debug, Clone)]
pub struct ExportValidationResult {
    /// Whether export succeeded
    pub success: bool,
    /// Error message if export failed
    pub error: Option<String>,

    /// File size in bytes
    pub file_size: usize,

    /// Streams created
    pub streams: Vec<String>,
    /// Stream sizes
    pub stream_sizes: HashMap<String, usize>,

    /// Whether the file is a valid OLE compound document
    pub valid_ole: bool,

    /// Whether Contents stream exists and has valid header
    pub valid_contents_header: bool,
    /// Contents stream size
    pub contents_size: usize,
    /// Trailer offset found in Contents
    pub trailer_offset: Option<u32>,
    /// Number of chunk entries in trailer
    pub chunk_count: Option<usize>,

    /// Whether Quill stream exists
    pub has_quill: bool,
    /// Quill stream size
    pub quill_size: usize,

    /// Whether Escher streams exist
    pub has_escher: bool,
    /// EscherStm size
    pub escher_stm_size: usize,
    /// EscherDelayStm size
    pub escher_delay_size: usize,

    /// Whether the exported file can be re-imported
    pub reimportable: bool,
    /// Re-import error if any
    pub reimport_error: Option<String>,
    /// Number of pages after re-import
    pub reimport_pages: usize,
    /// Number of objects after re-import
    pub reimport_objects: usize,

    /// Validation score (0-100%)
    pub score: f64,
    /// Score breakdown
    pub score_breakdown: HashMap<String, f64>,
}

impl ExportValidationResult {
    fn new() -> Self {
        Self {
            success: false,
            error: None,
            file_size: 0,
            streams: Vec::new(),
            stream_sizes: HashMap::new(),
            valid_ole: false,
            valid_contents_header: false,
            contents_size: 0,
            trailer_offset: None,
            chunk_count: None,
            has_quill: false,
            quill_size: 0,
            has_escher: false,
            escher_stm_size: 0,
            escher_delay_size: 0,
            reimportable: false,
            reimport_error: None,
            reimport_pages: 0,
            reimport_objects: 0,
            score: 0.0,
            score_breakdown: HashMap::new(),
        }
    }

    /// Generate a text summary
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&"Export Validation\n".to_string());
        s.push_str(&format!(
            "Status: {}\n",
            if self.success { "SUCCESS" } else { "FAILED" }
        ));
        if let Some(err) = &self.error {
            s.push_str(&format!("Error: {}\n", err));
        }
        s.push_str(&format!("\nScore: {:.1}%\n", self.score));
        s.push_str(&format!("\nFile size: {} bytes\n", self.file_size));
        s.push_str(&format!("Valid OLE: {}\n", self.valid_ole));
        s.push_str(&format!("Streams: {}\n", self.streams.len()));

        if self.valid_contents_header {
            s.push_str(&format!(
                "\nContents stream: {} bytes\n",
                self.contents_size
            ));
            if let Some(offset) = self.trailer_offset {
                s.push_str(&format!("  Trailer offset: 0x{:x}\n", offset));
            }
            if let Some(count) = self.chunk_count {
                s.push_str(&format!("  Chunk count: {}\n", count));
            }
        }

        s.push_str(&format!(
            "\nQuill: {} ({} bytes)\n",
            if self.has_quill { "yes" } else { "no" },
            self.quill_size
        ));
        s.push_str(&format!(
            "Escher: {} (stm: {} bytes, delay: {} bytes)\n",
            if self.has_escher { "yes" } else { "no" },
            self.escher_stm_size,
            self.escher_delay_size
        ));

        s.push_str(&format!("\nRe-importable: {}\n", self.reimportable));
        if let Some(err) = &self.reimport_error {
            s.push_str(&format!("  Re-import error: {}\n", err));
        }
        if self.reimportable {
            s.push_str(&format!("  Pages: {}\n", self.reimport_pages));
            s.push_str(&format!("  Objects: {}\n", self.reimport_objects));
        }

        if !self.score_breakdown.is_empty() {
            s.push_str("\nScore Breakdown:\n");
            for (category, score) in &self.score_breakdown {
                s.push_str(&format!("  {}: {:.1}%\n", category, score));
            }
        }

        s
    }
}

/// Validates .pub file exports
pub struct ExportValidator {
    /// Export options to use
    pub options: PubExportOptions,
    /// Enable verbose logging
    pub verbose: bool,
}

impl ExportValidator {
    pub fn new() -> Self {
        Self {
            options: PubExportOptions::default(),
            verbose: false,
        }
    }

    pub fn with_options(mut self, options: PubExportOptions) -> Self {
        self.options = options;
        self
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Validate export of a document
    pub fn validate(&self, document: &Document) -> ExportValidationResult {
        let mut result = ExportValidationResult::new();

        // Export to memory buffer
        let mut buffer = std::io::Cursor::new(Vec::new());
        match pub_export::export_to_writer(document, &mut buffer, &self.options) {
            Ok(()) => {
                result.success = true;
            }
            Err(e) => {
                result.success = false;
                result.error = Some(format!("{}", e));
                return result;
            }
        }

        let data = buffer.into_inner();
        result.file_size = data.len();

        // Validate OLE structure
        self.validate_ole_structure(&data, &mut result);

        // Try to re-import
        self.validate_reimport(&data, document, &mut result);

        // Calculate score
        self.calculate_score(&mut result, document);

        result
    }

    /// Validate export to a file path
    pub fn validate_to_file<P: AsRef<Path>>(
        &self,
        document: &Document,
        path: P,
    ) -> ExportValidationResult {
        let mut result = ExportValidationResult::new();

        // Export to file
        match pub_export::export(document, path.as_ref(), &self.options) {
            Ok(()) => {
                result.success = true;
            }
            Err(e) => {
                result.success = false;
                result.error = Some(format!("{}", e));
                return result;
            }
        }

        // Read the file back
        let data = match std::fs::read(path.as_ref()) {
            Ok(d) => d,
            Err(e) => {
                result.error = Some(format!("Failed to read exported file: {}", e));
                return result;
            }
        };

        result.file_size = data.len();

        // Validate OLE structure
        self.validate_ole_structure(&data, &mut result);

        // Try to re-import from file
        match crate::pub_import::import(path.as_ref()) {
            Ok(doc) => {
                result.reimportable = true;
                result.reimport_pages = doc.pages().len();
                for page in doc.pages() {
                    for layer in page.layers() {
                        result.reimport_objects += layer.objects().len();
                    }
                }
            }
            Err(e) => {
                result.reimportable = false;
                result.reimport_error = Some(format!("{}", e));
            }
        }

        // Calculate score
        self.calculate_score(&mut result, document);

        result
    }

    fn validate_ole_structure(&self, data: &[u8], result: &mut ExportValidationResult) {
        let cursor = std::io::Cursor::new(data);

        match cfb::CompoundFile::open(cursor) {
            Ok(mut cfb) => {
                result.valid_ole = true;

                // Collect streams
                let paths: Vec<String> = cfb
                    .walk()
                    .filter(|e| e.is_stream())
                    .map(|e| e.path().to_string_lossy().to_string())
                    .collect();

                for path in &paths {
                    if let Ok(stream) = cfb.open_stream(path) {
                        result
                            .stream_sizes
                            .insert(path.clone(), stream.len() as usize);
                    }
                }
                result.streams = paths;

                // Check Contents stream
                if let Ok(mut contents) = cfb.open_stream("/Contents") {
                    result.contents_size = contents.len() as usize;

                    if contents.len() >= 0x1E {
                        let mut header = vec![0u8; 0x1E];
                        if contents.read_exact(&mut header).is_ok() {
                            // Trailer offset at 0x1A
                            let trailer_offset = u32::from_le_bytes([
                                header[0x1A],
                                header[0x1B],
                                header[0x1C],
                                header[0x1D],
                            ]);

                            if trailer_offset > 0
                                && (trailer_offset as usize) < result.contents_size
                            {
                                result.valid_contents_header = true;
                                result.trailer_offset = Some(trailer_offset);
                            }
                        }
                    }
                }

                // Check Quill stream
                if let Ok(quill) = cfb.open_stream("/Quill/QuillSub/CONTENTS") {
                    result.has_quill = true;
                    result.quill_size = quill.len() as usize;
                }

                // Check Escher streams
                if let Ok(escher) = cfb.open_stream("/Escher/EscherStm") {
                    result.has_escher = true;
                    result.escher_stm_size = escher.len() as usize;
                }
                if let Ok(delay) = cfb.open_stream("/Escher/EscherDelayStm") {
                    result.escher_delay_size = delay.len() as usize;
                }
            }
            Err(_) => {
                result.valid_ole = false;
            }
        }
    }

    fn validate_reimport(
        &self,
        data: &[u8],
        _original: &Document,
        result: &mut ExportValidationResult,
    ) {
        // Write to temp file for import
        let temp_path = std::env::temp_dir().join("repub_export_validation.pub");

        if std::fs::write(&temp_path, data).is_err() {
            result.reimport_error = Some("Failed to write temp file".to_string());
            return;
        }

        match crate::pub_import::import(&temp_path) {
            Ok(doc) => {
                result.reimportable = true;
                result.reimport_pages = doc.pages().len();
                for page in doc.pages() {
                    for layer in page.layers() {
                        result.reimport_objects += layer.objects().len();
                    }
                }
            }
            Err(e) => {
                result.reimportable = false;
                result.reimport_error = Some(format!("{}", e));
            }
        }

        let _ = std::fs::remove_file(&temp_path);
    }

    fn calculate_score(&self, result: &mut ExportValidationResult, original: &Document) {
        let mut scores = Vec::new();

        // Export success (20%)
        let export_score = if result.success { 100.0 } else { 0.0 };
        result
            .score_breakdown
            .insert("Export Success".to_string(), export_score);
        scores.push((20.0, export_score));

        // Valid OLE (15%)
        let ole_score = if result.valid_ole { 100.0 } else { 0.0 };
        result
            .score_breakdown
            .insert("Valid OLE".to_string(), ole_score);
        scores.push((15.0, ole_score));

        // Required streams (15%)
        let stream_count = ["/Contents", "/Quill/QuillSub/CONTENTS", "/Escher/EscherStm"]
            .iter()
            .filter(|s| result.streams.iter().any(|x| x == *s))
            .count();
        let stream_score = stream_count as f64 / 3.0 * 100.0;
        result
            .score_breakdown
            .insert("Required Streams".to_string(), stream_score);
        scores.push((15.0, stream_score));

        // Contents structure (15%)
        let contents_score = if result.valid_contents_header && result.trailer_offset.is_some() {
            100.0
        } else if result.contents_size > 0 {
            50.0
        } else {
            0.0
        };
        result
            .score_breakdown
            .insert("Contents Structure".to_string(), contents_score);
        scores.push((15.0, contents_score));

        // Re-importable (25%)
        let reimport_score = if result.reimportable {
            // Check if page/object count matches
            let original_pages = original.pages().len();
            let original_objects: usize = original
                .pages()
                .iter()
                .flat_map(|p| p.layers().iter())
                .map(|l| l.objects().len())
                .sum();

            let page_match = if original_pages == result.reimport_pages {
                50.0
            } else {
                0.0
            };
            let obj_match = if original_objects > 0 && result.reimport_objects > 0 {
                (result.reimport_objects.min(original_objects) as f64
                    / result.reimport_objects.max(original_objects) as f64)
                    * 50.0
            } else if original_objects == 0 && result.reimport_objects == 0 {
                50.0
            } else {
                0.0
            };

            page_match + obj_match
        } else {
            0.0
        };
        result
            .score_breakdown
            .insert("Re-importable".to_string(), reimport_score);
        scores.push((25.0, reimport_score));

        // Quill content (5%)
        let quill_score = if result.has_quill && result.quill_size > 24 {
            100.0
        } else {
            0.0
        };
        result
            .score_breakdown
            .insert("Quill Content".to_string(), quill_score);
        scores.push((5.0, quill_score));

        // Escher content (5%)
        let escher_score = if result.has_escher { 100.0 } else { 0.0 };
        result
            .score_breakdown
            .insert("Escher Content".to_string(), escher_score);
        scores.push((5.0, escher_score));

        // Calculate weighted average
        let total_weight: f64 = scores.iter().map(|(w, _)| w).sum();
        let weighted_sum: f64 = scores.iter().map(|(w, s)| w * s).sum();
        result.score = weighted_sum / total_weight;
    }
}

impl Default for ExportValidator {
    fn default() -> Self {
        Self::new()
    }
}
