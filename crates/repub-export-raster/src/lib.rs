//! # Repub Raster & HTML Export
//!
//! PNG/JPEG (and other raster) export, plus HTML export, for Repub documents.
//!
//! - [`image`] composes a page into a [`Scene`](repub_scene::Scene), rasterizes
//!   it with [`repub_raster`]'s CPU rasterizer at a chosen DPI/scale, and
//!   encodes the pixmap to the target format. This is the one place in the
//!   Layer-5 format crates that pulls in a renderer.
//! - [`html`] emits a self-contained HTML page per document, with
//!   absolutely-positioned elements for shapes and live text for text frames.
//!   It works from the model (not the scene) to keep text semantic.
//!
//! This crate has no serialization layer.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod error;
pub mod html;
pub mod image;

pub use error::{RasterExportError, RasterExportResult};
pub use html::{export_html, export_html_string, HtmlOptions};
pub use image::{export_image, export_image_bytes, export_png, RasterExportOptions};

// Re-export the raster export format enum for callers choosing a format.
pub use repub_raster::ExportFormat;

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
        doc.metadata.title = Some("Raster Test".to_string());
        let page = doc.page_mut(0).unwrap();
        let mut rect = Shape::rectangle(Rect::new(50.0, 50.0, 120.0, 80.0));
        rect.fill = Fill::Solid(Color::from_rgb8(30, 120, 220));
        page.add_shape(rect);
        page.add_shape(Shape::text_frame_with_text(
            Rect::new(50.0, 160.0, 300.0, 40.0),
            "Hello HTML",
        ));
        doc
    }

    #[test]
    fn png_bytes_have_valid_header_and_size() {
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        let opts = RasterExportOptions::png(72.0);
        let bytes = export_image_bytes(&doc, 0, &opts, &fonts).unwrap();
        // PNG magic.
        assert_eq!(&bytes[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        assert!(bytes.len() > 100);
    }

    #[test]
    fn png_size_matches_page_at_dpi() {
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        // At 72 DPI a Letter page (612x792 pt) should rasterize to 612x792 px.
        let opts = RasterExportOptions::png(72.0);
        let bytes = export_image_bytes(&doc, 0, &opts, &fonts).unwrap();
        // Parse width/height from the PNG IHDR (bytes 16..24, big-endian).
        let w = u32::from_be_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
        let h = u32::from_be_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
        assert_eq!((w, h), (612, 792));
    }

    #[test]
    fn png_file_export_writes_a_file() {
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        let path = std::env::temp_dir().join("repub_raster_test.png");
        export_png(&doc, 0, &path, &RasterExportOptions::png(72.0), &fonts).unwrap();
        let data = std::fs::read(&path).unwrap();
        assert_eq!(&data[0..8], &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn png_out_of_range_page_errors() {
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        assert!(matches!(
            export_image_bytes(&doc, 5, &RasterExportOptions::png(72.0), &fonts),
            Err(RasterExportError::PageOutOfRange(5))
        ));
    }

    #[test]
    fn html_has_root_and_positioned_element() {
        let doc = sample_doc();
        let html = export_html_string(&doc, &HtmlOptions::default());
        assert!(html.contains("<html"), "has an <html> root");
        assert!(html.contains("</html>"), "closes <html>");
        assert!(html.contains("class=\"page\""), "has a page container");
        assert!(html.contains("position:absolute"), "shapes are positioned");
        assert!(html.contains("Hello HTML"), "keeps live text content");
    }
}
