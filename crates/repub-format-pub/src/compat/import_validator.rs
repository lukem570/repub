//! Import validation - measures how completely we parse .pub files.

use std::path::Path;
use std::collections::HashMap;

use crate::error::FormatResult;
use crate::model::LayerObjectExt;
use crate::pub_import;
use super::block_coverage::BlockCoverage;

/// Result of import validation
#[derive(Debug, Clone)]
pub struct ImportValidationResult {
    /// File path that was validated
    pub file_path: String,
    /// Whether import succeeded
    pub success: bool,
    /// Error message if import failed
    pub error: Option<String>,
    
    /// Number of pages found
    pub pages: usize,
    /// Number of objects found (all types)
    pub objects: usize,
    /// Objects by type
    pub objects_by_type: HashMap<String, usize>,
    
    /// Text frames found
    pub text_frames: usize,
    /// Total text characters extracted
    pub text_chars: usize,
    /// Text frames with content
    pub text_frames_with_content: usize,
    
    /// Images found
    pub images: usize,
    /// Embedded images (vs linked)
    pub embedded_images: usize,
    
    /// Tables found
    pub tables: usize,
    /// Total table cells
    pub table_cells: usize,
    
    /// Shapes found
    pub shapes: usize,
    /// Groups found
    pub groups: usize,
    
    /// Block coverage statistics
    pub block_coverage: BlockCoverage,
    
    /// Streams found in OLE file
    pub streams: Vec<String>,
    /// Stream sizes
    pub stream_sizes: HashMap<String, usize>,
    
    /// Validation score (0-100%)
    pub score: f64,
    /// Detailed scoring breakdown
    pub score_breakdown: HashMap<String, f64>,
}

impl ImportValidationResult {
    fn new(file_path: &str) -> Self {
        Self {
            file_path: file_path.to_string(),
            success: false,
            error: None,
            pages: 0,
            objects: 0,
            objects_by_type: HashMap::new(),
            text_frames: 0,
            text_chars: 0,
            text_frames_with_content: 0,
            images: 0,
            embedded_images: 0,
            tables: 0,
            table_cells: 0,
            shapes: 0,
            groups: 0,
            block_coverage: BlockCoverage::new(),
            streams: Vec::new(),
            stream_sizes: HashMap::new(),
            score: 0.0,
            score_breakdown: HashMap::new(),
        }
    }

    /// Generate a text summary
    pub fn summary(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("Import Validation: {}\n", self.file_path));
        s.push_str(&format!("Status: {}\n", if self.success { "SUCCESS" } else { "FAILED" }));
        if let Some(err) = &self.error {
            s.push_str(&format!("Error: {}\n", err));
        }
        s.push_str(&format!("\nScore: {:.1}%\n", self.score));
        s.push_str("\nContent Summary:\n");
        s.push_str(&format!("  Pages: {}\n", self.pages));
        s.push_str(&format!("  Objects: {}\n", self.objects));
        s.push_str(&format!("  Text frames: {} ({} with content, {} chars)\n", 
            self.text_frames, self.text_frames_with_content, self.text_chars));
        s.push_str(&format!("  Images: {} ({} embedded)\n", self.images, self.embedded_images));
        s.push_str(&format!("  Tables: {} ({} cells)\n", self.tables, self.table_cells));
        s.push_str(&format!("  Shapes: {}\n", self.shapes));
        s.push_str(&format!("  Groups: {}\n", self.groups));
        
        if !self.score_breakdown.is_empty() {
            s.push_str("\nScore Breakdown:\n");
            for (category, score) in &self.score_breakdown {
                s.push_str(&format!("  {}: {:.1}%\n", category, score));
            }
        }
        
        s
    }
}

/// Validates .pub file imports
pub struct ImportValidator {
    /// Enable verbose logging
    pub verbose: bool,
}

impl ImportValidator {
    pub fn new() -> Self {
        Self { verbose: false }
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Validate a single .pub file
    pub fn validate<P: AsRef<Path>>(&self, path: P) -> ImportValidationResult {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_string();
        let mut result = ImportValidationResult::new(&path_str);

        // First, analyze the OLE structure
        if let Ok(file) = std::fs::File::open(path) {
            if let Ok(mut cfb) = cfb::CompoundFile::open(file) {
                let paths: Vec<String> = cfb.walk()
                    .filter(|e| e.is_stream())
                    .map(|e| e.path().to_string_lossy().to_string())
                    .collect();
                
                for stream_path in &paths {
                    if let Ok(stream) = cfb.open_stream(stream_path) {
                        result.stream_sizes.insert(stream_path.clone(), stream.len() as usize);
                    }
                }
                result.streams = paths;
            }
        }

        // Now try to import
        match pub_import::import(path) {
            Ok(doc) => {
                result.success = true;
                result.pages = doc.pages().len();
                
                // Count objects
                for page in doc.pages() {
                    for layer in page.layers() {
                        self.count_objects(&layer.objects(), &mut result);
                    }
                }
                
                // Calculate score
                self.calculate_score(&mut result);
            }
            Err(e) => {
                result.success = false;
                result.error = Some(format!("{}", e));
            }
        }

        result
    }

    fn count_objects(&self, objects: &[crate::model::Object], result: &mut ImportValidationResult) {
        use crate::model::Object;
        
        for obj in objects {
            result.objects += 1;
            
            match obj {
                Object::TextFrame(tf) => {
                    result.text_frames += 1;
                    *result.objects_by_type.entry("TextFrame".to_string()).or_insert(0) += 1;
                    
                    let text = tf.content.plain_text();
                    result.text_chars += text.len();
                    if !text.is_empty() {
                        result.text_frames_with_content += 1;
                    }
                }
                Object::ImageFrame(img) => {
                    result.images += 1;
                    *result.objects_by_type.entry("ImageFrame".to_string()).or_insert(0) += 1;
                    
                    if matches!(img.source, crate::model::ImageSource::Embedded { .. }) {
                        result.embedded_images += 1;
                    }
                }
                Object::Table(table) => {
                    result.tables += 1;
                    result.table_cells += table.cells.len();
                    *result.objects_by_type.entry("Table".to_string()).or_insert(0) += 1;
                }
                Object::Shape(_) => {
                    result.shapes += 1;
                    *result.objects_by_type.entry("Shape".to_string()).or_insert(0) += 1;
                }
                Object::Line(_) => {
                    *result.objects_by_type.entry("Line".to_string()).or_insert(0) += 1;
                }
                Object::Group(group) => {
                    result.groups += 1;
                    *result.objects_by_type.entry("Group".to_string()).or_insert(0) += 1;
                    // Recurse into group
                    self.count_objects(&group.children, result);
                }
                Object::WordArt(_) => {
                    *result.objects_by_type.entry("WordArt".to_string()).or_insert(0) += 1;
                }
                Object::TextOnPath(_) => {
                    *result.objects_by_type.entry("TextOnPath".to_string()).or_insert(0) += 1;
                }
            }
        }
    }

    fn calculate_score(&self, result: &mut ImportValidationResult) {
        let mut scores = Vec::new();
        
        // Basic import success (40%)
        if result.success {
            result.score_breakdown.insert("Import Success".to_string(), 100.0);
            scores.push((40.0, 100.0));
        } else {
            result.score_breakdown.insert("Import Success".to_string(), 0.0);
            scores.push((40.0, 0.0));
        }
        
        // Pages found (10%)
        let page_score = if result.pages > 0 { 100.0 } else { 0.0 };
        result.score_breakdown.insert("Pages".to_string(), page_score);
        scores.push((10.0, page_score));
        
        // Objects found (15%)
        let object_score = if result.objects > 0 { 100.0 } else if result.pages > 0 { 50.0 } else { 0.0 };
        result.score_breakdown.insert("Objects".to_string(), object_score);
        scores.push((15.0, object_score));
        
        // Text content (15%)
        let text_score = if result.text_frames_with_content > 0 {
            100.0
        } else if result.text_frames > 0 {
            50.0  // Frames found but no content
        } else {
            0.0
        };
        result.score_breakdown.insert("Text Content".to_string(), text_score);
        scores.push((15.0, text_score));
        
        // Images (10%)
        let image_score = if result.embedded_images > 0 {
            100.0
        } else if result.images > 0 {
            50.0
        } else if result.stream_sizes.contains_key("/Escher/EscherDelayStm") {
            25.0  // Stream exists but no images extracted
        } else {
            100.0  // No images in file, that's fine
        };
        result.score_breakdown.insert("Images".to_string(), image_score);
        scores.push((10.0, image_score));
        
        // Stream presence (10%)
        let expected_streams = ["/Contents", "/Quill/QuillSub/CONTENTS", "/Escher/EscherStm"];
        let found = expected_streams.iter()
            .filter(|s| result.streams.iter().any(|x| x == *s))
            .count();
        let stream_score = found as f64 / expected_streams.len() as f64 * 100.0;
        result.score_breakdown.insert("Streams".to_string(), stream_score);
        scores.push((10.0, stream_score));
        
        // Calculate weighted average
        let total_weight: f64 = scores.iter().map(|(w, _)| w).sum();
        let weighted_sum: f64 = scores.iter().map(|(w, s)| w * s).sum();
        result.score = weighted_sum / total_weight;
    }

    /// Validate multiple files and return aggregate results
    pub fn validate_all<P: AsRef<Path>>(&self, paths: &[P]) -> Vec<ImportValidationResult> {
        paths.iter().map(|p| self.validate(p)).collect()
    }
}

impl Default for ImportValidator {
    fn default() -> Self {
        Self::new()
    }
}
