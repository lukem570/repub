//! Round-trip testing: Import → Export → Re-import comparison.

use std::collections::HashMap;
use std::path::Path;

use repub_model::document::Document;

use crate::error::FormatResult;
use crate::model::{LayerObjectExt, Object};
use crate::pub_export::{self, PubExportOptions};
use crate::pub_import;

/// Result of a round-trip test
#[derive(Debug, Clone)]
pub struct RoundtripResult {
    /// Source file path
    pub source_path: String,

    /// Import succeeded
    pub import_success: bool,
    /// Import error if any
    pub import_error: Option<String>,

    /// Export succeeded
    pub export_success: bool,
    /// Export error if any
    pub export_error: Option<String>,

    /// Re-import succeeded
    pub reimport_success: bool,
    /// Re-import error if any
    pub reimport_error: Option<String>,

    /// Original document stats
    pub original: DocumentStats,
    /// Re-imported document stats
    pub reimported: DocumentStats,

    /// Comparison results
    pub comparison: ComparisonResult,

    /// Overall round-trip score (0-100%)
    pub score: f64,
    /// Score breakdown
    pub score_breakdown: HashMap<String, f64>,
}

/// Statistics about a document
#[derive(Debug, Clone, Default)]
pub struct DocumentStats {
    pub pages: usize,
    pub total_objects: usize,
    pub text_frames: usize,
    pub text_chars: usize,
    pub images: usize,
    pub shapes: usize,
    pub tables: usize,
    pub table_cells: usize,
    pub groups: usize,
    pub lines: usize,
    pub wordart: usize,

    /// Page sizes (width, height) in points
    pub page_sizes: Vec<(f64, f64)>,

    /// Object bounds for comparison
    pub object_bounds: Vec<(f64, f64, f64, f64)>,
}

impl DocumentStats {
    pub fn from_document(doc: &Document) -> Self {
        let mut stats = Self::default();

        stats.pages = doc.pages().len();

        for page in doc.pages() {
            stats.page_sizes.push((page.size.width, page.size.height));

            for layer in page.layers() {
                Self::count_objects(&layer.objects(), &mut stats);
            }
        }

        stats
    }

    fn count_objects(objects: &[Object], stats: &mut DocumentStats) {
        for obj in objects {
            stats.total_objects += 1;

            // Record bounds
            let bounds = obj.bounds();
            stats.object_bounds.push((
                bounds.origin.x,
                bounds.origin.y,
                bounds.width(),
                bounds.height(),
            ));

            match obj {
                Object::TextFrame(tf) => {
                    stats.text_frames += 1;
                    stats.text_chars += tf.content.plain_text().len();
                }
                Object::ImageFrame(_) => {
                    stats.images += 1;
                }
                Object::Shape(_) => {
                    stats.shapes += 1;
                }
                Object::Table(table) => {
                    stats.tables += 1;
                    stats.table_cells += table.cells.len();
                }
                Object::Group(group) => {
                    stats.groups += 1;
                    Self::count_objects(&group.children, stats);
                }
                Object::Line(_) => {
                    stats.lines += 1;
                }
                Object::WordArt(_) => {
                    stats.wordart += 1;
                }
                Object::TextOnPath(_) => {
                    stats.wordart += 1;
                }
            }
        }
    }
}

/// Comparison between original and re-imported documents
#[derive(Debug, Clone, Default)]
pub struct ComparisonResult {
    /// Page count matches
    pub pages_match: bool,
    /// Page count difference
    pub page_diff: i32,

    /// Object count matches
    pub objects_match: bool,
    /// Object count difference
    pub object_diff: i32,

    /// Text character count matches (within tolerance)
    pub text_match: bool,
    /// Text char difference
    pub text_diff: i32,
    /// Text match percentage
    pub text_match_percent: f64,

    /// Image count matches
    pub images_match: bool,
    /// Image count difference
    pub image_diff: i32,

    /// Page sizes match (within tolerance)
    pub sizes_match: bool,
    /// Number of matching page sizes
    pub matching_sizes: usize,

    /// Overall match percentage
    pub overall_match: f64,
}

impl ComparisonResult {
    pub fn compare(original: &DocumentStats, reimported: &DocumentStats) -> Self {
        let mut result = Self::default();

        // Page count
        result.page_diff = reimported.pages as i32 - original.pages as i32;
        result.pages_match = result.page_diff == 0;

        // Object count
        result.object_diff = reimported.total_objects as i32 - original.total_objects as i32;
        result.objects_match = result.object_diff == 0;

        // Text
        result.text_diff = reimported.text_chars as i32 - original.text_chars as i32;
        if original.text_chars > 0 {
            let min = original.text_chars.min(reimported.text_chars);
            let max = original.text_chars.max(reimported.text_chars);
            result.text_match_percent = min as f64 / max as f64 * 100.0;
        } else if reimported.text_chars == 0 {
            result.text_match_percent = 100.0;
        } else {
            result.text_match_percent = 0.0;
        }
        result.text_match = result.text_match_percent >= 95.0;

        // Images
        result.image_diff = reimported.images as i32 - original.images as i32;
        result.images_match = result.image_diff == 0;

        // Page sizes
        result.matching_sizes = 0;
        for (i, (ow, oh)) in original.page_sizes.iter().enumerate() {
            if let Some((rw, rh)) = reimported.page_sizes.get(i) {
                if (ow - rw).abs() < 1.0 && (oh - rh).abs() < 1.0 {
                    result.matching_sizes += 1;
                }
            }
        }
        result.sizes_match = result.matching_sizes == original.page_sizes.len()
            && original.page_sizes.len() == reimported.page_sizes.len();

        // Overall match
        let mut match_scores = Vec::new();
        match_scores.push(if result.pages_match { 100.0 } else { 0.0 });
        match_scores.push(if result.objects_match {
            100.0
        } else {
            (1.0 - (result.object_diff.abs() as f64 / original.total_objects.max(1) as f64))
                .max(0.0)
                * 100.0
        });
        match_scores.push(result.text_match_percent);
        match_scores.push(if result.images_match { 100.0 } else { 0.0 });
        match_scores.push(if result.sizes_match { 100.0 } else { 0.0 });

        result.overall_match = match_scores.iter().sum::<f64>() / match_scores.len() as f64;

        result
    }
}

/// Performs round-trip testing
pub struct RoundtripTest {
    /// Export options
    pub export_options: PubExportOptions,
    /// Enable verbose output
    pub verbose: bool,
}

impl RoundtripTest {
    pub fn new() -> Self {
        Self {
            export_options: PubExportOptions::default(),
            verbose: false,
        }
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Run round-trip test on a .pub file
    pub fn test<P: AsRef<Path>>(&self, path: P) -> RoundtripResult {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_string();

        let mut result = RoundtripResult {
            source_path: path_str,
            import_success: false,
            import_error: None,
            export_success: false,
            export_error: None,
            reimport_success: false,
            reimport_error: None,
            original: DocumentStats::default(),
            reimported: DocumentStats::default(),
            comparison: ComparisonResult::default(),
            score: 0.0,
            score_breakdown: HashMap::new(),
        };

        // Step 1: Import
        let doc = match pub_import::import(path) {
            Ok(d) => {
                result.import_success = true;
                d
            }
            Err(e) => {
                result.import_error = Some(format!("{}", e));
                self.calculate_score(&mut result);
                return result;
            }
        };

        result.original = DocumentStats::from_document(&doc);

        // Step 2: Export to temp file
        let temp_path = std::env::temp_dir().join("repub_roundtrip_test.pub");

        match pub_export::export(&doc, &temp_path, &self.export_options) {
            Ok(()) => {
                result.export_success = true;
            }
            Err(e) => {
                result.export_error = Some(format!("{}", e));
                self.calculate_score(&mut result);
                return result;
            }
        }

        // Step 3: Re-import
        match pub_import::import(&temp_path) {
            Ok(reimported_doc) => {
                result.reimport_success = true;
                result.reimported = DocumentStats::from_document(&reimported_doc);
            }
            Err(e) => {
                result.reimport_error = Some(format!("{}", e));
            }
        }

        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);

        // Compare
        if result.reimport_success {
            result.comparison = ComparisonResult::compare(&result.original, &result.reimported);
        }

        // Calculate score
        self.calculate_score(&mut result);

        result
    }

    /// Run round-trip test on a Document directly
    pub fn test_document(&self, doc: &Document) -> RoundtripResult {
        let mut result = RoundtripResult {
            source_path: "<in-memory document>".to_string(),
            import_success: true, // We already have the document
            import_error: None,
            export_success: false,
            export_error: None,
            reimport_success: false,
            reimport_error: None,
            original: DocumentStats::from_document(doc),
            reimported: DocumentStats::default(),
            comparison: ComparisonResult::default(),
            score: 0.0,
            score_breakdown: HashMap::new(),
        };

        // Export to temp file
        let temp_path = std::env::temp_dir().join("repub_roundtrip_doc_test.pub");

        match pub_export::export(doc, &temp_path, &self.export_options) {
            Ok(()) => {
                result.export_success = true;
            }
            Err(e) => {
                result.export_error = Some(format!("{}", e));
                self.calculate_score(&mut result);
                return result;
            }
        }

        // Re-import
        match pub_import::import(&temp_path) {
            Ok(reimported_doc) => {
                result.reimport_success = true;
                result.reimported = DocumentStats::from_document(&reimported_doc);
            }
            Err(e) => {
                result.reimport_error = Some(format!("{}", e));
            }
        }

        let _ = std::fs::remove_file(&temp_path);

        // Compare
        if result.reimport_success {
            result.comparison = ComparisonResult::compare(&result.original, &result.reimported);
        }

        self.calculate_score(&mut result);

        result
    }

    fn calculate_score(&self, result: &mut RoundtripResult) {
        let mut scores = Vec::new();

        // Import success (20%)
        let import_score = if result.import_success { 100.0 } else { 0.0 };
        result
            .score_breakdown
            .insert("Import".to_string(), import_score);
        scores.push((20.0, import_score));

        // Export success (20%)
        let export_score = if result.export_success { 100.0 } else { 0.0 };
        result
            .score_breakdown
            .insert("Export".to_string(), export_score);
        scores.push((20.0, export_score));

        // Re-import success (20%)
        let reimport_score = if result.reimport_success { 100.0 } else { 0.0 };
        result
            .score_breakdown
            .insert("Re-import".to_string(), reimport_score);
        scores.push((20.0, reimport_score));

        // Data fidelity (40%)
        let fidelity_score = if result.reimport_success {
            result.comparison.overall_match
        } else {
            0.0
        };
        result
            .score_breakdown
            .insert("Data Fidelity".to_string(), fidelity_score);
        scores.push((40.0, fidelity_score));

        // Calculate weighted average
        let total_weight: f64 = scores.iter().map(|(w, _)| w).sum();
        let weighted_sum: f64 = scores.iter().map(|(w, s)| w * s).sum();
        result.score = weighted_sum / total_weight;
    }

    /// Test multiple files and return all results
    pub fn test_all<P: AsRef<Path>>(&self, paths: &[P]) -> Vec<RoundtripResult> {
        paths.iter().map(|p| self.test(p)).collect()
    }
}

impl Default for RoundtripTest {
    fn default() -> Self {
        Self::new()
    }
}

impl RoundtripResult {
    /// Generate a text summary
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("Round-trip Test: {}\n", self.source_path));
        s.push_str(&format!("Overall Score: {:.1}%\n", self.score));
        s.push_str("\nSteps:\n");
        s.push_str(&format!(
            "  Import:    {} {}\n",
            if self.import_success { "✓" } else { "✗" },
            self.import_error.as_deref().unwrap_or("")
        ));
        s.push_str(&format!(
            "  Export:    {} {}\n",
            if self.export_success { "✓" } else { "✗" },
            self.export_error.as_deref().unwrap_or("")
        ));
        s.push_str(&format!(
            "  Re-import: {} {}\n",
            if self.reimport_success { "✓" } else { "✗" },
            self.reimport_error.as_deref().unwrap_or("")
        ));

        if self.import_success && self.reimport_success {
            s.push_str("\nComparison:\n");
            s.push_str(&format!(
                "  Pages:   {} → {} ({})\n",
                self.original.pages,
                self.reimported.pages,
                if self.comparison.pages_match {
                    "match"
                } else {
                    "MISMATCH"
                }
            ));
            s.push_str(&format!(
                "  Objects: {} → {} ({})\n",
                self.original.total_objects,
                self.reimported.total_objects,
                if self.comparison.objects_match {
                    "match"
                } else {
                    "MISMATCH"
                }
            ));
            s.push_str(&format!(
                "  Text:    {} → {} chars ({:.0}% match)\n",
                self.original.text_chars,
                self.reimported.text_chars,
                self.comparison.text_match_percent
            ));
            s.push_str(&format!(
                "  Images:  {} → {} ({})\n",
                self.original.images,
                self.reimported.images,
                if self.comparison.images_match {
                    "match"
                } else {
                    "MISMATCH"
                }
            ));
            s.push_str(&format!(
                "\n  Overall Fidelity: {:.1}%\n",
                self.comparison.overall_match
            ));
        }

        s
    }
}
