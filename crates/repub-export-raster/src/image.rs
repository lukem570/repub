//! Scene-driven raster (PNG/JPEG/…) export via [`repub_raster`].
//!
//! A page is composed into a [`Scene`](repub_scene::Scene), rasterized with
//! [`CpuRasterizer`](repub_raster::CpuRasterizer) at the requested DPI/scale,
//! and encoded to the target format with [`repub_raster::export`].

use std::path::Path;

use repub_fonts::FontProvider;
use repub_model::Document;
use repub_raster::{export_to_bytes, export_to_file, CpuRasterizer, ExportFormat, ExportOptions};
use repub_render::RenderOptions;
use repub_scene::compose;

use crate::error::{RasterExportError, RasterExportResult};

/// Options for raster image export.
#[derive(Debug, Clone)]
pub struct RasterExportOptions {
    /// Output image format.
    pub format: ExportFormat,
    /// Output resolution in DPI (72 = one device pixel per point).
    pub dpi: f64,
    /// Extra scale applied on top of DPI.
    pub scale: f64,
    /// Quality (0–100) for lossy formats such as JPEG.
    pub quality: u8,
    /// Whether to antialias edges.
    pub antialias: bool,
    /// Optional opaque background color (RGBA 0–255); `None` keeps transparency
    /// where the scene has none.
    pub background: Option<[u8; 4]>,
}

impl Default for RasterExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Png,
            dpi: 96.0,
            scale: 1.0,
            quality: 90,
            antialias: true,
            background: None,
        }
    }
}

impl RasterExportOptions {
    /// PNG options at the given DPI.
    pub fn png(dpi: f64) -> Self {
        Self {
            format: ExportFormat::Png,
            dpi,
            ..Default::default()
        }
    }

    /// JPEG options at the given DPI and quality.
    pub fn jpeg(dpi: f64, quality: u8) -> Self {
        Self {
            format: ExportFormat::Jpeg,
            dpi,
            quality,
            ..Default::default()
        }
    }

    fn render_options(&self) -> RenderOptions {
        let mut opts = RenderOptions::at_dpi(self.dpi);
        opts.scale = self.scale;
        opts.antialias = self.antialias;
        opts.background = self.background.map(|[r, g, b, a]| {
            repub_color::Color::from_rgba8(r, g, b, a)
        });
        opts
    }
}

/// Render page `page` of `doc` to encoded image bytes.
pub fn export_image_bytes(
    doc: &Document,
    page: usize,
    options: &RasterExportOptions,
    fonts: &dyn FontProvider,
) -> RasterExportResult<Vec<u8>> {
    if doc.page(page).is_none() {
        return Err(RasterExportError::PageOutOfRange(page));
    }
    let scene = compose(doc, page, fonts);
    let render_opts = options.render_options();
    let pixmap = CpuRasterizer::from_scene(&scene, &render_opts)
        .ok_or_else(|| RasterExportError::Render("could not allocate pixmap".into()))?;
    let export_opts = ExportOptions {
        format: options.format,
        quality: options.quality,
    };
    export_to_bytes(&pixmap, &export_opts).map_err(|e| RasterExportError::Encode(e.to_string()))
}

/// Render page `page` of `doc` to a PNG file.
pub fn export_png(
    doc: &Document,
    page: usize,
    path: &Path,
    options: &RasterExportOptions,
    fonts: &dyn FontProvider,
) -> RasterExportResult<()> {
    if doc.page(page).is_none() {
        return Err(RasterExportError::PageOutOfRange(page));
    }
    let scene = compose(doc, page, fonts);
    let render_opts = options.render_options();
    let pixmap = CpuRasterizer::from_scene(&scene, &render_opts)
        .ok_or_else(|| RasterExportError::Render("could not allocate pixmap".into()))?;
    let export_opts = ExportOptions {
        format: options.format,
        quality: options.quality,
    };
    export_to_file(&pixmap, path, &export_opts).map_err(|e| RasterExportError::Encode(e.to_string()))
}

/// Render page `page` of `doc` to an image file, inferring the format from
/// `options.format` (the path extension is not used to choose the format).
pub fn export_image(
    doc: &Document,
    page: usize,
    path: &Path,
    options: &RasterExportOptions,
    fonts: &dyn FontProvider,
) -> RasterExportResult<()> {
    export_png(doc, page, path, options, fonts)
}
