//! The `validate` command - validate document structure.

use std::fmt;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use repub_format_native as native;
use repub_model::shape::{ImageSource, Shape, ShapeKind};
use serde::Serialize;

use crate::cli::GlobalOptions;
use crate::output;

/// Arguments for the `validate` command.
#[derive(Args, Debug)]
pub struct ValidateArgs {
    /// Input document path
    pub input: PathBuf,

    /// Fix issues where possible (creates backup)
    #[arg(long)]
    pub fix: bool,
}

/// Validation result.
#[derive(Debug, Serialize)]
pub struct ValidationResult {
    /// Whether the document is valid
    pub valid: bool,
    /// Number of errors
    pub errors: usize,
    /// Number of warnings
    pub warnings: usize,
    /// List of issues
    pub issues: Vec<ValidationIssue>,
}

impl fmt::Display for ValidationResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.valid {
            writeln!(f, "Document is valid")?;
        } else {
            writeln!(f, "Document has issues")?;
        }
        writeln!(f, "Errors: {}", self.errors)?;
        writeln!(f, "Warnings: {}", self.warnings)?;
        if !self.issues.is_empty() {
            writeln!(f)?;
            for issue in &self.issues {
                writeln!(f, "{}", issue)?;
            }
        }
        Ok(())
    }
}

/// A validation issue.
#[derive(Debug, Serialize)]
pub struct ValidationIssue {
    /// Severity
    pub severity: Severity,
    /// Issue message
    pub message: String,
    /// Location (e.g., "Page 1, Layer 'Main'")
    pub location: Option<String>,
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.severity {
            Severity::Error => "ERROR",
            Severity::Warning => "WARN ",
            Severity::Info => "INFO ",
        };
        if let Some(ref loc) = self.location {
            write!(f, "[{}] {} ({})", prefix, self.message, loc)
        } else {
            write!(f, "[{}] {}", prefix, self.message)
        }
    }
}

/// Issue severity.
#[derive(Debug, Clone, Copy, Serialize)]
pub enum Severity {
    /// Error - document may not work correctly
    Error,
    /// Warning - potential issue
    Warning,
    /// Info - suggestion
    Info,
}

/// Run the `validate` command.
pub fn run(args: ValidateArgs, opts: &GlobalOptions) -> Result<()> {
    if !args.input.exists() {
        anyhow::bail!("File not found: {}", args.input.display());
    }

    output::verbose(format!("Validating: {}", args.input.display()), opts);

    let doc = native::load(&args.input)
        .with_context(|| format!("Failed to load: {}", args.input.display()))?;

    let mut issues = Vec::new();

    // Check for empty document
    if doc.pages().is_empty() {
        issues.push(ValidationIssue {
            severity: Severity::Warning,
            message: "Document has no pages".to_string(),
            location: None,
        });
    }

    // Validate each page
    for (page_idx, page) in doc.pages().iter().enumerate() {
        let page_num = page_idx + 1;

        // Check page dimensions
        if page.size.width <= 0.0 || page.size.height <= 0.0 {
            issues.push(ValidationIssue {
                severity: Severity::Error,
                message: "Invalid page dimensions".to_string(),
                location: Some(format!("Page {}", page_num)),
            });
        }

        // Check for empty layers
        if page.layers().is_empty() {
            issues.push(ValidationIssue {
                severity: Severity::Warning,
                message: "Page has no layers".to_string(),
                location: Some(format!("Page {}", page_num)),
            });
        }

        // Validate objects
        for layer in page.layers() {
            for obj in layer.shapes() {
                validate_object(obj, &mut issues, page_num, &layer.name);
            }
        }
    }

    // Calculate summary
    let errors = issues
        .iter()
        .filter(|i| matches!(i.severity, Severity::Error))
        .count();
    let warnings = issues
        .iter()
        .filter(|i| matches!(i.severity, Severity::Warning))
        .count();

    let result = ValidationResult {
        valid: errors == 0,
        errors,
        warnings,
        issues,
    };

    output::print_data(&result, opts)?;

    if result.valid {
        output::success("Document is valid", opts);
    } else {
        output::warn(
            format!("Found {} error(s) and {} warning(s)", errors, warnings),
            opts,
        );
    }

    // Fix mode
    if args.fix && !result.valid {
        output::info("--fix is not yet implemented", opts);
    }

    Ok(())
}

fn validate_object(obj: &Shape, issues: &mut Vec<ValidationIssue>, page: usize, layer: &str) {
    let location = Some(format!("Page {}, Layer '{}'", page, layer));
    let bounds = &obj.common.bounds;

    match &obj.kind {
        ShapeKind::AutoShape(_) => {
            // Check bounds
            if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
                issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    message: "Shape has zero or negative dimensions".to_string(),
                    location: location.clone(),
                });
            }
        }
        ShapeKind::TextFrame(_) => {
            if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
                issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    message: "Text frame has zero or negative dimensions".to_string(),
                    location: location.clone(),
                });
            }
        }
        ShapeKind::Picture(pic) => {
            if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
                issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    message: "Image frame has zero or negative dimensions".to_string(),
                    location: location.clone(),
                });
            }

            // Check for placeholder image source
            if matches!(pic.source, ImageSource::Placeholder) {
                issues.push(ValidationIssue {
                    severity: Severity::Info,
                    message: "Image frame has placeholder source (no image loaded)".to_string(),
                    location: location.clone(),
                });
            }
        }
        ShapeKind::Table(t) => {
            if t.rows == 0 || t.cols == 0 {
                issues.push(ValidationIssue {
                    severity: Severity::Error,
                    message: "Table has zero rows or columns".to_string(),
                    location: location.clone(),
                });
            }
        }
        ShapeKind::Group(g) => {
            if g.children.is_empty() {
                issues.push(ValidationIssue {
                    severity: Severity::Warning,
                    message: "Group is empty".to_string(),
                    location: location.clone(),
                });
            }
            for child in &g.children {
                validate_object(child, issues, page, layer);
            }
        }
        ShapeKind::Line(_) | ShapeKind::TextEffect(_) => {
            // Basic validation
        }
    }
}
