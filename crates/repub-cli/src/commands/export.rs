//! The `export` command - export documents to various formats.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use repub_export_raster::{export_image, ExportFormat as RasterFormat, RasterExportOptions};
use repub_export_vector::{export_pdf, export_svg, PdfOptions, SvgOptions};
use repub_fonts::SystemFontProvider;
use repub_format_native as native;

use crate::cli::GlobalOptions;
use crate::output;

/// Arguments for the `export` command.
#[derive(Args, Debug)]
pub struct ExportArgs {
    /// Input document path
    pub input: PathBuf,

    /// Output file path
    pub output: PathBuf,

    /// Export format (auto-detected from extension if not specified)
    #[arg(short, long)]
    pub format: Option<ExportFormat>,

    /// Pages to export (e.g., "1-5", "1,3,5", "all")
    #[arg(short, long, default_value = "all")]
    pub pages: String,

    /// DPI for raster formats
    #[arg(long, default_value = "150")]
    pub dpi: f64,

    /// JPEG quality (1-100)
    #[arg(long, default_value = "90")]
    pub quality: u8,

    /// Include crop marks (PDF only)
    #[arg(long)]
    pub crop_marks: bool,

    /// Bleed amount in points (PDF only)
    #[arg(long, default_value = "0")]
    pub bleed: f64,

    /// Overwrite existing files
    #[arg(long)]
    pub force: bool,
}

/// Export format options.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ExportFormat {
    /// PDF document
    Pdf,
    /// SVG vector graphics
    Svg,
    /// PNG image
    Png,
    /// JPEG image
    Jpg,
    /// TIFF image
    Tiff,
}

impl ExportFormat {
    /// Detect format from file extension.
    pub fn from_extension(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "pdf" => Some(Self::Pdf),
            "svg" => Some(Self::Svg),
            "png" => Some(Self::Png),
            "jpg" | "jpeg" => Some(Self::Jpg),
            "tiff" | "tif" => Some(Self::Tiff),
            _ => None,
        }
    }
}

/// Run the `export` command.
pub fn run(args: ExportArgs, opts: &GlobalOptions) -> Result<()> {
    if !args.input.exists() {
        anyhow::bail!("File not found: {}", args.input.display());
    }

    if args.output.exists() && !args.force {
        anyhow::bail!(
            "Output file already exists: {}. Use --force to overwrite.",
            args.output.display()
        );
    }

    // Determine format
    let format = args
        .format
        .or_else(|| ExportFormat::from_extension(&args.output))
        .context("Could not determine export format. Use --format to specify.")?;

    output::verbose(format!("Loading document: {}", args.input.display()), opts);

    let doc = native::load(&args.input)
        .with_context(|| format!("Failed to load document: {}", args.input.display()))?;

    // Parse page range
    let page_indices = parse_page_range(&args.pages, doc.pages().len())?;

    output::verbose(
        format!("Exporting {} page(s) as {:?}", page_indices.len(), format),
        opts,
    );

    // The scene-driven exporters resolve and measure fonts through a provider.
    let fonts = SystemFontProvider::new().context("Failed to load system fonts")?;

    match format {
        ExportFormat::Pdf => {
            if args.crop_marks || args.bleed != 0.0 {
                output::warn(
                    "Crop marks and bleed are not supported by the current PDF exporter and will be ignored",
                    opts,
                );
            }

            let pdf_opts = PdfOptions::default();

            // The PDF exporter writes one PDF with every document page; it does
            // not support exporting a page subset.
            export_pdf(&doc, &args.output, &pdf_opts, &fonts)
                .with_context(|| format!("Failed to export PDF to {}", args.output.display()))?;

            output::success(format!("Exported PDF to {}", args.output.display()), opts);
        }

        ExportFormat::Svg => {
            let svg_opts = SvgOptions::default();

            if page_indices.len() == 1 {
                // Single page
                export_svg(&doc, page_indices[0], &args.output, &svg_opts, &fonts)
                    .with_context(|| format!("Failed to export SVG to {}", args.output.display()))?;
                output::success(format!("Exported SVG to {}", args.output.display()), opts);
            } else {
                // Multiple pages - add page number to filename
                for &page_idx in &page_indices {
                    let path = add_page_suffix(&args.output, page_idx + 1);
                    export_svg(&doc, page_idx, &path, &svg_opts, &fonts)
                        .with_context(|| format!("Failed to export SVG to {}", path.display()))?;
                    output::verbose(format!("Exported page {} to {}", page_idx + 1, path.display()), opts);
                }
                output::success(
                    format!("Exported {} SVG files", page_indices.len()),
                    opts,
                );
            }
        }

        ExportFormat::Png | ExportFormat::Jpg | ExportFormat::Tiff => {
            let raster_format = match format {
                ExportFormat::Jpg => RasterFormat::Jpeg,
                ExportFormat::Tiff => RasterFormat::Tiff,
                _ => RasterFormat::Png,
            };
            let raster_opts = RasterExportOptions {
                format: raster_format,
                dpi: args.dpi,
                quality: args.quality,
                ..RasterExportOptions::default()
            };

            if page_indices.len() == 1 {
                export_image(&doc, page_indices[0], &args.output, &raster_opts, &fonts)
                    .with_context(|| format!("Failed to export image to {}", args.output.display()))?;
                output::success(format!("Exported image to {}", args.output.display()), opts);
            } else {
                for &page_idx in &page_indices {
                    let path = add_page_suffix(&args.output, page_idx + 1);
                    export_image(&doc, page_idx, &path, &raster_opts, &fonts)
                        .with_context(|| format!("Failed to export image to {}", path.display()))?;
                    output::verbose(format!("Exported page {} to {}", page_idx + 1, path.display()), opts);
                }
                output::success(
                    format!("Exported {} image files", page_indices.len()),
                    opts,
                );
            }
        }
    }

    Ok(())
}

/// Parse a page range string like "1-5", "1,3,5", or "all".
fn parse_page_range(range: &str, total_pages: usize) -> Result<Vec<usize>> {
    let range = range.trim().to_lowercase();

    if range == "all" {
        return Ok((0..total_pages).collect());
    }

    let mut pages = Vec::new();

    for part in range.split(',') {
        let part = part.trim();
        if part.contains('-') {
            let bounds: Vec<&str> = part.split('-').collect();
            if bounds.len() != 2 {
                anyhow::bail!("Invalid range: {}", part);
            }
            let start: usize = bounds[0].parse().context("Invalid start page number")?;
            let end: usize = bounds[1].parse().context("Invalid end page number")?;

            if start == 0 || end == 0 {
                anyhow::bail!("Page numbers start at 1");
            }
            if start > end {
                anyhow::bail!("Invalid range: start > end");
            }
            if end > total_pages {
                anyhow::bail!("Page {} does not exist (document has {} pages)", end, total_pages);
            }

            for p in start..=end {
                pages.push(p - 1); // Convert to 0-based index
            }
        } else {
            let p: usize = part.parse().context("Invalid page number")?;
            if p == 0 {
                anyhow::bail!("Page numbers start at 1");
            }
            if p > total_pages {
                anyhow::bail!("Page {} does not exist (document has {} pages)", p, total_pages);
            }
            pages.push(p - 1);
        }
    }

    // Remove duplicates and sort
    pages.sort();
    pages.dedup();

    Ok(pages)
}

/// Add a page number suffix to a file path.
fn add_page_suffix(path: &Path, page_num: usize) -> PathBuf {
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let ext = path.extension().unwrap_or_default().to_string_lossy();
    let parent = path.parent().unwrap_or(std::path::Path::new("."));

    parent.join(format!("{}-page{}.{}", stem, page_num, ext))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_page_range_all() {
        let pages = parse_page_range("all", 5).unwrap();
        assert_eq!(pages, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn test_parse_page_range_single() {
        let pages = parse_page_range("3", 5).unwrap();
        assert_eq!(pages, vec![2]);
    }

    #[test]
    fn test_parse_page_range_range() {
        let pages = parse_page_range("2-4", 5).unwrap();
        assert_eq!(pages, vec![1, 2, 3]);
    }

    #[test]
    fn test_parse_page_range_list() {
        let pages = parse_page_range("1,3,5", 5).unwrap();
        assert_eq!(pages, vec![0, 2, 4]);
    }

    #[test]
    fn test_parse_page_range_mixed() {
        let pages = parse_page_range("1-2,4", 5).unwrap();
        assert_eq!(pages, vec![0, 1, 3]);
    }

    #[test]
    fn test_add_page_suffix() {
        let path = PathBuf::from("output.svg");
        let result = add_page_suffix(&path, 3);
        assert_eq!(result, PathBuf::from("output-page3.svg"));
    }
}
