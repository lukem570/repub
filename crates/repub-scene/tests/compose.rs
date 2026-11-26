//! Integration tests for composing documents into scenes.

use std::sync::Arc;

use repub_color::{Color, ColorScheme, SchemeColor, SchemeRef};
use repub_fonts::SystemFontProvider;
use repub_geometry::{Point, Rect};
use repub_model::shape::{
    LineData, LineEnding, Shape, ShapeKind, TextEffectData, WordArtPreset,
};
use repub_model::styles::{Fill, Stroke};
use repub_model::{Document, PageOrientation, PageSize};
use repub_scene::{compose, compose_with_scheme, FillRule, Paint, Primitive};

fn provider() -> Arc<SystemFontProvider> {
    Arc::new(SystemFontProvider::default())
}

fn count<F: Fn(&Primitive) -> bool>(scene: &repub_scene::Scene, pred: F) -> usize {
    scene.primitives.iter().filter(|p| pred(p)).count()
}

#[test]
fn rectangle_emits_fill_and_stroke() {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let mut rect = Shape::rectangle(Rect::new(100.0, 100.0, 200.0, 150.0));
    rect.fill = Fill::Solid(Color::RED);
    rect.line = Some(Stroke::solid(Color::BLACK, 2.0));
    doc.page_mut(0).unwrap().add_shape(rect);

    let fonts = provider();
    let scene = compose(&doc, 0, fonts.as_ref());

    let fills = count(&scene, |p| matches!(p, Primitive::FillPath { .. }));
    let strokes = count(&scene, |p| matches!(p, Primitive::StrokePath { .. }));
    assert_eq!(fills, 1, "one fill for the rectangle");
    assert_eq!(strokes, 1, "one stroke for the border");

    // The fill paint should be the concrete red color.
    let fill = scene
        .primitives
        .iter()
        .find_map(|p| match p {
            Primitive::FillPath { paint, fill_rule, .. } => Some((paint, *fill_rule)),
            _ => None,
        })
        .unwrap();
    assert_eq!(*fill.0, Paint::Solid(Color::RED));
    assert_eq!(fill.1, FillRule::NonZero);
}

#[test]
fn unfilled_rectangle_emits_only_stroke() {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let mut rect = Shape::rectangle(Rect::new(10.0, 10.0, 50.0, 50.0));
    rect.fill = Fill::None;
    rect.line = Some(Stroke::solid(Color::BLACK, 1.0));
    doc.page_mut(0).unwrap().add_shape(rect);

    let fonts = provider();
    let scene = compose(&doc, 0, fonts.as_ref());
    assert_eq!(count(&scene, |p| matches!(p, Primitive::FillPath { .. })), 0);
    assert_eq!(count(&scene, |p| matches!(p, Primitive::StrokePath { .. })), 1);
}

#[test]
fn text_frame_emits_glyph_runs() {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let frame = Shape::text_frame_with_text(Rect::new(72.0, 72.0, 300.0, 100.0), "Hello World");
    doc.page_mut(0).unwrap().add_shape(frame);

    let fonts = provider();
    let scene = compose(&doc, 0, fonts.as_ref());
    let glyph_runs = count(&scene, |p| matches!(p, Primitive::Glyphs { .. }));
    assert!(glyph_runs >= 1, "expected at least one glyph run, got {glyph_runs}");
}

#[test]
fn line_emits_stroke() {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let line = Shape::new(
        Rect::new(0.0, 0.0, 100.0, 0.0),
        ShapeKind::Line(LineData::new(Point::new(10.0, 10.0), Point::new(110.0, 10.0))),
    );
    doc.page_mut(0).unwrap().add_shape(line);

    let fonts = provider();
    let scene = compose(&doc, 0, fonts.as_ref());
    assert!(count(&scene, |p| matches!(p, Primitive::StrokePath { .. })) >= 1);
}

#[test]
fn line_with_arrow_emits_fill() {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let mut data = LineData::new(Point::new(10.0, 10.0), Point::new(110.0, 10.0));
    data.end_ending = LineEnding::arrow();
    let mut line = Shape::new(Rect::new(0.0, 0.0, 100.0, 0.0), ShapeKind::Line(data));
    line.line = Some(Stroke::solid(Color::BLACK, 2.0));
    doc.page_mut(0).unwrap().add_shape(line);

    let fonts = provider();
    let scene = compose(&doc, 0, fonts.as_ref());
    assert!(
        count(&scene, |p| matches!(p, Primitive::FillPath { .. })) >= 1,
        "arrowhead should be a filled path"
    );
}

#[test]
fn picture_emits_image() {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let pic = Shape::new(
        Rect::new(0.0, 0.0, 100.0, 100.0),
        ShapeKind::Picture(repub_model::shape::PictureData::from_file("photo.png")),
    );
    doc.page_mut(0).unwrap().add_shape(pic);

    let fonts = provider();
    let scene = compose(&doc, 0, fonts.as_ref());
    assert_eq!(count(&scene, |p| matches!(p, Primitive::Image { .. })), 1);
}

#[test]
fn wordart_resolves_to_glyph_fill_paths() {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let mut data = TextEffectData::with_preset("WAVE", WordArtPreset::Wave);
    data.style.font.family = "DejaVu Sans".to_string();
    let mut shape = Shape::new(Rect::new(50.0, 50.0, 300.0, 120.0), ShapeKind::TextEffect(data));
    shape.fill = Fill::Solid(Color::BLUE);
    doc.page_mut(0).unwrap().add_shape(shape);

    let fonts = provider();
    let scene = compose(&doc, 0, fonts.as_ref());
    let fills = count(&scene, |p| matches!(p, Primitive::FillPath { .. }));
    assert!(
        fills >= 1,
        "WordArt should resolve to at least one glyph fill path, got {fills}"
    );
    // Every WordArt fill carries the concrete fill color.
    for prim in &scene.primitives {
        if let Primitive::FillPath { paint, .. } = prim {
            assert_eq!(*paint, Paint::Solid(Color::BLUE));
        }
    }
}

#[test]
fn wordart_3d_emits_extra_fill_layers() {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let mut flat = TextEffectData::with_preset("A", WordArtPreset::Plain);
    flat.style.font.family = "DejaVu Sans".to_string();
    let mut flat_shape = Shape::new(Rect::new(0.0, 0.0, 200.0, 120.0), ShapeKind::TextEffect(flat));
    flat_shape.fill = Fill::Solid(Color::BLUE);

    let mut deep = TextEffectData::with_preset("A", WordArtPreset::Plain);
    deep.style.font.family = "DejaVu Sans".to_string();
    deep = deep.with_extrusion(24.0);
    let mut deep_shape = Shape::new(Rect::new(0.0, 0.0, 200.0, 120.0), ShapeKind::TextEffect(deep));
    deep_shape.fill = Fill::Solid(Color::BLUE);

    doc.page_mut(0).unwrap().add_shape(flat_shape);
    let mut doc2 = Document::new(PageSize::Letter, PageOrientation::Portrait);
    doc2.page_mut(0).unwrap().add_shape(deep_shape);

    let fonts = provider();
    let flat_scene = compose(&doc, 0, fonts.as_ref());
    let deep_scene = compose(&doc2, 0, fonts.as_ref());

    let flat_fills = count(&flat_scene, |p| matches!(p, Primitive::FillPath { .. }));
    let deep_fills = count(&deep_scene, |p| matches!(p, Primitive::FillPath { .. }));
    assert!(
        deep_fills > flat_fills,
        "3D extrusion should add fill layers: flat={flat_fills} deep={deep_fills}"
    );
}

#[test]
fn scheme_color_resolves_to_concrete_color() {
    // A custom scheme whose Accent1 role is a known color.
    let mut scheme = ColorScheme::OFFICE;
    scheme.set(SchemeColor::Accent1, Color::rgb(0.1, 0.2, 0.3));

    // Resolve a scheme reference into a concrete color and use it as a fill.
    let resolved = SchemeRef::new(SchemeColor::Accent1).resolve(&scheme);
    assert_eq!(resolved, Color::rgb(0.1, 0.2, 0.3));

    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let mut rect = Shape::rectangle(Rect::new(0.0, 0.0, 50.0, 50.0));
    rect.fill = Fill::Solid(resolved);
    doc.page_mut(0).unwrap().add_shape(rect);

    let fonts = provider();
    let scene = compose_with_scheme(&doc, 0, fonts.as_ref(), &scheme);
    let paint = scene
        .primitives
        .iter()
        .find_map(|p| match p {
            Primitive::FillPath { paint, .. } => Some(paint.clone()),
            _ => None,
        })
        .unwrap();
    assert_eq!(paint, Paint::Solid(Color::rgb(0.1, 0.2, 0.3)));
}

#[test]
fn group_recurses_into_children() {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let mut a = Shape::rectangle(Rect::new(0.0, 0.0, 10.0, 10.0));
    a.fill = Fill::Solid(Color::RED);
    let mut b = Shape::rectangle(Rect::new(20.0, 20.0, 10.0, 10.0));
    b.fill = Fill::Solid(Color::GREEN);
    let group = Shape::group_from(vec![a, b]);
    doc.page_mut(0).unwrap().add_shape(group);

    let fonts = provider();
    let scene = compose(&doc, 0, fonts.as_ref());
    // Two child rectangles, each with a default fill.
    assert!(count(&scene, |p| matches!(p, Primitive::FillPath { .. })) >= 2);
}

#[test]
fn table_emits_cell_fills_and_borders() {
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let mut table_data = repub_model::shape::TableData::new(Rect::new(0.0, 0.0, 200.0, 100.0), 2, 2);
    table_data
        .cell_mut(0, 0)
        .unwrap()
        .set_text("A");
    table_data.cells[0].fill = Fill::Solid(Color::YELLOW);
    let table = Shape::new(Rect::new(10.0, 10.0, 200.0, 100.0), ShapeKind::Table(table_data));
    doc.page_mut(0).unwrap().add_shape(table);

    let fonts = provider();
    let scene = compose(&doc, 0, fonts.as_ref());
    // At least one cell fill and several border strokes.
    assert!(count(&scene, |p| matches!(p, Primitive::FillPath { .. })) >= 1);
    assert!(count(&scene, |p| matches!(p, Primitive::StrokePath { .. })) >= 4);
}

#[test]
fn out_of_range_page_is_empty() {
    let doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
    let fonts = provider();
    let scene = compose(&doc, 99, fonts.as_ref());
    assert!(scene.is_empty());
}
