use repub_export_raster::{export_png, RasterExportOptions};
use repub_fonts::SystemFontProvider;
use repub_model::shape::{Shape, ShapeKind};
use repub_model::styles::{CharacterStyle, LineSpacing};
use repub_model::text::Paragraph;
use repub_model::{Document, PageOrientation, PageSize, Rect};

fn main() {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let page = doc.pages_mut().get_mut(0).unwrap();

    // Text frame 1: Single line spacing
    let mut tf1 = Shape::text_frame(Rect::new(10.0, 10.0, 180.0, 100.0));
    if let ShapeKind::TextFrame(data) = &mut tf1.kind {
        data.inset = 0.0; // No inset to see exact positioning
        let mut para1 = Paragraph::styled("Single spacing", CharacterStyle::default().with_size(24.0));
        para1.style.line_spacing = LineSpacing::Single;
        data.content.paragraphs.push(para1);
    }
    page.add_shape(tf1);

    // Text frame 2: Double line spacing
    let mut tf2 = Shape::text_frame(Rect::new(200.0, 10.0, 180.0, 100.0));
    if let ShapeKind::TextFrame(data) = &mut tf2.kind {
        data.inset = 0.0;
        let mut para2 = Paragraph::styled("Double spacing", CharacterStyle::default().with_size(24.0));
        para2.style.line_spacing = LineSpacing::Multiple(2.0);
        data.content.paragraphs.push(para2);
    }
    page.add_shape(tf2);

    // Compose and rasterize the first page to a PNG.
    let fonts = SystemFontProvider::new().unwrap();
    let options = RasterExportOptions::png(72.0);
    export_png(&doc, 0, std::path::Path::new("/tmp/line_spacing_test.png"), &options, &fonts).unwrap();
    println!("Saved to /tmp/line_spacing_test.png");
}
