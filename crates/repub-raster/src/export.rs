//! Export a rendered pixmap to common image formats.

use std::io::Write;
use std::path::Path;

use image::{ImageFormat, RgbaImage};
use tiny_skia::Pixmap;

use crate::error::{RasterError, RasterResult};

/// An output image format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExportFormat {
    /// PNG (lossless).
    #[default]
    Png,
    /// JPEG (lossy).
    Jpeg,
    /// TIFF.
    Tiff,
    /// WebP.
    WebP,
    /// BMP.
    Bmp,
}

impl ExportFormat {
    /// The conventional file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Png => "png",
            ExportFormat::Jpeg => "jpg",
            ExportFormat::Tiff => "tiff",
            ExportFormat::WebP => "webp",
            ExportFormat::Bmp => "bmp",
        }
    }

    /// The MIME type for this format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            ExportFormat::Png => "image/png",
            ExportFormat::Jpeg => "image/jpeg",
            ExportFormat::Tiff => "image/tiff",
            ExportFormat::WebP => "image/webp",
            ExportFormat::Bmp => "image/bmp",
        }
    }

    fn to_image_format(self) -> ImageFormat {
        match self {
            ExportFormat::Png => ImageFormat::Png,
            ExportFormat::Jpeg => ImageFormat::Jpeg,
            ExportFormat::Tiff => ImageFormat::Tiff,
            ExportFormat::WebP => ImageFormat::WebP,
            ExportFormat::Bmp => ImageFormat::Bmp,
        }
    }
}

/// Options controlling how a pixmap is exported.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    /// The output format.
    pub format: ExportFormat,
    /// Quality (0–100) for lossy formats.
    pub quality: u8,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            format: ExportFormat::Png,
            quality: 90,
        }
    }
}

impl ExportOptions {
    /// PNG export options.
    pub fn png() -> Self {
        Self {
            format: ExportFormat::Png,
            ..Default::default()
        }
    }

    /// JPEG export options at the given quality.
    pub fn jpeg(quality: u8) -> Self {
        Self {
            format: ExportFormat::Jpeg,
            quality,
        }
    }

    /// TIFF export options.
    pub fn tiff() -> Self {
        Self {
            format: ExportFormat::Tiff,
            ..Default::default()
        }
    }

    /// WebP export options at the given quality.
    pub fn webp(quality: u8) -> Self {
        Self {
            format: ExportFormat::WebP,
            quality,
        }
    }
}

/// Export a pixmap to a file, inferring encoding from `options.format`.
pub fn export_to_file<P: AsRef<Path>>(
    pixmap: &Pixmap,
    path: P,
    options: &ExportOptions,
) -> RasterResult<()> {
    let image = pixmap_to_rgba_image(pixmap)?;
    image
        .save_with_format(path.as_ref(), options.format.to_image_format())
        .map_err(|e| RasterError::Export(e.to_string()))
}

/// Export a pixmap to an in-memory byte buffer.
pub fn export_to_bytes(pixmap: &Pixmap, options: &ExportOptions) -> RasterResult<Vec<u8>> {
    let image = pixmap_to_rgba_image(pixmap)?;
    let mut buffer = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut buffer);
    image
        .write_to(&mut cursor, options.format.to_image_format())
        .map_err(|e| RasterError::Export(e.to_string()))?;
    Ok(buffer)
}

/// Export a pixmap to PNG bytes using tiny-skia's native encoder.
pub fn export_png(pixmap: &Pixmap) -> RasterResult<Vec<u8>> {
    pixmap
        .encode_png()
        .map_err(|e| RasterError::Export(e.to_string()))
}

/// Export a pixmap to a seekable writer.
pub fn export_to_writer<W: Write + std::io::Seek>(
    pixmap: &Pixmap,
    writer: &mut W,
    options: &ExportOptions,
) -> RasterResult<()> {
    let image = pixmap_to_rgba_image(pixmap)?;
    image
        .write_to(writer, options.format.to_image_format())
        .map_err(|e| RasterError::Export(e.to_string()))
}

/// Convert a premultiplied tiny-skia pixmap to a straight-alpha `RgbaImage`.
fn pixmap_to_rgba_image(pixmap: &Pixmap) -> RasterResult<RgbaImage> {
    let width = pixmap.width();
    let height = pixmap.height();
    let data = pixmap.data();
    let mut rgba = Vec::with_capacity(data.len());
    for chunk in data.chunks(4) {
        let (r, g, b, a) = (chunk[0], chunk[1], chunk[2], chunk[3]);
        match a {
            0 => rgba.extend_from_slice(&[0, 0, 0, 0]),
            255 => rgba.extend_from_slice(&[r, g, b, a]),
            _ => {
                let af = 255.0 / a as f32;
                rgba.push((r as f32 * af).min(255.0) as u8);
                rgba.push((g as f32 * af).min(255.0) as u8);
                rgba.push((b as f32 * af).min(255.0) as u8);
                rgba.push(a);
            }
        }
    }
    RgbaImage::from_raw(width, height, rgba)
        .ok_or_else(|| RasterError::Export("invalid image dimensions".into()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_bytes_have_png_header() {
        let mut pixmap = Pixmap::new(8, 8).unwrap();
        pixmap.fill(tiny_skia::Color::WHITE);
        let bytes = export_to_bytes(&pixmap, &ExportOptions::png()).unwrap();
        assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn export_png_is_nonempty() {
        let mut pixmap = Pixmap::new(4, 4).unwrap();
        pixmap.fill(tiny_skia::Color::BLACK);
        assert!(!export_png(&pixmap).unwrap().is_empty());
    }

    #[test]
    fn format_extensions() {
        assert_eq!(ExportFormat::Png.extension(), "png");
        assert_eq!(ExportFormat::Jpeg.mime_type(), "image/jpeg");
    }
}
