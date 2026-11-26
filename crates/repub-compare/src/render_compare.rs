//! Render comparison tool - compares Repub CPU renderer output with reference images
//!
//! Usage:
//!   repub-render-compare <input.pub> [--reference <dir>] [--output <dir>] [--dpi <dpi>]
//!
//! This tool renders a Publisher file using Repub's CPU renderer and optionally
//! compares the output with reference images (e.g., screenshots from MS Publisher).

use anyhow::{Context, Result};
use clap::Parser;
use image::{GenericImageView, Rgba, RgbaImage};
use repub_export_raster::{export_png, RasterExportOptions};
use repub_fonts::SystemFontProvider;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "repub-render-compare")]
#[command(about = "Compare Repub rendering with reference images")]
struct Args {
    /// Input Publisher file (.pub)
    #[arg(required = true)]
    input: PathBuf,

    /// Directory containing reference images (page_1.png, page_2.png, etc.)
    #[arg(short, long)]
    reference: Option<PathBuf>,

    /// Output directory for rendered images and comparison results
    #[arg(short, long, default_value = "/tmp/repub-compare")]
    output: PathBuf,

    /// DPI for rendering (default: 150)
    #[arg(long, default_value = "150")]
    dpi: f64,

    /// Show detailed per-pixel differences
    #[arg(long)]
    verbose: bool,

    /// Generate diff images highlighting differences
    #[arg(long)]
    diff_images: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Create output directory
    std::fs::create_dir_all(&args.output)?;

    println!("Loading {}...", args.input.display());
    let doc = repub_format_pub::import(&args.input).context("Failed to import Publisher file")?;

    println!("Document has {} pages", doc.pages().len());

    // Font provider for scene composition and glyph rasterization.
    let fonts = SystemFontProvider::new().context("Failed to load system fonts")?;
    let options = RasterExportOptions::png(args.dpi);

    let mut total_pages = 0;
    let mut rendered_pages = 0;
    let mut compared_pages = 0;
    let mut matching_pages = 0;

    for (i, page) in doc.pages().iter().enumerate() {
        total_pages += 1;
        let page_num = i + 1;

        println!(
            "\n--- Page {} ({:.0}x{:.0}pt) ---",
            page_num, page.size.width, page.size.height
        );

        // Render the page
        let rendered_path = args.output.join(format!("rendered_page_{}.png", page_num));
        match export_png(&doc, i, &rendered_path, &options, &fonts) {
            Ok(()) => {
                println!("  Rendered: {}", rendered_path.display());
                rendered_pages += 1;

                // Compare with reference if available
                if let Some(ref ref_dir) = args.reference {
                    let ref_path = ref_dir.join(format!("page_{}.png", page_num));
                    if ref_path.exists() {
                        compared_pages += 1;
                        let result = compare_images(&rendered_path, &ref_path, args.verbose)?;

                        println!("  Reference: {}", ref_path.display());
                        println!("  Similarity: {:.2}%", result.similarity * 100.0);
                        println!(
                            "  Different pixels: {} ({:.2}%)",
                            result.different_pixels,
                            result.different_pixels as f64 / result.total_pixels as f64 * 100.0
                        );

                        if result.similarity > 0.95 {
                            matching_pages += 1;
                            println!("  Status: MATCH ✓");
                        } else if result.similarity > 0.80 {
                            println!("  Status: CLOSE (minor differences)");
                        } else {
                            println!("  Status: DIFFERENT ✗");
                        }

                        // Generate diff image if requested
                        if args.diff_images {
                            let diff_path = args.output.join(format!("diff_page_{}.png", page_num));
                            generate_diff_image(&rendered_path, &ref_path, &diff_path)?;
                            println!("  Diff image: {}", diff_path.display());
                        }
                    } else {
                        println!("  Reference: not found at {}", ref_path.display());
                    }
                }
            }
            Err(e) => {
                println!("  Render error: {}", e);
                // Save error info
                let error_path = args.output.join(format!("error_page_{}.txt", page_num));
                std::fs::write(&error_path, format!("Render error: {}", e))?;
            }
        }
    }

    // Summary
    println!("\n========== Summary ==========");
    println!("Total pages: {}", total_pages);
    println!("Rendered successfully: {}", rendered_pages);
    if args.reference.is_some() {
        println!("Compared with reference: {}", compared_pages);
        println!("Matching (>95% similar): {}", matching_pages);
    }
    println!("Output directory: {}", args.output.display());

    Ok(())
}

#[derive(Debug)]
struct CompareResult {
    similarity: f64,
    different_pixels: usize,
    total_pixels: usize,
    max_diff: u32,
}

fn compare_images(
    rendered_path: &PathBuf,
    reference_path: &PathBuf,
    verbose: bool,
) -> Result<CompareResult> {
    let rendered = image::open(rendered_path).context("Failed to open rendered image")?;
    let reference = image::open(reference_path).context("Failed to open reference image")?;

    let (w1, h1) = rendered.dimensions();
    let (w2, h2) = reference.dimensions();

    // If sizes differ, resize reference to match rendered
    let reference = if (w1, h1) != (w2, h2) {
        if verbose {
            println!(
                "  Size mismatch: rendered={}x{}, reference={}x{}",
                w1, h1, w2, h2
            );
        }
        reference.resize_exact(w1, h1, image::imageops::FilterType::Lanczos3)
    } else {
        reference
    };

    let rendered_rgba = rendered.to_rgba8();
    let reference_rgba = reference.to_rgba8();

    let total_pixels = (w1 * h1) as usize;
    let mut different_pixels = 0;
    let mut max_diff: u32 = 0;
    let mut total_diff: u64 = 0;

    for (r_pixel, ref_pixel) in rendered_rgba.pixels().zip(reference_rgba.pixels()) {
        let diff = pixel_diff(r_pixel, ref_pixel);
        if diff > 0 {
            different_pixels += 1;
            max_diff = max_diff.max(diff);
            total_diff += diff as u64;
        }
    }

    let avg_diff = if different_pixels > 0 {
        total_diff as f64 / different_pixels as f64
    } else {
        0.0
    };

    // Similarity based on percentage of matching pixels and average difference
    let pixel_match_ratio = 1.0 - (different_pixels as f64 / total_pixels as f64);
    let diff_penalty = (avg_diff / 255.0).min(1.0) * 0.5; // Max 50% penalty for high avg diff
    let similarity = (pixel_match_ratio - diff_penalty).max(0.0);

    if verbose {
        println!("  Avg diff per changed pixel: {:.1}", avg_diff);
        println!("  Max single-channel diff: {}", max_diff);
    }

    Ok(CompareResult {
        similarity,
        different_pixels,
        total_pixels,
        max_diff,
    })
}

fn pixel_diff(a: &Rgba<u8>, b: &Rgba<u8>) -> u32 {
    let dr = (a[0] as i32 - b[0] as i32).unsigned_abs();
    let dg = (a[1] as i32 - b[1] as i32).unsigned_abs();
    let db = (a[2] as i32 - b[2] as i32).unsigned_abs();
    let da = (a[3] as i32 - b[3] as i32).unsigned_abs();

    // Consider pixels matching if difference is small (anti-aliasing tolerance)
    let total = dr + dg + db + da;
    if total <= 32 {
        // Tolerance for anti-aliasing
        0
    } else {
        total
    }
}

fn generate_diff_image(
    rendered_path: &PathBuf,
    reference_path: &PathBuf,
    diff_path: &PathBuf,
) -> Result<()> {
    let rendered = image::open(rendered_path)?.to_rgba8();
    let reference = image::open(reference_path)?;

    let (w, h) = rendered.dimensions();
    let reference = reference
        .resize_exact(w, h, image::imageops::FilterType::Lanczos3)
        .to_rgba8();

    let mut diff_img = RgbaImage::new(w, h);

    for (x, y, r_pixel) in rendered.enumerate_pixels() {
        let ref_pixel = reference.get_pixel(x, y);
        let diff = pixel_diff(r_pixel, ref_pixel);

        if diff == 0 {
            // Matching - show dimmed version of rendered
            diff_img.put_pixel(
                x,
                y,
                Rgba([r_pixel[0] / 2, r_pixel[1] / 2, r_pixel[2] / 2, 255]),
            );
        } else {
            // Different - highlight in red/green
            // Red = only in rendered, Green = only in reference, Yellow = both but different
            let r_bright = (r_pixel[0] as u32 + r_pixel[1] as u32 + r_pixel[2] as u32) / 3;
            let ref_bright = (ref_pixel[0] as u32 + ref_pixel[1] as u32 + ref_pixel[2] as u32) / 3;

            if r_bright > ref_bright + 50 {
                // Rendered has more content - red
                diff_img.put_pixel(x, y, Rgba([255, 0, 0, 255]));
            } else if ref_bright > r_bright + 50 {
                // Reference has more content - green
                diff_img.put_pixel(x, y, Rgba([0, 255, 0, 255]));
            } else {
                // Both have content but different - yellow
                diff_img.put_pixel(x, y, Rgba([255, 255, 0, 255]));
            }
        }
    }

    diff_img.save(diff_path)?;
    Ok(())
}
