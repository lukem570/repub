//! The `list` command - list document contents.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use repub_format_native as native;
use repub_model::shape::{Shape, ShapeKind};
use repub_model::geometry::ShapeGeometry;
use serde::Serialize;
use tabled::Tabled;

use crate::cli::{GlobalOptions, OutputFormat};
use crate::output;

/// Arguments for the `list` command.
#[derive(Args, Debug)]
pub struct ListArgs {
    /// Input document path
    pub input: PathBuf,

    /// List pages
    #[arg(long)]
    pub pages: bool,

    /// List objects
    #[arg(long)]
    pub objects: bool,

    /// List layers
    #[arg(long)]
    pub layers: bool,

    /// Filter by page number (1-based)
    #[arg(short, long)]
    pub page: Option<usize>,
}

/// Page information for display.
#[derive(Debug, Serialize, Tabled)]
pub struct PageInfo {
    /// Page number (1-based)
    #[tabled(rename = "#")]
    pub number: usize,
    /// Page width
    #[tabled(rename = "Width")]
    pub width: String,
    /// Page height
    #[tabled(rename = "Height")]
    pub height: String,
    /// Orientation
    #[tabled(rename = "Orientation")]
    pub orientation: String,
    /// Object count
    #[tabled(rename = "Objects")]
    pub objects: usize,
}

/// Object information for display.
#[derive(Debug, Serialize, Tabled)]
pub struct ObjectInfo {
    /// Page number
    #[tabled(rename = "Page")]
    pub page: usize,
    /// Layer name
    #[tabled(rename = "Layer")]
    pub layer: String,
    /// Object type
    #[tabled(rename = "Type")]
    pub obj_type: String,
    /// Position
    #[tabled(rename = "Position")]
    pub position: String,
    /// Size
    #[tabled(rename = "Size")]
    pub size: String,
}

/// Layer information for display.
#[derive(Debug, Serialize, Tabled)]
pub struct LayerInfo {
    /// Page number
    #[tabled(rename = "Page")]
    pub page: usize,
    /// Layer name
    #[tabled(rename = "Name")]
    pub name: String,
    /// Visible
    #[tabled(rename = "Visible")]
    pub visible: String,
    /// Locked
    #[tabled(rename = "Locked")]
    pub locked: String,
    /// Object count
    #[tabled(rename = "Objects")]
    pub objects: usize,
}

/// Run the `list` command.
pub fn run(args: ListArgs, opts: &GlobalOptions) -> Result<()> {
    if !args.input.exists() {
        anyhow::bail!("File not found: {}", args.input.display());
    }

    let doc = native::load(&args.input)
        .with_context(|| format!("Failed to load: {}", args.input.display()))?;

    // Default to listing pages if nothing specified
    let show_pages = args.pages || (!args.objects && !args.layers);
    let show_objects = args.objects;
    let show_layers = args.layers;

    if show_pages {
        list_pages(&doc, args.page, opts)?;
    }

    if show_layers {
        list_layers(&doc, args.page, opts)?;
    }

    if show_objects {
        list_objects(&doc, args.page, opts)?;
    }

    Ok(())
}

fn list_pages(
    doc: &repub_model::Document,
    filter_page: Option<usize>,
    opts: &GlobalOptions,
) -> Result<()> {
    let mut pages: Vec<PageInfo> = Vec::new();

    for (i, page) in doc.pages().iter().enumerate() {
        let page_num = i + 1;

        // Apply filter
        if let Some(filter) = filter_page {
            if page_num != filter {
                continue;
            }
        }

        let obj_count: usize = page.layers().iter().map(|l| l.shapes().len()).sum();

        let orient = if page.size.width > page.size.height {
            "Landscape"
        } else {
            "Portrait"
        };

        pages.push(PageInfo {
            number: page_num,
            width: format!("{:.1} pt", page.size.width),
            height: format!("{:.1} pt", page.size.height),
            orientation: orient.to_string(),
            objects: obj_count,
        });
    }

    match opts.output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&pages)?;
            println!("{json}");
        }
        OutputFormat::Text => {
            if pages.is_empty() {
                output::info("No pages found", opts);
            } else {
                output::print_table(&pages, opts);
            }
        }
    }

    Ok(())
}

fn list_layers(
    doc: &repub_model::Document,
    filter_page: Option<usize>,
    opts: &GlobalOptions,
) -> Result<()> {
    let mut layers: Vec<LayerInfo> = Vec::new();

    for (i, page) in doc.pages().iter().enumerate() {
        let page_num = i + 1;

        if let Some(filter) = filter_page {
            if page_num != filter {
                continue;
            }
        }

        for layer in page.layers() {
            layers.push(LayerInfo {
                page: page_num,
                name: layer.name.clone(),
                visible: if layer.visible { "✓" } else { "✗" }.to_string(),
                locked: if layer.locked { "✓" } else { "✗" }.to_string(),
                objects: layer.shapes().len(),
            });
        }
    }

    match opts.output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&layers)?;
            println!("{json}");
        }
        OutputFormat::Text => {
            if layers.is_empty() {
                output::info("No layers found", opts);
            } else {
                output::print_table(&layers, opts);
            }
        }
    }

    Ok(())
}

fn list_objects(
    doc: &repub_model::Document,
    filter_page: Option<usize>,
    opts: &GlobalOptions,
) -> Result<()> {
    let mut objects: Vec<ObjectInfo> = Vec::new();

    for (i, page) in doc.pages().iter().enumerate() {
        let page_num = i + 1;

        if let Some(filter) = filter_page {
            if page_num != filter {
                continue;
            }
        }

        for layer in page.layers() {
            for obj in layer.shapes() {
                collect_object_info(&mut objects, obj, page_num, &layer.name);
            }
        }
    }

    match opts.output {
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&objects)?;
            println!("{json}");
        }
        OutputFormat::Text => {
            if objects.is_empty() {
                output::info("No objects found", opts);
            } else {
                output::print_table(&objects, opts);
            }
        }
    }

    Ok(())
}

fn collect_object_info(
    objects: &mut Vec<ObjectInfo>,
    obj: &Shape,
    page: usize,
    layer: &str,
) {
    let bounds = &obj.common.bounds;
    let obj_type = match &obj.kind {
        ShapeKind::AutoShape(auto) => {
            let shape_type = match &auto.geometry {
                ShapeGeometry::Rectangle { .. } => "Rectangle",
                ShapeGeometry::Ellipse => "Ellipse",
                ShapeGeometry::Polygon { .. } => "Polygon",
                ShapeGeometry::Star { .. } => "Star",
                ShapeGeometry::Path(_) => "Path",
            };
            shape_type.to_string()
        }
        ShapeKind::TextFrame(_) => "TextFrame".to_string(),
        ShapeKind::Picture(_) => "ImageFrame".to_string(),
        ShapeKind::Table(_) => "Table".to_string(),
        ShapeKind::Line(l) => {
            objects.push(ObjectInfo {
                page,
                layer: layer.to_string(),
                obj_type: "Line".to_string(),
                position: format!("({:.1}, {:.1})", l.start.x, l.start.y),
                size: format!("to ({:.1}, {:.1})", l.end.x, l.end.y),
            });
            return;
        }
        ShapeKind::Group(g) => {
            objects.push(ObjectInfo {
                page,
                layer: layer.to_string(),
                obj_type: format!("Group ({})", g.children.len()),
                position: format!("({:.1}, {:.1})", bounds.origin.x, bounds.origin.y),
                size: format!("{:.1}x{:.1}", bounds.width(), bounds.height()),
            });
            for child in &g.children {
                collect_object_info(objects, child, page, layer);
            }
            return;
        }
        ShapeKind::TextEffect(_) => "WordArt".to_string(),
    };

    objects.push(ObjectInfo {
        page,
        layer: layer.to_string(),
        obj_type,
        position: format!("({:.1}, {:.1})", bounds.origin.x, bounds.origin.y),
        size: format!("{:.1}x{:.1}", bounds.width(), bounds.height()),
    });
}
