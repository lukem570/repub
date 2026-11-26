//! # Repub Vector Export
//!
//! Scene-driven PDF and SVG export for Repub documents.
//!
//! Both exporters are driven by [`repub_scene`]: each page is composed into a
//! flat [`Scene`](repub_scene::Scene) of resolved primitives, and those
//! primitives are translated into the target format. The exporters never walk
//! the document's shape tree directly, so geometry, WordArt, text-on-path and
//! tables are already resolved by the time they are emitted.
//!
//! - [`pdf`] writes a valid, viewable PDF. Fills and strokes become path
//!   operators; glyph runs become selectable text in a standard base-14 font
//!   (the document font is not embedded); images are drawn as placeholders;
//!   gradients are approximated by a representative color. See `docs/TODO.md`.
//! - [`svg`] writes an SVG document. Fills and strokes become `<path>`, glyph
//!   runs become `<text>`, pictures become `<image>` (or a placeholder), and
//!   gradients become `<linearGradient>`/`<radialGradient>` definitions.
//!
//! This crate has no serialization layer.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod error;
pub mod pdf;
mod scene_source;
pub mod svg;

pub use error::{VectorError, VectorResult};
pub use pdf::{export_pdf, export_pdf_bytes, export_pdf_page_bytes, PdfOptions};
pub use svg::{export_svg, export_svg_bytes, scene_to_svg, SvgOptions};

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
        let mut rect = Shape::rectangle(Rect::new(72.0, 72.0, 200.0, 100.0));
        rect.fill = Fill::Solid(Color::from_rgb8(220, 40, 40));
        page.add_shape(rect);
        let text = Shape::text_frame_with_text(Rect::new(72.0, 200.0, 400.0, 40.0), "Hello PDF");
        page.add_shape(text);
        doc
    }

    #[test]
    fn pdf_bytes_are_a_valid_pdf() {
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        let bytes = export_pdf_bytes(&doc, &PdfOptions::default(), &fonts).unwrap();
        assert!(bytes.starts_with(b"%PDF-"), "should begin with the PDF header");
        assert!(bytes.len() > 200, "a one-page PDF should be non-trivial");
        // A well-formed PDF ends with an EOF marker.
        let tail = &bytes[bytes.len().saturating_sub(16)..];
        assert!(
            tail.windows(5).any(|w| w == b"%%EOF"),
            "should end with the %%EOF marker"
        );
    }

    #[test]
    fn pdf_single_page_export_works() {
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        let bytes = export_pdf_page_bytes(&doc, 0, &PdfOptions::default(), &fonts).unwrap();
        assert!(bytes.starts_with(b"%PDF-"));
    }

    #[test]
    fn pdf_out_of_range_page_errors() {
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        assert!(matches!(
            export_pdf_page_bytes(&doc, 9, &PdfOptions::default(), &fonts),
            Err(VectorError::PageOutOfRange(9))
        ));
    }

    #[test]
    fn svg_has_root_and_a_path() {
        let doc = sample_doc();
        let fonts = SystemFontProvider::new().unwrap();
        let bytes = export_svg_bytes(&doc, 0, &SvgOptions::default(), &fonts).unwrap();
        let svg = String::from_utf8(bytes).unwrap();
        assert!(svg.contains("<svg"), "has an <svg> root");
        assert!(svg.contains("</svg>"), "closes the <svg> root");
        assert!(svg.contains("<path"), "emits at least one <path> for the rectangle");
        assert!(svg.contains("<text"), "emits <text> for the text frame");
    }

    #[test]
    fn svg_gradient_emits_definition() {
        let mut doc = sample_doc();
        let grad = repub_color::LinearGradient::horizontal(Color::RED, Color::BLUE);
        let mut shape = Shape::rectangle(Rect::new(300.0, 300.0, 100.0, 100.0));
        shape.fill = Fill::LinearGradient(grad);
        doc.page_mut(0).unwrap().add_shape(shape);
        let fonts = SystemFontProvider::new().unwrap();
        let bytes = export_svg_bytes(&doc, 0, &SvgOptions::default(), &fonts).unwrap();
        let svg = String::from_utf8(bytes).unwrap();
        assert!(svg.contains("<linearGradient"), "gradient fill becomes a def");
    }
}
