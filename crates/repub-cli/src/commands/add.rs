//! The `add` command - add content to documents.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, Subcommand};
use repub_color::Color;
use repub_format_native::{self as native, SaveOptions};
use repub_geometry::Rect;
use repub_model::shape::{Shape, ShapeKind};
use repub_model::styles::Fill;
use repub_model::text::StyledText;

use crate::cli::GlobalOptions;
use crate::output;

/// Add subcommands.
#[derive(Subcommand, Debug)]
pub enum AddCommand {
    /// Add a new page
    Page(AddPageArgs),
    /// Add a shape
    Shape(AddShapeArgs),
    /// Add a text frame
    Text(AddTextArgs),
}

/// Arguments for adding a page.
#[derive(Args, Debug)]
pub struct AddPageArgs {
    /// Document to modify
    pub document: PathBuf,

    /// Position to insert page (1-based, default: end)
    #[arg(short, long)]
    pub position: Option<usize>,
}

/// Arguments for adding a shape.
#[derive(Args, Debug)]
pub struct AddShapeArgs {
    /// Document to modify
    pub document: PathBuf,

    /// Shape type
    #[arg(short = 't', long, default_value = "rectangle")]
    pub shape_type: ShapeType,

    /// X position in points
    #[arg(short, long, default_value = "72")]
    pub x: f64,

    /// Y position in points
    #[arg(short, long, default_value = "72")]
    pub y: f64,

    /// Width in points
    #[arg(short, long, default_value = "144")]
    pub width: f64,

    /// Height in points
    #[arg(short = 'H', long, default_value = "72")]
    pub height: f64,

    /// Fill color (hex, e.g., "ff0000" for red)
    #[arg(long)]
    pub fill: Option<String>,

    /// Page number to add to (1-based, default: 1)
    #[arg(short, long, default_value = "1")]
    pub page: usize,

    /// Corner radius for rectangles
    #[arg(long, default_value = "0")]
    pub radius: f64,

    /// Number of sides for polygons
    #[arg(long, default_value = "6")]
    pub sides: u32,

    /// Number of points for stars
    #[arg(long, default_value = "5")]
    pub points: u32,
}

/// Shape type argument.
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum ShapeType {
    /// Rectangle
    #[default]
    Rectangle,
    /// Ellipse
    Ellipse,
    /// Polygon (use --sides)
    Polygon,
    /// Star (use --points)
    Star,
}

/// Arguments for adding a text frame.
#[derive(Args, Debug)]
pub struct AddTextArgs {
    /// Document to modify
    pub document: PathBuf,

    /// Text content
    #[arg(short = 't', long)]
    pub text: Option<String>,

    /// X position in points
    #[arg(short, long, default_value = "72")]
    pub x: f64,

    /// Y position in points
    #[arg(short, long, default_value = "72")]
    pub y: f64,

    /// Width in points
    #[arg(short, long, default_value = "288")]
    pub width: f64,

    /// Height in points
    #[arg(short = 'H', long, default_value = "144")]
    pub height: f64,

    /// Page number to add to (1-based, default: 1)
    #[arg(short, long, default_value = "1")]
    pub page: usize,
}

/// Run the `add` command.
pub fn run(cmd: AddCommand, opts: &GlobalOptions) -> Result<()> {
    match cmd {
        AddCommand::Page(args) => add_page(args, opts),
        AddCommand::Shape(args) => add_shape(args, opts),
        AddCommand::Text(args) => add_text(args, opts),
    }
}

fn add_page(args: AddPageArgs, opts: &GlobalOptions) -> Result<()> {
    if !args.document.exists() {
        anyhow::bail!("File not found: {}", args.document.display());
    }

    let mut doc = native::load(&args.document)
        .with_context(|| format!("Failed to load: {}", args.document.display()))?;

    let page_count = doc.pages().len();

    if let Some(pos) = args.position {
        if pos == 0 || pos > page_count + 1 {
            anyhow::bail!(
                "Invalid position: {}. Must be 1-{}",
                pos,
                page_count + 1
            );
        }
        doc.insert_page(pos - 1);
        output::verbose(format!("Inserted page at position {}", pos), opts);
    } else {
        doc.add_page();
        output::verbose(format!("Added page at end (position {})", page_count + 1), opts);
    }

    native::save(&doc, &args.document, &SaveOptions::default())
        .with_context(|| format!("Failed to save: {}", args.document.display()))?;

    output::success(
        format!("Added page to {} (now {} pages)", args.document.display(), doc.pages().len()),
        opts,
    );

    Ok(())
}

fn add_shape(args: AddShapeArgs, opts: &GlobalOptions) -> Result<()> {
    if !args.document.exists() {
        anyhow::bail!("File not found: {}", args.document.display());
    }

    let mut doc = native::load(&args.document)
        .with_context(|| format!("Failed to load: {}", args.document.display()))?;

    let page_idx = args.page - 1;
    if page_idx >= doc.pages().len() {
        anyhow::bail!(
            "Page {} does not exist (document has {} pages)",
            args.page,
            doc.pages().len()
        );
    }

    let bounds = Rect::new(args.x, args.y, args.width, args.height);

    let mut shape = match args.shape_type {
        ShapeType::Rectangle => {
            let mut s = Shape::rectangle(bounds);
            if args.radius > 0.0 {
                s = Shape::rounded_rectangle(bounds, args.radius);
            }
            s
        }
        ShapeType::Ellipse => Shape::ellipse(bounds),
        ShapeType::Polygon => Shape::polygon(bounds, args.sides),
        ShapeType::Star => Shape::star(bounds, args.points, 0.5),
    };

    // Apply fill color if specified
    if let Some(ref hex) = args.fill {
        let color = parse_hex_color(hex)?;
        shape.fill = Fill::Solid(color);
    }

    output::verbose(
        format!("Adding {:?} at ({}, {}) size {}x{}", args.shape_type, args.x, args.y, args.width, args.height),
        opts,
    );

    doc.pages_mut()[page_idx].add_shape(shape);

    native::save(&doc, &args.document, &SaveOptions::default())
        .with_context(|| format!("Failed to save: {}", args.document.display()))?;

    output::success(
        format!("Added {:?} to page {} of {}", args.shape_type, args.page, args.document.display()),
        opts,
    );

    Ok(())
}

fn add_text(args: AddTextArgs, opts: &GlobalOptions) -> Result<()> {
    if !args.document.exists() {
        anyhow::bail!("File not found: {}", args.document.display());
    }

    let mut doc = native::load(&args.document)
        .with_context(|| format!("Failed to load: {}", args.document.display()))?;

    let page_idx = args.page - 1;
    if page_idx >= doc.pages().len() {
        anyhow::bail!(
            "Page {} does not exist (document has {} pages)",
            args.page,
            doc.pages().len()
        );
    }

    let bounds = Rect::new(args.x, args.y, args.width, args.height);

    let mut text_frame = Shape::text_frame(bounds);

    if let Some(text) = args.text {
        if let ShapeKind::TextFrame(data) = &mut text_frame.kind {
            data.content = StyledText::plain(&text);
        }
    }

    output::verbose(
        format!("Adding text frame at ({}, {}) size {}x{}", args.x, args.y, args.width, args.height),
        opts,
    );

    doc.pages_mut()[page_idx].add_shape(text_frame);

    native::save(&doc, &args.document, &SaveOptions::default())
        .with_context(|| format!("Failed to save: {}", args.document.display()))?;

    output::success(
        format!("Added text frame to page {} of {}", args.page, args.document.display()),
        opts,
    );

    Ok(())
}

/// Parse a hex color string (e.g., "ff0000" or "#ff0000").
fn parse_hex_color(hex: &str) -> Result<Color> {
    let hex = hex.trim_start_matches('#');

    if hex.len() != 6 {
        anyhow::bail!("Invalid hex color: {}. Expected 6 hex digits.", hex);
    }

    let r = u8::from_str_radix(&hex[0..2], 16)
        .context("Invalid red component")?;
    let g = u8::from_str_radix(&hex[2..4], 16)
        .context("Invalid green component")?;
    let b = u8::from_str_radix(&hex[4..6], 16)
        .context("Invalid blue component")?;

    Ok(Color::from_rgb8(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_color() {
        let red = parse_hex_color("ff0000").unwrap();
        assert_eq!(red.r, 1.0);
        assert_eq!(red.g, 0.0);
        assert_eq!(red.b, 0.0);

        let with_hash = parse_hex_color("#00ff00").unwrap();
        assert_eq!(with_hash.g, 1.0);
    }
}
