//! The `new` command - create a new document.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use repub_format_native::{self as native, SaveOptions};
use repub_model::{Document, PageOrientation, PageSize};

use crate::cli::GlobalOptions;
use crate::output;

/// Arguments for the `new` command.
#[derive(Args, Debug)]
pub struct NewArgs {
    /// Output file path
    pub output: PathBuf,

    /// Page size
    #[arg(short, long, default_value = "letter")]
    pub size: PageSizeArg,

    /// Page orientation
    #[arg(short, long, default_value = "portrait")]
    pub orientation: OrientationArg,

    /// Initial number of pages
    #[arg(short, long, default_value = "1")]
    pub pages: u32,

    /// Custom width in points (requires --size custom)
    #[arg(long)]
    pub width: Option<f64>,

    /// Custom height in points (requires --size custom)
    #[arg(long)]
    pub height: Option<f64>,

    /// Compress the output file
    #[arg(long)]
    pub compress: bool,

    /// Overwrite existing file without prompting
    #[arg(short, long)]
    pub force: bool,
}

/// Page size argument.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum PageSizeArg {
    /// US Letter (8.5" x 11")
    #[default]
    Letter,
    /// US Legal (8.5" x 14")
    Legal,
    /// US Tabloid (11" x 17")
    Tabloid,
    /// ISO A3 (297mm x 420mm)
    A3,
    /// ISO A4 (210mm x 297mm)
    A4,
    /// ISO A5 (148mm x 210mm)
    A5,
    /// Custom size (use --width and --height)
    Custom,
}

/// Page orientation argument.
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
pub enum OrientationArg {
    /// Portrait orientation (taller than wide)
    #[default]
    Portrait,
    /// Landscape orientation (wider than tall)
    Landscape,
}

impl From<OrientationArg> for PageOrientation {
    fn from(arg: OrientationArg) -> Self {
        match arg {
            OrientationArg::Portrait => PageOrientation::Portrait,
            OrientationArg::Landscape => PageOrientation::Landscape,
        }
    }
}

/// Run the `new` command.
pub fn run(args: NewArgs, opts: &GlobalOptions) -> Result<()> {
    // Check if file exists
    if args.output.exists() && !args.force {
        anyhow::bail!(
            "File already exists: {}. Use --force to overwrite.",
            args.output.display()
        );
    }

    // Determine page size
    let page_size = match args.size {
        PageSizeArg::Letter => PageSize::Letter,
        PageSizeArg::Legal => PageSize::Legal,
        PageSizeArg::Tabloid => PageSize::Tabloid,
        PageSizeArg::A3 => PageSize::A3,
        PageSizeArg::A4 => PageSize::A4,
        PageSizeArg::A5 => PageSize::A5,
        PageSizeArg::Custom => {
            let width = args
                .width
                .context("--width is required for custom page size")?;
            let height = args
                .height
                .context("--height is required for custom page size")?;
            PageSize::Custom { width, height }
        }
    };

    let orientation: PageOrientation = args.orientation.into();

    output::verbose(
        format!(
            "Creating document with size {:?}, orientation {:?}",
            page_size, orientation
        ),
        opts,
    );

    // Create the document
    let mut doc = Document::new(page_size, orientation);

    // Add additional pages if requested
    for _ in 1..args.pages {
        doc.add_page();
    }

    output::verbose(format!("Document has {} page(s)", doc.pages().len()), opts);

    // Determine save options
    let save_opts = if args.compress {
        SaveOptions::compressed()
    } else {
        SaveOptions::default()
    };

    // Save the document
    native::save(&doc, &args.output, &save_opts)
        .with_context(|| format!("Failed to save document to {}", args.output.display()))?;

    output::success(format!("Created {}", args.output.display()), opts);

    Ok(())
}
