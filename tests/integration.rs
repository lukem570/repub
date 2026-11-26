//! Integration tests for Repub.

use repub_model::shape::Shape;
use repub_model::{Document, PageOrientation, PageSize, Rect};

#[test]
fn test_create_document_with_objects() {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);

    // Add a text frame
    let text_frame =
        Shape::text_frame_with_text(Rect::new(72.0, 72.0, 468.0, 100.0), "Hello, Repub!");
    doc.page_mut(0).unwrap().add_shape(text_frame);

    // Add a shape
    let shape = Shape::rectangle(Rect::new(72.0, 200.0, 200.0, 150.0));
    doc.page_mut(0).unwrap().add_shape(shape);

    assert_eq!(doc.page(0).unwrap().shape_count(), 2);
}

#[test]
fn test_multi_page_document() {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    doc.add_page();
    doc.add_page();

    // Add shapes to different pages
    doc.page_mut(0)
        .unwrap()
        .add_shape(Shape::rectangle(Rect::new(0.0, 0.0, 100.0, 100.0)));
    doc.page_mut(1)
        .unwrap()
        .add_shape(Shape::ellipse(Rect::new(0.0, 0.0, 100.0, 100.0)));
    doc.page_mut(2)
        .unwrap()
        .add_shape(Shape::triangle(Rect::new(0.0, 0.0, 100.0, 100.0)));

    assert_eq!(doc.page_count(), 3);
    assert_eq!(doc.total_shape_count(), 3);
}

#[test]
fn test_layer_operations() {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let page = doc.page_mut(0).unwrap();

    // Add a second layer
    let layer_id = page.add_layer("Background");

    // Add shapes to different layers
    page.add_shape(Shape::rectangle(Rect::new(0.0, 0.0, 100.0, 100.0)));
    page.add_shape_to_layer(
        layer_id,
        Shape::ellipse(Rect::new(50.0, 50.0, 100.0, 100.0)),
    );

    assert_eq!(page.layers().len(), 2);
    assert_eq!(page.shape_count(), 2);
    assert_eq!(page.default_layer().shape_count(), 1);
    assert_eq!(page.layer(layer_id).unwrap().shape_count(), 1);
}
