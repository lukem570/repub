//! # Repub Format
//!
//! The thin umbrella (facade) over the Layer-5 format crates. It dispatches
//! load and save requests by file extension to the right backend:
//!
//! | Extension        | Direction | Backend                          |
//! |------------------|-----------|----------------------------------|
//! | `.repub`         | load/save | [`repub_format_native`] (JSON)   |
//! | `.repubz`        | load/save | [`repub_format_native`] (zipped) |
//! | `.json`          | load/save | [`repub_format_native`] (JSON)   |
//! | `.pdf`           | save      | [`repub_export_vector`]          |
//! | `.svg`           | save      | [`repub_export_vector`]          |
//! | `.png` / `.jpg` / `.jpeg` / `.tiff` / `.bmp` / `.webp` | save | [`repub_export_raster`] |
//! | `.html` / `.htm` | save      | [`repub_export_raster`]          |
//! | `.pub`           | load/save | [`repub_format_pub`] (Publisher) |
//!
//! Loading supports the native formats and `.pub`; the other targets are
//! write-only exports. Attempting to load an export-only format returns a
//! clear error. Note that `.pub` export is still incomplete (see that crate's
//! docs), though it dispatches here.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod error;

use std::path::Path;

use repub_fonts::FontProvider;
use repub_model::Document;

pub use error::{FormatError, FormatResult};

// Re-export the common entry points and option types from each backend, so a
// caller can reach everything through this one crate.
pub use repub_export_raster::{
    export_html, export_html_string, export_image, export_image_bytes,
    export_png as export_png_page, ExportFormat, HtmlOptions, RasterExportOptions,
};
pub use repub_export_vector::{
    export_pdf, export_pdf_bytes, export_svg, export_svg_bytes, PdfOptions, SvgOptions,
};
pub use repub_format_native::{SaveOptions, EXTENSION, EXTENSION_COMPRESSED, FORMAT_VERSION};

/// The lower-cased extension of `path`, or `None` if it has none.
fn extension(path: &Path) -> Option<String> {
    path.extension().map(|e| e.to_string_lossy().to_lowercase())
}

/// Load a document from `path`, dispatching by extension.
///
/// The native formats (`.repub`, `.repubz`, `.json`) and `.pub` can be loaded;
/// any other extension returns [`FormatError::UnsupportedExtension`].
pub fn load(path: &Path) -> FormatResult<Document> {
    let ext = extension(path).ok_or_else(|| FormatError::MissingExtension(path.to_path_buf()))?;
    match ext.as_str() {
        "repub" | "repubz" | "json" => Ok(repub_format_native::load(path)?),
        "pub" => Ok(repub_format_pub::import(path)?),
        _ => Err(FormatError::UnsupportedExtension(path.to_path_buf())),
    }
}

/// Save a document to `path`, dispatching by extension.
///
/// `fonts` is used by the Scene-driven exporters (PDF, SVG, PNG/JPEG) to
/// resolve and measure fonts; native and HTML saves ignore it.
pub fn save(doc: &Document, path: &Path, fonts: &dyn FontProvider) -> FormatResult<()> {
    let ext = extension(path).ok_or_else(|| FormatError::MissingExtension(path.to_path_buf()))?;
    match ext.as_str() {
        "repub" => {
            repub_format_native::save(doc, path, &SaveOptions::default())?;
            Ok(())
        }
        "repubz" => {
            repub_format_native::save(doc, path, &SaveOptions::compressed())?;
            Ok(())
        }
        "json" => {
            repub_format_native::save(doc, path, &SaveOptions::compact())?;
            Ok(())
        }
        "pdf" => {
            repub_export_vector::export_pdf(doc, path, &PdfOptions::default(), fonts)?;
            Ok(())
        }
        "svg" => {
            repub_export_vector::export_svg(doc, 0, path, &SvgOptions::default(), fonts)?;
            Ok(())
        }
        "png" | "jpg" | "jpeg" | "tiff" | "tif" | "bmp" | "webp" => {
            let options = raster_options_for(&ext);
            repub_export_raster::export_png(doc, 0, path, &options, fonts)?;
            Ok(())
        }
        "html" | "htm" => {
            repub_export_raster::export_html(doc, path, &HtmlOptions::default())?;
            Ok(())
        }
        "pub" => {
            repub_format_pub::export(doc, path)?;
            Ok(())
        }
        _ => Err(FormatError::UnsupportedExtension(path.to_path_buf())),
    }
}

/// Load a document from the contents of a file called `name`, dispatching by its extension.
///
/// This is [`load`] without a file system, for callers (a browser) that already hold the bytes.
pub fn load_bytes(name: &str, bytes: &[u8]) -> FormatResult<Document> {
    let path = Path::new(name);
    let ext = extension(path).ok_or_else(|| FormatError::MissingExtension(path.to_path_buf()))?;
    match ext.as_str() {
        "repub" | "json" => Ok(repub_format_native::load_bytes(bytes, false)?),
        "repubz" => Ok(repub_format_native::load_bytes(bytes, true)?),
        "pub" => Ok(repub_format_pub::import_bytes(bytes)?),
        _ => Err(FormatError::UnsupportedExtension(path.to_path_buf())),
    }
}

/// Encode a document in the format named by the extension of `name` (page 1 for the single-page
/// formats: SVG and images), like [`save`] without a file system.
pub fn save_bytes(doc: &Document, name: &str, fonts: &dyn FontProvider) -> FormatResult<Vec<u8>> {
    let path = Path::new(name);
    let ext = extension(path).ok_or_else(|| FormatError::MissingExtension(path.to_path_buf()))?;
    Ok(match ext.as_str() {
        "repub" => repub_format_native::save_bytes(doc, &SaveOptions::default())?,
        "repubz" => repub_format_native::save_bytes(doc, &SaveOptions::compressed())?,
        "json" => repub_format_native::save_bytes(doc, &SaveOptions::compact())?,
        "pdf" => repub_export_vector::export_pdf_bytes(doc, &PdfOptions::default(), fonts)?,
        "svg" => repub_export_vector::export_svg_bytes(doc, 0, &SvgOptions::default(), fonts)?,
        "png" | "jpg" | "jpeg" | "tiff" | "tif" | "bmp" | "webp" => {
            repub_export_raster::export_image_bytes(doc, 0, &raster_options_for(&ext), fonts)?
        }
        "html" | "htm" => {
            repub_export_raster::export_html_string(doc, &HtmlOptions::default()).into_bytes()
        }
        "pub" => repub_format_pub::export_bytes(doc)?,
        _ => return Err(FormatError::UnsupportedExtension(path.to_path_buf())),
    })
}

/// Default raster options for a given image extension (chooses the format).
fn raster_options_for(ext: &str) -> RasterExportOptions {
    let format = match ext {
        "jpg" | "jpeg" => ExportFormat::Jpeg,
        "tiff" | "tif" => ExportFormat::Tiff,
        "bmp" => ExportFormat::Bmp,
        "webp" => ExportFormat::WebP,
        _ => ExportFormat::Png,
    };
    RasterExportOptions {
        format,
        ..RasterExportOptions::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_color::Color;
    use repub_fonts::SystemFontProvider;
    use repub_geometry::Rect;
    use repub_model::shape::Shape;
    use repub_model::styles::Fill;
    use repub_model::{Document, PageOrientation, PageSize};

    fn sample_doc() -> Document {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        let page = doc.page_mut(0).unwrap();
        let mut rect = Shape::rectangle(Rect::new(72.0, 72.0, 120.0, 80.0));
        rect.fill = Fill::Solid(Color::from_rgb8(10, 180, 90));
        page.add_shape(rect);
        page.add_shape(Shape::text_frame_with_text(
            Rect::new(72.0, 180.0, 300.0, 40.0),
            "Facade",
        ));
        doc
    }

    #[test]
    fn bytes_round_trip_without_a_file_system() {
        let fonts = SystemFontProvider::new().unwrap();
        let doc = sample_doc();
        for name in ["doc.repub", "doc.repubz", "doc.json"] {
            let bytes = save_bytes(&doc, name, &fonts).unwrap();
            let back = load_bytes(name, &bytes).unwrap();
            assert_eq!(back.pages().len(), doc.pages().len(), "{name}");
            assert_eq!(
                back.pages()[0].all_shapes().count(),
                doc.pages()[0].all_shapes().count(),
                "{name}"
            );
        }
        assert!(save_bytes(&doc, "doc.pdf", &fonts)
            .unwrap()
            .starts_with(b"%PDF"));
        assert!(save_bytes(&doc, "doc.png", &fonts)
            .unwrap()
            .starts_with(&[0x89, b'P', b'N', b'G']));
        assert!(matches!(
            load_bytes("doc.xyz", b""),
            Err(FormatError::UnsupportedExtension(_))
        ));
        assert!(matches!(
            save_bytes(&doc, "noext", &fonts),
            Err(FormatError::MissingExtension(_))
        ));
    }

    #[test]
    fn native_round_trip_through_facade() {
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        let path = std::env::temp_dir().join("repub_facade_test.repub");
        save(&doc, &path, &fonts).unwrap();
        let restored = load(&path).unwrap();
        assert_eq!(doc, restored);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn compressed_native_round_trip_through_facade() {
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        let path = std::env::temp_dir().join("repub_facade_test.repubz");
        save(&doc, &path, &fonts).unwrap();
        let restored = load(&path).unwrap();
        assert_eq!(doc.total_shape_count(), restored.total_shape_count());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn export_to_pdf_svg_png_produce_non_empty_files() {
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        let dir = std::env::temp_dir();
        for name in ["repub_facade.pdf", "repub_facade.svg", "repub_facade.png"] {
            let path = dir.join(name);
            save(&doc, &path, &fonts).unwrap();
            let data = std::fs::read(&path).unwrap();
            assert!(!data.is_empty(), "{name} should be non-empty");
            let _ = std::fs::remove_file(&path);
        }
    }

    #[test]
    fn export_to_html_produces_markup() {
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        let path = std::env::temp_dir().join("repub_facade.html");
        save(&doc, &path, &fonts).unwrap();
        let data = std::fs::read_to_string(&path).unwrap();
        assert!(data.contains("<html"));
        assert!(data.contains("Facade"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn unknown_extension_errors() {
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        let path = std::env::temp_dir().join("repub_facade.xyz");
        assert!(matches!(
            save(&doc, &path, &fonts),
            Err(FormatError::UnsupportedExtension(_))
        ));
        assert!(matches!(
            load(&path),
            Err(FormatError::UnsupportedExtension(_))
        ));
    }

    #[test]
    fn pub_dispatches_through_facade() {
        // The facade dispatches .pub to repub-format-pub. Export is still a
        // skeleton (see that crate's docs), but save must succeed and the file
        // must load back into a Document through the facade.
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        let path = std::env::temp_dir().join("repub_facade.pub");
        save(&doc, &path, &fonts).expect("pub save dispatches");
        let loaded = load(&path).expect("pub load dispatches");
        assert!(!loaded.pages().is_empty());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn missing_extension_errors() {
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        let path = std::env::temp_dir().join("repub_facade_noext");
        assert!(matches!(
            save(&doc, &path, &fonts),
            Err(FormatError::MissingExtension(_))
        ));
    }
}
