//! Compatibility report generation.

use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::export_validator::{ExportValidationResult, ExportValidator};
use super::import_validator::{ImportValidationResult, ImportValidator};
use super::roundtrip::{RoundtripResult, RoundtripTest};

/// Overall compatibility report
#[derive(Debug, Clone)]
pub struct CompatibilityReport {
    /// Timestamp of report generation
    pub timestamp: u64,
    /// Report title/description
    pub title: String,

    /// Import validation results
    pub import_results: Vec<ImportValidationResult>,
    /// Export validation results
    pub export_results: Vec<ExportValidationResult>,
    /// Round-trip results
    pub roundtrip_results: Vec<RoundtripResult>,

    /// Aggregate import score
    pub import_score: f64,
    /// Aggregate export score
    pub export_score: f64,
    /// Aggregate round-trip score
    pub roundtrip_score: f64,

    /// Overall compatibility score
    pub overall_score: f64,

    /// Files tested
    pub files_tested: usize,
    /// Files that passed all tests
    pub files_passed: usize,

    /// Issues found
    pub issues: Vec<CompatibilityIssue>,
}

/// A specific compatibility issue found
#[derive(Debug, Clone)]
pub struct CompatibilityIssue {
    /// Severity: error, warning, info
    pub severity: IssueSeverity,
    /// Category of issue
    pub category: String,
    /// Description
    pub description: String,
    /// File path if applicable
    pub file: Option<String>,
    /// Suggested fix
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IssueSeverity {
    Error,
    Warning,
    Info,
}

impl CompatibilityReport {
    pub fn new(title: &str) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            timestamp,
            title: title.to_string(),
            import_results: Vec::new(),
            export_results: Vec::new(),
            roundtrip_results: Vec::new(),
            import_score: 0.0,
            export_score: 0.0,
            roundtrip_score: 0.0,
            overall_score: 0.0,
            files_tested: 0,
            files_passed: 0,
            issues: Vec::new(),
        }
    }

    /// Add an import result
    pub fn add_import(&mut self, result: ImportValidationResult) {
        self.import_results.push(result);
        self.recalculate_scores();
    }

    /// Add an export result
    pub fn add_export(&mut self, result: ExportValidationResult) {
        self.export_results.push(result);
        self.recalculate_scores();
    }

    /// Add a round-trip result
    pub fn add_roundtrip(&mut self, result: RoundtripResult) {
        self.roundtrip_results.push(result);
        self.recalculate_scores();
    }

    fn recalculate_scores(&mut self) {
        // Import score
        if !self.import_results.is_empty() {
            self.import_score = self.import_results.iter().map(|r| r.score).sum::<f64>()
                / self.import_results.len() as f64;
        }

        // Export score
        if !self.export_results.is_empty() {
            self.export_score = self.export_results.iter().map(|r| r.score).sum::<f64>()
                / self.export_results.len() as f64;
        }

        // Round-trip score
        if !self.roundtrip_results.is_empty() {
            self.roundtrip_score = self.roundtrip_results.iter().map(|r| r.score).sum::<f64>()
                / self.roundtrip_results.len() as f64;
        }

        // Overall score (weighted)
        let mut total_weight = 0.0;
        let mut weighted_sum = 0.0;

        if !self.import_results.is_empty() {
            total_weight += 30.0;
            weighted_sum += 30.0 * self.import_score;
        }
        if !self.export_results.is_empty() {
            total_weight += 30.0;
            weighted_sum += 30.0 * self.export_score;
        }
        if !self.roundtrip_results.is_empty() {
            total_weight += 40.0;
            weighted_sum += 40.0 * self.roundtrip_score;
        }

        self.overall_score = if total_weight > 0.0 {
            weighted_sum / total_weight
        } else {
            0.0
        };

        // Count files
        let mut all_files: std::collections::HashSet<String> = std::collections::HashSet::new();
        for r in &self.import_results {
            all_files.insert(r.file_path.clone());
        }
        for r in &self.roundtrip_results {
            all_files.insert(r.source_path.clone());
        }
        self.files_tested = all_files.len();

        // Count passed (score >= 90%)
        self.files_passed = self
            .roundtrip_results
            .iter()
            .filter(|r| r.score >= 90.0)
            .count();
    }

    /// Analyze results and generate issues
    pub fn analyze_issues(&mut self) {
        self.issues.clear();

        // Check import issues
        for result in &self.import_results {
            if !result.success {
                self.issues.push(CompatibilityIssue {
                    severity: IssueSeverity::Error,
                    category: "Import".to_string(),
                    description: "Failed to import file".to_string(),
                    file: Some(result.file_path.clone()),
                    suggestion: result.error.clone(),
                });
            } else if result.text_frames > 0 && result.text_frames_with_content == 0 {
                self.issues.push(CompatibilityIssue {
                    severity: IssueSeverity::Warning,
                    category: "Import".to_string(),
                    description: format!(
                        "Found {} text frames but no text content extracted",
                        result.text_frames
                    ),
                    file: Some(result.file_path.clone()),
                    suggestion: Some("Check Quill stream parsing".to_string()),
                });
            }
        }

        // Check export issues
        for result in &self.export_results {
            if !result.success {
                self.issues.push(CompatibilityIssue {
                    severity: IssueSeverity::Error,
                    category: "Export".to_string(),
                    description: "Export failed".to_string(),
                    file: None,
                    suggestion: result.error.clone(),
                });
            } else if !result.valid_ole {
                self.issues.push(CompatibilityIssue {
                    severity: IssueSeverity::Error,
                    category: "Export".to_string(),
                    description: "Exported file is not a valid OLE document".to_string(),
                    file: None,
                    suggestion: Some("Check CFB file creation".to_string()),
                });
            } else if !result.valid_contents_header {
                self.issues.push(CompatibilityIssue {
                    severity: IssueSeverity::Warning,
                    category: "Export".to_string(),
                    description: "Contents stream header is invalid".to_string(),
                    file: None,
                    suggestion: Some("Check trailer offset encoding at 0x1A".to_string()),
                });
            } else if !result.reimportable {
                self.issues.push(CompatibilityIssue {
                    severity: IssueSeverity::Error,
                    category: "Export".to_string(),
                    description: "Exported file cannot be re-imported".to_string(),
                    file: None,
                    suggestion: result.reimport_error.clone(),
                });
            }
        }

        // Check round-trip issues
        for result in &self.roundtrip_results {
            if !result.reimport_success {
                self.issues.push(CompatibilityIssue {
                    severity: IssueSeverity::Error,
                    category: "Round-trip".to_string(),
                    description: "Round-trip failed at re-import".to_string(),
                    file: Some(result.source_path.clone()),
                    suggestion: result.reimport_error.clone(),
                });
            } else if !result.comparison.pages_match {
                self.issues.push(CompatibilityIssue {
                    severity: IssueSeverity::Error,
                    category: "Round-trip".to_string(),
                    description: format!(
                        "Page count mismatch: {} → {}",
                        result.original.pages, result.reimported.pages
                    ),
                    file: Some(result.source_path.clone()),
                    suggestion: Some("Check document chunk and page list encoding".to_string()),
                });
            } else if result.comparison.text_match_percent < 90.0 {
                self.issues.push(CompatibilityIssue {
                    severity: IssueSeverity::Warning,
                    category: "Round-trip".to_string(),
                    description: format!(
                        "Text content loss: {:.0}% retained",
                        result.comparison.text_match_percent
                    ),
                    file: Some(result.source_path.clone()),
                    suggestion: Some("Check Quill stream encoding".to_string()),
                });
            }
        }

        // Sort by severity
        self.issues.sort_by(|a, b| {
            let severity_order = |s: &IssueSeverity| match s {
                IssueSeverity::Error => 0,
                IssueSeverity::Warning => 1,
                IssueSeverity::Info => 2,
            };
            severity_order(&a.severity).cmp(&severity_order(&b.severity))
        });
    }

    /// Generate a text report
    pub fn to_text(&self) -> String {
        let mut s = String::new();

        s.push_str(
            &"╔══════════════════════════════════════════════════════════════╗\n".to_string(),
        );
        s.push_str(
            &"║  Publisher Compatibility Report                              ║\n".to_string(),
        );
        s.push_str(&format!("║  {}  ║\n", self.title));
        s.push_str(
            &"╠══════════════════════════════════════════════════════════════╣\n".to_string(),
        );
        s.push_str(
            &"║                                                              ║\n".to_string(),
        );
        s.push_str(&format!(
            "║  OVERALL SCORE: {:>5.1}%                                      ║\n",
            self.overall_score
        ));
        s.push_str(
            &"║                                                              ║\n".to_string(),
        );
        s.push_str(
            &"╠══════════════════════════════════════════════════════════════╣\n".to_string(),
        );
        s.push_str(
            &"║  Category Scores:                                            ║\n".to_string(),
        );
        s.push_str(&format!(
            "║    Import:     {:>5.1}% ({} files)                            ║\n",
            self.import_score,
            self.import_results.len()
        ));
        s.push_str(&format!(
            "║    Export:     {:>5.1}% ({} tests)                            ║\n",
            self.export_score,
            self.export_results.len()
        ));
        s.push_str(&format!(
            "║    Round-trip: {:>5.1}% ({} files)                            ║\n",
            self.roundtrip_score,
            self.roundtrip_results.len()
        ));
        s.push_str(
            &"║                                                              ║\n".to_string(),
        );
        s.push_str(&format!(
            "║  Files: {}/{} passed (≥90%)                                  ║\n",
            self.files_passed, self.files_tested
        ));
        s.push_str(
            &"╚══════════════════════════════════════════════════════════════╝\n".to_string(),
        );

        // Issues
        if !self.issues.is_empty() {
            s.push('\n');
            s.push_str("ISSUES FOUND:\n");
            s.push_str("─────────────\n");

            for issue in &self.issues {
                let severity = match issue.severity {
                    IssueSeverity::Error => "ERROR",
                    IssueSeverity::Warning => "WARN ",
                    IssueSeverity::Info => "INFO ",
                };
                s.push_str(&format!(
                    "[{}] {}: {}\n",
                    severity, issue.category, issue.description
                ));
                if let Some(file) = &issue.file {
                    s.push_str(&format!("         File: {}\n", file));
                }
                if let Some(suggestion) = &issue.suggestion {
                    s.push_str(&format!("         → {}\n", suggestion));
                }
            }
        }

        // Per-file details
        if !self.roundtrip_results.is_empty() {
            s.push('\n');
            s.push_str("ROUND-TRIP DETAILS:\n");
            s.push_str("───────────────────\n");

            for result in &self.roundtrip_results {
                let status = if result.score >= 90.0 {
                    "✓"
                } else if result.score >= 60.0 {
                    "△"
                } else {
                    "✗"
                };
                s.push_str(&format!(
                    "{} {:>5.1}% | {}\n",
                    status, result.score, result.source_path
                ));

                if result.reimport_success {
                    s.push_str(&format!(
                        "         Pages: {} → {}, Objects: {} → {}\n",
                        result.original.pages,
                        result.reimported.pages,
                        result.original.total_objects,
                        result.reimported.total_objects
                    ));
                } else if let Some(err) = &result.reimport_error {
                    s.push_str(&format!("         Error: {}\n", err));
                }
            }
        }

        s
    }

    /// Generate a JSON report
    pub fn to_json(&self) -> String {
        // Simple manual JSON generation
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str(&format!("  \"title\": \"{}\",\n", self.title));
        s.push_str(&format!("  \"timestamp\": {},\n", self.timestamp));
        s.push_str(&format!(
            "  \"overall_score\": {:.2},\n",
            self.overall_score
        ));
        s.push_str(&format!("  \"import_score\": {:.2},\n", self.import_score));
        s.push_str(&format!("  \"export_score\": {:.2},\n", self.export_score));
        s.push_str(&format!(
            "  \"roundtrip_score\": {:.2},\n",
            self.roundtrip_score
        ));
        s.push_str(&format!("  \"files_tested\": {},\n", self.files_tested));
        s.push_str(&format!("  \"files_passed\": {},\n", self.files_passed));
        s.push_str(&format!("  \"issues_count\": {}\n", self.issues.len()));
        s.push_str("}\n");
        s
    }
}

/// Generate a comprehensive compatibility report for a set of .pub files
pub fn generate_report<P: AsRef<Path>>(test_files: &[P], title: &str) -> CompatibilityReport {
    let mut report = CompatibilityReport::new(title);

    let import_validator = ImportValidator::new();
    let roundtrip_tester = RoundtripTest::new();

    for path in test_files {
        // Import validation
        let import_result = import_validator.validate(path);
        report.add_import(import_result.clone());

        // If import succeeded, do round-trip test
        if import_result.success {
            let roundtrip_result = roundtrip_tester.test(path);
            report.add_roundtrip(roundtrip_result);
        }
    }

    // Analyze issues
    report.analyze_issues();

    report
}

/// Quick single-file compatibility check
pub fn check_file<P: AsRef<Path>>(path: P) -> (f64, String) {
    let roundtrip = RoundtripTest::new().test(path);
    (roundtrip.score, roundtrip.summary())
}
