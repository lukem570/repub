//! Benchmarks for document operations.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use repub_model::shape::Shape;
use repub_model::{Document, PageOrientation, PageSize, Rect};

fn create_document(c: &mut Criterion) {
    c.bench_function("create_document", |b| {
        b.iter(|| {
            Document::new(
                black_box(PageSize::Letter),
                black_box(PageOrientation::Portrait),
            )
        })
    });
}

fn add_shapes(c: &mut Criterion) {
    c.bench_function("add_100_shapes", |b| {
        b.iter(|| {
            let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
            let page = doc.page_mut(0).unwrap();
            for i in 0..100 {
                let x = (i % 10) as f64 * 50.0;
                let y = (i / 10) as f64 * 50.0;
                page.add_shape(Shape::rectangle(Rect::new(x, y, 40.0, 40.0)));
            }
            black_box(doc)
        })
    });
}

criterion_group!(benches, create_document, add_shapes);
criterion_main!(benches);
