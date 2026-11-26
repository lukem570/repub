//! The `convert` command - convert between document formats.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use repub_format_native::{self as native, SaveOptions};

use crate::cli::GlobalOptions;
use crate::output;

/// Arguments for the `convert` command.
#[derive(Args, Debug)]
pub struct ConvertArgs {
    /// Input file path
    pub input: PathBuf,

    /// Output file path
    pub output: PathBuf,

    /// Compress output (for native format)
    #[arg(long)]
    pub compress: bool,

    /// Overwrite existing files
    #[arg(long)]
    pub force: bool,
}

/// Run the `convert` command.
pub fn run(args: ConvertArgs, opts: &GlobalOptions) -> Result<()> {
    if !args.input.exists() {
        anyhow::bail!("File not found: {}", args.input.display());
    }

    if args.output.exists() && !args.force {
        anyhow::bail!(
            "Output file already exists: {}. Use --force to overwrite.",
            args.output.display()
        );
    }

    // Detect input format
    let input_ext = args
        .input
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    // Detect output format
    let output_ext = args
        .output
        .extension()
        .and_then(|e| e.to_str())
        .map(|s| s.to_lowercase())
        .unwrap_or_default();

    output::verbose(format!("Converting {} -> {}", input_ext, output_ext), opts);

    // Currently we only support native format conversions
    match (input_ext.as_str(), output_ext.as_str()) {
        // Native to native (compressed/uncompressed)
        ("pcd", "pcd")
        | ("pcd", "pcdz")
        | ("pcdz", "pcd")
        | ("pcdz", "pcdz")
        | ("json", "pcd")
        | ("pcd", "json") => {
            let doc = native::load(&args.input)
                .with_context(|| format!("Failed to load: {}", args.input.display()))?;

            let save_opts = if args.compress || output_ext == "pcdz" {
                SaveOptions::compressed()
            } else {
                SaveOptions::default()
            };

            native::save(&doc, &args.output, &save_opts)
                .with_context(|| format!("Failed to save: {}", args.output.display()))?;

            output::success(
                format!(
                    "Converted {} to {}",
                    args.input.display(),
                    args.output.display()
                ),
                opts,
            );
        }

        // PUB import (not yet implemented)
        ("pub", _) => {
            output::warn("Publisher (.pub) import is not yet implemented", opts);
            anyhow::bail!("PUB import coming soon");
        }

        _ => {
            anyhow::bail!("Unsupported conversion: {} to {}", input_ext, output_ext);
        }
    }

    Ok(())
}
