//! Import comparison tool - compares Repub's .pub parsing with libmspub reference
//!
//! Usage:
//!   repub-import-compare <input.pub> [--reference <json>] [--output <json>]
//!
//! This tool parses a Publisher file and outputs structured data about its contents,
//! optionally comparing with a reference JSON (e.g., from libmspub or manual inspection).

use anyhow::{Context, Result};
use clap::Parser;
use repub_model::shape::{ImageSource, Shape, ShapeKind};
use repub_model::styles::{FontWeight, FontStyle, TextAlignment, LineSpacing};
use repub_model::text::InlineElement;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "repub-import-compare")]
#[command(about = "Compare Repub Publisher import with reference data")]
struct Args {
    /// Input Publisher file (.pub)
    #[arg(required = true)]
    input: PathBuf,

    /// Reference JSON file for comparison
    #[arg(short, long)]
    reference: Option<PathBuf>,

    /// Output JSON file for parsed data
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Show detailed text content
    #[arg(long)]
    verbose: bool,

    /// Output format: json, text, or summary
    #[arg(long, default_value = "text")]
    format: String,
}

/// Structured representation of parsed document for comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedDocument {
    filename: String,
    page_count: usize,
    pages: Vec<ParsedPage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedPage {
    index: usize,
    width: f64,
    height: f64,
    objects: Vec<ParsedObject>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum ParsedObject {
    TextFrame(ParsedTextFrame),
    Table(ParsedTable),
    Shape(ParsedShape),
    Image(ParsedImage),
    Other { description: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedTextFrame {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    paragraphs: Vec<ParsedParagraph>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedTable {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    rows: u32,
    cols: u32,
    row_heights: Vec<f64>,
    col_widths: Vec<f64>,
    cells: Vec<ParsedCell>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedCell {
    row: usize,
    col: usize,
    paragraphs: Vec<ParsedParagraph>,
    has_fill: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedParagraph {
    alignment: String,
    line_spacing: String,
    runs: Vec<ParsedRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedRun {
    text: String,
    font_family: String,
    font_size: f64,
    bold: bool,
    italic: bool,
    underline: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedShape {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    shape_type: String,
    has_fill: bool,
    has_stroke: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ParsedImage {
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    source_type: String,
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("Loading {}...", args.input.display());
    let doc = repub_format_pub::import(&args.input)
        .context("Failed to import Publisher file")?;

    // Parse document into structured format
    let parsed = parse_document(&doc, &args.input);

    // Output based on format
    match args.format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&parsed)?;
            if let Some(ref output_path) = args.output {
                std::fs::write(output_path, &json)?;
                println!("Written to: {}", output_path.display());
            } else {
                println!("{}", json);
            }
        }
        "text" => {
            print_document(&parsed, args.verbose);
        }
        "summary" => {
            print_summary(&parsed);
        }
        _ => {
            anyhow::bail!("Unknown format: {}. Use json, text, or summary", args.format);
        }
    }

    // Compare with reference if provided
    if let Some(ref ref_path) = args.reference {
        println!("\n========== Comparison with Reference ==========");
        let ref_json = std::fs::read_to_string(ref_path)
            .context("Failed to read reference file")?;
        let reference: ParsedDocument = serde_json::from_str(&ref_json)
            .context("Failed to parse reference JSON")?;
        
        compare_documents(&parsed, &reference);
    }

    // Always save output JSON if path provided
    if let Some(ref output_path) = args.output {
        let json = serde_json::to_string_pretty(&parsed)?;
        std::fs::write(output_path, &json)?;
        println!("\nParsed data saved to: {}", output_path.display());
    }

    Ok(())
}

fn parse_document(doc: &repub_model::Document, path: &std::path::Path) -> ParsedDocument {
    let filename = path.file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let pages: Vec<ParsedPage> = doc.pages().iter().enumerate().map(|(i, page)| {
        let objects: Vec<ParsedObject> = page.layers().iter()
            .flat_map(|layer| layer.shapes().iter())
            .map(|obj| parse_object(obj))
            .collect();

        ParsedPage {
            index: i,
            width: page.size.width,
            height: page.size.height,
            objects,
        }
    }).collect();

    ParsedDocument {
        filename,
        page_count: pages.len(),
        pages,
    }
}

fn parse_object(obj: &Shape) -> ParsedObject {
    let bounds = obj.common.bounds;
    match &obj.kind {
        ShapeKind::TextFrame(tf) => {
            ParsedObject::TextFrame(ParsedTextFrame {
                x: bounds.origin.x,
                y: bounds.origin.y,
                width: bounds.width(),
                height: bounds.height(),
                paragraphs: tf.content.paragraphs.iter().map(|p| parse_paragraph(p)).collect(),
            })
        }
        ShapeKind::Table(table) => {
            let cols = table.cols as usize;
            let cells: Vec<ParsedCell> = table.cells.iter().enumerate().map(|(i, cell)| {
                ParsedCell {
                    row: i / cols,
                    col: i % cols,
                    paragraphs: cell.content.paragraphs.iter().map(|p| parse_paragraph(p)).collect(),
                    has_fill: !matches!(cell.fill, repub_model::styles::Fill::None),
                }
            }).collect();

            ParsedObject::Table(ParsedTable {
                x: bounds.origin.x,
                y: bounds.origin.y,
                width: bounds.width(),
                height: bounds.height(),
                rows: table.rows,
                cols: table.cols,
                row_heights: table.row_heights.clone(),
                col_widths: table.col_widths.clone(),
                cells,
            })
        }
        ShapeKind::AutoShape(auto) => {
            ParsedObject::Shape(ParsedShape {
                x: bounds.origin.x,
                y: bounds.origin.y,
                width: bounds.width(),
                height: bounds.height(),
                shape_type: format!("{:?}", auto.geometry),
                has_fill: !matches!(obj.fill, repub_model::styles::Fill::None),
                has_stroke: obj.line.is_some(),
            })
        }
        ShapeKind::Line(_) => {
            ParsedObject::Shape(ParsedShape {
                x: bounds.origin.x,
                y: bounds.origin.y,
                width: bounds.width(),
                height: bounds.height(),
                shape_type: "Line".to_string(),
                has_fill: !matches!(obj.fill, repub_model::styles::Fill::None),
                has_stroke: obj.line.is_some(),
            })
        }
        ShapeKind::Picture(pic) => {
            let source_type = match &pic.source {
                ImageSource::File(_) => "file",
                ImageSource::Url(_) => "url",
                ImageSource::Embedded { .. } => "embedded",
                ImageSource::Placeholder => "placeholder",
            };
            ParsedObject::Image(ParsedImage {
                x: bounds.origin.x,
                y: bounds.origin.y,
                width: bounds.width(),
                height: bounds.height(),
                source_type: source_type.to_string(),
            })
        }
        _ => ParsedObject::Other {
            description: format!("{:?}", obj.shape_type()),
        },
    }
}

fn parse_paragraph(para: &repub_model::text::Paragraph) -> ParsedParagraph {
    let alignment = match para.style.alignment {
        TextAlignment::Left => "left",
        TextAlignment::Center => "center",
        TextAlignment::Right => "right",
        TextAlignment::Justify => "justify",
    };

    let line_spacing = match para.style.line_spacing {
        LineSpacing::Single => "single".to_string(),
        LineSpacing::OneAndHalf => "1.5".to_string(),
        LineSpacing::Double => "double".to_string(),
        LineSpacing::Multiple(m) => format!("{}x", m),
        LineSpacing::Exact(pts) => format!("{}pt", pts),
        LineSpacing::AtLeast(pts) => format!(">={}pt", pts),
    };

    let runs: Vec<ParsedRun> = para.elements.iter().filter_map(|elem| {
        match elem {
            InlineElement::Text(run) => {
                Some(ParsedRun {
                    text: run.text.clone(),
                    font_family: run.style.font.family.clone(),
                    font_size: run.style.font_size,
                    bold: matches!(run.style.font.weight, 
                        FontWeight::Bold | FontWeight::SemiBold | 
                        FontWeight::ExtraBold | FontWeight::Black),
                    italic: matches!(run.style.font.style,
                        FontStyle::Italic | FontStyle::Oblique),
                    underline: !matches!(run.style.underline, 
                        repub_model::styles::UnderlineStyle::None),
                })
            }
            _ => None,
        }
    }).collect();

    ParsedParagraph {
        alignment: alignment.to_string(),
        line_spacing,
        runs,
    }
}

fn print_document(doc: &ParsedDocument, verbose: bool) {
    println!("\n========== {} ==========", doc.filename);
    println!("Pages: {}", doc.page_count);

    for page in &doc.pages {
        println!("\n--- Page {} ({:.0}x{:.0}pt) ---", page.index + 1, page.width, page.height);
        println!("Objects: {}", page.objects.len());

        for obj in &page.objects {
            match obj {
                ParsedObject::TextFrame(tf) => {
                    println!("  TextFrame at ({:.0},{:.0}) {:.0}x{:.0}pt", 
                        tf.x, tf.y, tf.width, tf.height);
                    for para in &tf.paragraphs {
                        print_paragraph(para, verbose, "    ");
                    }
                }
                ParsedObject::Table(table) => {
                    println!("  Table {}x{} at ({:.0},{:.0})", 
                        table.rows, table.cols, table.x, table.y);
                    println!("    Row heights: {:?}", table.row_heights);
                    println!("    Col widths: {:?}", table.col_widths);
                    for cell in &table.cells {
                        if cell.paragraphs.iter().any(|p| !p.runs.is_empty()) {
                            println!("    Cell[{},{}]:", cell.row, cell.col);
                            for para in &cell.paragraphs {
                                print_paragraph(para, verbose, "      ");
                            }
                        }
                    }
                }
                ParsedObject::Shape(shape) => {
                    println!("  Shape {} at ({:.0},{:.0}) {:.0}x{:.0}", 
                        shape.shape_type, shape.x, shape.y, shape.width, shape.height);
                }
                ParsedObject::Image(img) => {
                    println!("  Image ({}) at ({:.0},{:.0}) {:.0}x{:.0}", 
                        img.source_type, img.x, img.y, img.width, img.height);
                }
                ParsedObject::Other { description } => {
                    println!("  Other: {}", description);
                }
            }
        }
    }
}

fn print_paragraph(para: &ParsedParagraph, verbose: bool, indent: &str) {
    if para.runs.is_empty() {
        return;
    }
    
    let text: String = para.runs.iter().map(|r| r.text.as_str()).collect();
    let preview: String = text.chars().take(50).collect();
    
    if verbose {
        println!("{}Para: align={}, spacing={}", indent, para.alignment, para.line_spacing);
        for run in &para.runs {
            let style_flags = format!("{}{}{}",
                if run.bold { "B" } else { "" },
                if run.italic { "I" } else { "" },
                if run.underline { "U" } else { "" });
            println!("{}  {:.0}pt {} {}: {:?}", 
                indent, run.font_size, run.font_family, style_flags, 
                run.text.chars().take(30).collect::<String>());
        }
    } else {
        // Compact output
        let first_run = para.runs.first().unwrap();
        let style_flags = format!("{}{}{}",
            if first_run.bold { "B" } else { "" },
            if first_run.italic { "I" } else { "" },
            if first_run.underline { "U" } else { "" });
        println!("{}{:.0}pt {} {} [{}]: {:?}", 
            indent, first_run.font_size, first_run.font_family, style_flags,
            para.alignment, preview);
    }
}

fn print_summary(doc: &ParsedDocument) {
    println!("\n========== Summary: {} ==========", doc.filename);
    println!("Pages: {}", doc.page_count);

    let mut text_frames = 0;
    let mut tables = 0;
    let mut shapes = 0;
    let mut images = 0;
    let mut total_runs = 0;
    let mut bold_runs = 0;
    let mut italic_runs = 0;

    for page in &doc.pages {
        for obj in &page.objects {
            match obj {
                ParsedObject::TextFrame(tf) => {
                    text_frames += 1;
                    for para in &tf.paragraphs {
                        for run in &para.runs {
                            total_runs += 1;
                            if run.bold { bold_runs += 1; }
                            if run.italic { italic_runs += 1; }
                        }
                    }
                }
                ParsedObject::Table(table) => {
                    tables += 1;
                    for cell in &table.cells {
                        for para in &cell.paragraphs {
                            for run in &para.runs {
                                total_runs += 1;
                                if run.bold { bold_runs += 1; }
                                if run.italic { italic_runs += 1; }
                            }
                        }
                    }
                }
                ParsedObject::Shape(_) => shapes += 1,
                ParsedObject::Image(_) => images += 1,
                _ => {}
            }
        }
    }

    println!("\nObjects:");
    println!("  Text frames: {}", text_frames);
    println!("  Tables: {}", tables);
    println!("  Shapes: {}", shapes);
    println!("  Images: {}", images);
    
    println!("\nText:");
    println!("  Total text runs: {}", total_runs);
    println!("  Bold runs: {} ({:.1}%)", bold_runs, 
        if total_runs > 0 { bold_runs as f64 / total_runs as f64 * 100.0 } else { 0.0 });
    println!("  Italic runs: {} ({:.1}%)", italic_runs,
        if total_runs > 0 { italic_runs as f64 / total_runs as f64 * 100.0 } else { 0.0 });
}

fn compare_documents(parsed: &ParsedDocument, reference: &ParsedDocument) {
    println!("\nPage count: {} vs {} {}", 
        parsed.page_count, reference.page_count,
        if parsed.page_count == reference.page_count { "✓" } else { "✗" });

    let mut total_checks = 0;
    let mut passed_checks = 0;

    for (i, (p_page, r_page)) in parsed.pages.iter().zip(reference.pages.iter()).enumerate() {
        println!("\nPage {}:", i + 1);
        
        // Compare object counts
        total_checks += 1;
        if p_page.objects.len() == r_page.objects.len() {
            passed_checks += 1;
            println!("  Object count: {} ✓", p_page.objects.len());
        } else {
            println!("  Object count: {} vs {} ✗", p_page.objects.len(), r_page.objects.len());
        }

        // Compare each object type
        for (p_obj, r_obj) in p_page.objects.iter().zip(r_page.objects.iter()) {
            total_checks += 1;
            match (p_obj, r_obj) {
                (ParsedObject::TextFrame(p_tf), ParsedObject::TextFrame(r_tf)) => {
                    let pos_match = (p_tf.x - r_tf.x).abs() < 1.0 && (p_tf.y - r_tf.y).abs() < 1.0;
                    let para_match = p_tf.paragraphs.len() == r_tf.paragraphs.len();
                    if pos_match && para_match {
                        passed_checks += 1;
                        println!("  TextFrame at ({:.0},{:.0}): ✓", p_tf.x, p_tf.y);
                    } else {
                        println!("  TextFrame: position or content mismatch ✗");
                    }
                }
                (ParsedObject::Table(p_t), ParsedObject::Table(r_t)) => {
                    let size_match = p_t.rows == r_t.rows && p_t.cols == r_t.cols;
                    if size_match {
                        passed_checks += 1;
                        println!("  Table {}x{}: ✓", p_t.rows, p_t.cols);
                    } else {
                        println!("  Table: size mismatch {}x{} vs {}x{} ✗", 
                            p_t.rows, p_t.cols, r_t.rows, r_t.cols);
                    }
                }
                _ => {
                    println!("  Object type mismatch ✗");
                }
            }
        }
    }

    println!("\n========== Comparison Result ==========");
    println!("Passed: {}/{} ({:.1}%)", passed_checks, total_checks,
        if total_checks > 0 { passed_checks as f64 / total_checks as f64 * 100.0 } else { 0.0 });
}
