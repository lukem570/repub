//! The `info` command - display document information.

use std::fmt;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use repub_format_native as native;
use repub_model::shape::{Shape, ShapeKind};
use serde::Serialize;

use crate::cli::GlobalOptions;
use crate::output;

/// Arguments for the `info` command.
#[derive(Args, Debug)]
pub struct InfoArgs {
    /// Input document path
    pub input: PathBuf,
}

/// Document information for output.
#[derive(Debug, Serialize)]
pub struct DocumentInfo {
    /// File path
    pub path: String,
    /// Number of pages
    pub pages: usize,
    /// Page size description
    pub page_size: String,
    /// Page orientation
    pub orientation: String,
    /// Total object count
    pub objects: usize,
    /// Object counts by type
    pub object_counts: ObjectCounts,
}

/// Object counts by type.
#[derive(Debug, Default, Serialize)]
pub struct ObjectCounts {
    /// Number of shapes
    pub shapes: usize,
    /// Number of text frames
    pub text_frames: usize,
    /// Number of image frames
    pub image_frames: usize,
    /// Number of tables
    pub tables: usize,
    /// Number of lines
    pub lines: usize,
    /// Number of groups
    pub groups: usize,
    /// Number of WordArt objects
    pub wordart: usize,
}

impl fmt::Display for DocumentInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Document: {}", self.path)?;
        writeln!(f, "Pages: {}", self.pages)?;
        writeln!(f, "Page Size: {}", self.page_size)?;
        writeln!(f, "Orientation: {}", self.orientation)?;
        writeln!(f, "Total Objects: {}", self.objects)?;
        if self.objects > 0 {
            writeln!(f, "Object Breakdown:")?;
            if self.object_counts.shapes > 0 {
                writeln!(f, "  Shapes: {}", self.object_counts.shapes)?;
            }
            if self.object_counts.text_frames > 0 {
                writeln!(f, "  Text Frames: {}", self.object_counts.text_frames)?;
            }
            if self.object_counts.image_frames > 0 {
                writeln!(f, "  Image Frames: {}", self.object_counts.image_frames)?;
            }
            if self.object_counts.tables > 0 {
                writeln!(f, "  Tables: {}", self.object_counts.tables)?;
            }
            if self.object_counts.lines > 0 {
                writeln!(f, "  Lines: {}", self.object_counts.lines)?;
            }
            if self.object_counts.groups > 0 {
                writeln!(f, "  Groups: {}", self.object_counts.groups)?;
            }
            if self.object_counts.wordart > 0 {
                writeln!(f, "  WordArt: {}", self.object_counts.wordart)?;
            }
        }
        Ok(())
    }
}

/// Run the `info` command.
pub fn run(args: InfoArgs, opts: &GlobalOptions) -> Result<()> {
    if !args.input.exists() {
        anyhow::bail!("File not found: {}", args.input.display());
    }

    let doc = native::load(&args.input)
        .with_context(|| format!("Failed to load document: {}", args.input.display()))?;

    // Count objects
    let mut counts = ObjectCounts::default();
    let mut total = 0;

    for page in doc.pages() {
        for layer in page.layers() {
            for obj in layer.shapes() {
                count_object(obj, &mut counts);
                total += 1;
            }
        }
    }

    // Get page info from first page
    let first_page = doc.pages().first();
    let (page_size, orientation) = if let Some(page) = first_page {
        // Determine orientation from dimensions
        let orient = if page.size.width > page.size.height {
            "Landscape"
        } else {
            "Portrait"
        };
        (
            format!("{:.0} x {:.0} pt", page.size.width, page.size.height),
            orient.to_string(),
        )
    } else {
        ("N/A".to_string(), "N/A".to_string())
    };

    let info = DocumentInfo {
        path: args.input.display().to_string(),
        pages: doc.pages().len(),
        page_size,
        orientation,
        objects: total,
        object_counts: counts,
    };

    output::print_data(&info, opts)?;

    Ok(())
}

fn count_object(obj: &Shape, counts: &mut ObjectCounts) {
    match &obj.kind {
        ShapeKind::AutoShape(_) => counts.shapes += 1,
        ShapeKind::TextFrame(_) => counts.text_frames += 1,
        ShapeKind::Picture(_) => counts.image_frames += 1,
        ShapeKind::Table(_) => counts.tables += 1,
        ShapeKind::Line(_) => counts.lines += 1,
        ShapeKind::Group(g) => {
            counts.groups += 1;
            for child in &g.children {
                count_object(child, counts);
            }
        }
        ShapeKind::TextEffect(_) => counts.wordart += 1,
    }
}
