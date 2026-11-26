//! Scene-driven PDF export.
//!
//! Each page of the document is composed into a [`Scene`](repub_scene::Scene)
//! and its primitives are translated into PDF content-stream operators:
//!
//! - [`Primitive::FillPath`](repub_scene::Primitive::FillPath) and
//!   [`Primitive::StrokePath`](repub_scene::Primitive::StrokePath) become path
//!   construction (`m`/`l`/`c`) followed by `f`/`f*`/`S`, with the paint's
//!   color. Gradients are approximated by their first stop color (see TODO),
//!   since broad PDF shading support is not yet wired.
//! - [`Primitive::Glyphs`](repub_scene::Primitive::Glyphs) becomes PDF text:
//!   each run is shown with a standard base-14 font (Helvetica family, chosen
//!   by weight/italic). The run is positioned by a text matrix. This produces
//!   selectable, viewable text; the document's actual font is **not** embedded
//!   (documented simplification — see TODO).
//! - [`Primitive::Image`](repub_scene::Primitive::Image) is drawn as a gray
//!   placeholder rectangle. Real image XObject embedding is a TODO.
//!
//! The result is a valid, viewable single- or multi-page PDF. A simple page
//! (filled rectangle + text) renders correctly in standard PDF viewers.

use std::path::Path;

use pdf_writer::{Content, Finish, Name, Pdf, Rect as PdfRect, Ref, Str, TextStr};
use repub_geometry::{Path as GeomPath, PathSegment, Point, Transform2D};
use repub_scene::{FillRule, Paint, Primitive, Scene};

use crate::error::{VectorError, VectorResult};
use crate::scene_source::compose_pages;
use repub_fonts::FontProvider;
use repub_model::Document;

/// Options for PDF export.
#[derive(Debug, Clone, Default)]
pub struct PdfOptions {
    /// Document title.
    pub title: Option<String>,
    /// Document author.
    pub author: Option<String>,
    /// Document subject.
    pub subject: Option<String>,
    /// Document keywords.
    pub keywords: Vec<String>,
}

/// Export the whole document to PDF bytes (one PDF page per document page).
pub fn export_pdf_bytes(
    doc: &Document,
    options: &PdfOptions,
    fonts: &dyn FontProvider,
) -> VectorResult<Vec<u8>> {
    let indices: Vec<usize> = (0..doc.page_count()).collect();
    let scenes = compose_pages(doc, &indices, fonts);
    write_pdf(&scenes, options)
}

/// Export a single page of the document to PDF bytes.
pub fn export_pdf_page_bytes(
    doc: &Document,
    page: usize,
    options: &PdfOptions,
    fonts: &dyn FontProvider,
) -> VectorResult<Vec<u8>> {
    if doc.page(page).is_none() {
        return Err(VectorError::PageOutOfRange(page));
    }
    let scenes = compose_pages(doc, &[page], fonts);
    write_pdf(&scenes, options)
}

/// Export the whole document to a PDF file.
pub fn export_pdf(
    doc: &Document,
    path: &Path,
    options: &PdfOptions,
    fonts: &dyn FontProvider,
) -> VectorResult<()> {
    let bytes = export_pdf_bytes(doc, options, fonts)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Write one or more composed scenes into a PDF document.
fn write_pdf(scenes: &[Scene], options: &PdfOptions) -> VectorResult<Vec<u8>> {
    let mut pdf = Pdf::new();
    let mut next = 1i32;
    let mut alloc = || {
        let r = Ref::new(next);
        next += 1;
        r
    };

    let catalog = alloc();
    let page_tree = alloc();

    // Four standard base-14 fonts: regular, bold, italic, bold-italic.
    let font_refs = [alloc(), alloc(), alloc(), alloc()];
    let font_names = [b"F0".as_slice(), b"F1", b"F2", b"F3"];
    let font_base = [
        b"Helvetica".as_slice(),
        b"Helvetica-Bold",
        b"Helvetica-Oblique",
        b"Helvetica-BoldOblique",
    ];

    let mut page_refs = Vec::with_capacity(scenes.len());
    let mut content_refs = Vec::with_capacity(scenes.len());
    for _ in scenes {
        page_refs.push(alloc());
        content_refs.push(alloc());
    }

    pdf.catalog(catalog).pages(page_tree);

    {
        let mut pages = pdf.pages(page_tree);
        pages.count(scenes.len() as i32);
        pages.kids(page_refs.iter().copied());
        pages.finish();
    }

    // Standard fonts (shared across pages).
    for i in 0..4 {
        pdf.type1_font(font_refs[i]).base_font(Name(font_base[i]));
    }

    // Document info.
    {
        let info = alloc();
        let mut info_dict = pdf.document_info(info);
        if let Some(t) = &options.title {
            info_dict.title(TextStr(t));
        }
        if let Some(a) = &options.author {
            info_dict.author(TextStr(a));
        }
        if let Some(s) = &options.subject {
            info_dict.subject(TextStr(s));
        }
        if !options.keywords.is_empty() {
            info_dict.keywords(TextStr(&options.keywords.join(", ")));
        }
        info_dict.creator(TextStr("Repub"));
        info_dict.producer(TextStr("repub-export-vector"));
    }

    for (i, scene) in scenes.iter().enumerate() {
        let (w, h) = (scene.size.width as f32, scene.size.height as f32);

        {
            let mut page = pdf.page(page_refs[i]);
            page.parent(page_tree);
            page.media_box(PdfRect::new(0.0, 0.0, w, h));
            page.contents(content_refs[i]);
            {
                let mut resources = page.resources();
                let mut font_dict = resources.fonts();
                for j in 0..4 {
                    font_dict.pair(Name(font_names[j]), font_refs[j]);
                }
                font_dict.finish();
                resources.finish();
            }
            page.finish();
        }

        let content = build_content(scene, &font_names);
        pdf.stream(content_refs[i], &content.finish());
    }

    Ok(pdf.finish())
}

/// Build a page's content stream from its scene.
///
/// PDF uses a bottom-left origin and y-up; the scene uses top-left y-down, so
/// the content starts with a flip transform `[1 0 0 -1 0 height]`.
fn build_content(scene: &Scene, font_names: &[&[u8]; 4]) -> Content {
    let mut content = Content::new();
    content.transform([1.0, 0.0, 0.0, -1.0, 0.0, scene.size.height as f32]);

    if let Some(bg) = &scene.background {
        let rect = GeomPath::from_rect(repub_geometry::Rect::new(
            0.0,
            0.0,
            scene.size.width,
            scene.size.height,
        ));
        fill_path(
            &mut content,
            &rect,
            Transform2D::IDENTITY,
            bg,
            FillRule::NonZero,
        );
    }

    for prim in &scene.primitives {
        emit_primitive(&mut content, prim, scene.size.height, font_names);
    }
    content
}

fn emit_primitive(
    content: &mut Content,
    prim: &Primitive,
    page_height: f64,
    font_names: &[&[u8]; 4],
) {
    match prim {
        Primitive::FillPath {
            path,
            paint,
            fill_rule,
            transform,
            ..
        } => {
            fill_path(content, path, *transform, paint, *fill_rule);
        }
        Primitive::StrokePath {
            path,
            paint,
            width,
            dash,
            transform,
            ..
        } => {
            content.save_state();
            let c = paint.primary_color();
            content.set_stroke_rgb(c.r as f32, c.g as f32, c.b as f32);
            content.set_line_width(*width as f32);
            if let Some(dashes) = dash {
                if !dashes.is_empty() {
                    let arr: Vec<f32> = dashes.iter().map(|d| *d as f32).collect();
                    content.set_dash_pattern(arr, 0.0);
                }
            }
            append_path(content, path, *transform);
            content.stroke();
            content.restore_state();
        }
        Primitive::Glyphs {
            font,
            size,
            glyphs,
            paint,
            transform,
            ..
        } => {
            if glyphs.is_empty() {
                return;
            }
            let c = paint.primary_color();
            let font_index = font_index(font.weight, font.italic);
            content.save_state();
            content.begin_text();
            content.set_fill_rgb(c.r as f32, c.g as f32, c.b as f32);
            content.set_font(Name(font_names[font_index]), *size as f32);
            // Each glyph is placed by an absolute text matrix. Because the
            // content stream is y-flipped, undo the flip for text so glyphs are
            // not drawn upside down: map page y to PDF y with a local flip.
            for g in glyphs {
                let p = transform.transform_point(Point::new(g.x, g.y));
                let pdf_y = page_height - p.y;
                content.set_text_matrix([1.0, 0.0, 0.0, 1.0, p.x as f32, pdf_y as f32]);
                let mut buf = [0u8; 4];
                let s = g.ch.encode_utf8(&mut buf);
                content.show(Str(encode_winansi(s).as_slice()));
            }
            content.end_text();
            content.restore_state();
        }
        Primitive::Image {
            dest, transform, ..
        } => {
            // Placeholder gray box (real XObject embedding is a TODO).
            content.save_state();
            content.set_fill_rgb(0.88, 0.88, 0.88);
            content.set_stroke_rgb(0.63, 0.63, 0.63);
            content.set_line_width(0.5);
            let rect = GeomPath::from_rect(*dest);
            append_path(content, &rect, *transform);
            content.fill_nonzero_and_stroke();
            content.restore_state();
        }
    }
}

fn fill_path(
    content: &mut Content,
    path: &GeomPath,
    transform: Transform2D,
    paint: &Paint,
    fill_rule: FillRule,
) {
    content.save_state();
    let c = paint.primary_color();
    content.set_fill_rgb(c.r as f32, c.g as f32, c.b as f32);
    append_path(content, path, transform);
    match fill_rule {
        FillRule::NonZero => content.fill_nonzero(),
        FillRule::EvenOdd => content.fill_even_odd(),
    };
    content.restore_state();
}

/// Append a geometry path's segments to the content stream, transformed.
fn append_path(content: &mut Content, path: &GeomPath, transform: Transform2D) {
    let tp = |p: Point| transform.transform_point(p);
    for seg in &path.segments {
        match *seg {
            PathSegment::MoveTo(p) => {
                let p = tp(p);
                content.move_to(p.x as f32, p.y as f32);
            }
            PathSegment::LineTo(p) => {
                let p = tp(p);
                content.line_to(p.x as f32, p.y as f32);
            }
            PathSegment::QuadTo(c, e) => {
                // Elevate the quadratic to a cubic for PDF, which has no quad op.
                let (c, e) = (tp(c), tp(e));
                // Approximate: use the quad control as both cubic controls.
                content.cubic_to(
                    c.x as f32, c.y as f32, c.x as f32, c.y as f32, e.x as f32, e.y as f32,
                );
            }
            PathSegment::CubicTo(c1, c2, e) => {
                let (c1, c2, e) = (tp(c1), tp(c2), tp(e));
                content.cubic_to(
                    c1.x as f32,
                    c1.y as f32,
                    c2.x as f32,
                    c2.y as f32,
                    e.x as f32,
                    e.y as f32,
                );
            }
            PathSegment::Close => {
                content.close_path();
            }
        }
    }
}

/// Pick the base-14 font index for a weight/italic combination.
fn font_index(weight: u16, italic: bool) -> usize {
    let bold = weight >= 600;
    match (bold, italic) {
        (false, false) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (true, true) => 3,
    }
}

/// Encode a short UTF-8 string into WinAnsi-ish single bytes for the base-14
/// fonts, replacing anything non-Latin-1 with `?`.
fn encode_winansi(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| {
            let code = c as u32;
            if code <= 0xFF {
                code as u8
            } else {
                b'?'
            }
        })
        .collect()
}
