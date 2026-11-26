//! Scene-driven SVG export.
//!
//! Each page is composed into a [`Scene`](repub_scene::Scene) and its
//! primitives are emitted as SVG elements: `<path>` for fills and strokes,
//! `<text>` for glyph runs, `<image>` for pictures, and
//! `<linearGradient>`/`<radialGradient>` definitions for gradient paints.
//!
//! The SVG is written directly as a string (no XML library) for tight control
//! over the output and to avoid a serialization dependency.

use std::fmt::Write as _;
use std::path::Path;

use repub_color::Color;
use repub_geometry::{Path as GeomPath, PathSegment, Point, Transform2D};
use repub_scene::{FillRule, Paint, Primitive, Scene};

use crate::error::{VectorError, VectorResult};
use crate::scene_source::compose_pages;
use repub_fonts::FontProvider;
use repub_model::Document;

/// Options for SVG export.
#[derive(Debug, Clone, Default)]
pub struct SvgOptions {
    /// Embed a white background rectangle behind the page.
    pub white_background: bool,
}

/// Export page `page` of `doc` to SVG bytes.
pub fn export_svg_bytes(
    doc: &Document,
    page: usize,
    options: &SvgOptions,
    fonts: &dyn FontProvider,
) -> VectorResult<Vec<u8>> {
    if doc.page(page).is_none() {
        return Err(VectorError::PageOutOfRange(page));
    }
    let scenes = compose_pages(doc, &[page], fonts);
    let svg = scene_to_svg(&scenes[0], options);
    Ok(svg.into_bytes())
}

/// Export page `page` of `doc` to an SVG file.
pub fn export_svg(
    doc: &Document,
    page: usize,
    path: &Path,
    options: &SvgOptions,
    fonts: &dyn FontProvider,
) -> VectorResult<()> {
    let bytes = export_svg_bytes(doc, page, options, fonts)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Render a single [`Scene`] to an SVG document string.
pub fn scene_to_svg(scene: &Scene, options: &SvgOptions) -> String {
    let mut defs = String::new();
    let mut body = String::new();
    let mut grad_id = 0usize;

    if options.white_background {
        let _ = write!(
            body,
            "<rect x=\"0\" y=\"0\" width=\"{:.3}\" height=\"{:.3}\" fill=\"#FFFFFF\"/>",
            scene.size.width, scene.size.height
        );
    }
    if let Some(bg) = &scene.background {
        emit_background(bg, scene, &mut defs, &mut body, &mut grad_id);
    }

    for prim in &scene.primitives {
        emit_primitive(prim, &mut defs, &mut body, &mut grad_id);
    }

    let mut out = String::new();
    let _ = write!(
        out,
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <svg xmlns=\"http://www.w3.org/2000/svg\" \
         xmlns:xlink=\"http://www.w3.org/1999/xlink\" \
         width=\"{w:.3}\" height=\"{h:.3}\" viewBox=\"0 0 {w:.3} {h:.3}\">",
        w = scene.size.width,
        h = scene.size.height
    );
    if !defs.is_empty() {
        let _ = write!(out, "<defs>{defs}</defs>");
    }
    out.push_str(&body);
    out.push_str("</svg>");
    out
}

fn emit_background(
    paint: &Paint,
    scene: &Scene,
    defs: &mut String,
    body: &mut String,
    grad_id: &mut usize,
) {
    let rect = GeomPath::from_rect(repub_geometry::Rect::new(
        0.0,
        0.0,
        scene.size.width,
        scene.size.height,
    ));
    let d = path_to_svg_d(&rect, Transform2D::IDENTITY);
    let fill = paint_to_svg_fill(paint, defs, grad_id);
    let _ = write!(body, "<path d=\"{d}\" fill=\"{fill}\"/>");
}

fn emit_primitive(prim: &Primitive, defs: &mut String, body: &mut String, grad_id: &mut usize) {
    match prim {
        Primitive::FillPath {
            path,
            paint,
            fill_rule,
            transform,
            ..
        } => {
            let d = path_to_svg_d(path, *transform);
            let fill = paint_to_svg_fill(paint, defs, grad_id);
            let rule = match fill_rule {
                FillRule::NonZero => "nonzero",
                FillRule::EvenOdd => "evenodd",
            };
            let _ = write!(
                body,
                "<path d=\"{d}\" fill=\"{fill}\" fill-rule=\"{rule}\"/>"
            );
        }
        Primitive::StrokePath {
            path,
            paint,
            width,
            dash,
            transform,
            ..
        } => {
            let d = path_to_svg_d(path, *transform);
            let (color, opacity) = paint_solid(paint);
            let mut attrs = format!(
                "fill=\"none\" stroke=\"{}\" stroke-width=\"{:.3}\"",
                hex(color),
                width
            );
            if opacity < 1.0 {
                let _ = write!(attrs, " stroke-opacity=\"{opacity:.3}\"");
            }
            if let Some(dashes) = dash {
                if !dashes.is_empty() {
                    let list: Vec<String> =
                        dashes.iter().map(|d| format!("{d:.3}")).collect();
                    let _ = write!(attrs, " stroke-dasharray=\"{}\"", list.join(","));
                }
            }
            let _ = write!(body, "<path d=\"{d}\" {attrs}/>");
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
            let (color, _opacity) = paint_solid(paint);
            // Anchor the run at its first glyph; place each glyph by absolute x.
            let xs: Vec<String> = glyphs.iter().map(|g| format!("{:.3}", g.x)).collect();
            let y = glyphs[0].y;
            let (origin_x, origin_y) = (xs.first().cloned().unwrap_or_default(), y);
            let text: String = glyphs.iter().map(|g| g.ch).collect();
            let weight = if font.weight >= 600 { "bold" } else { "normal" };
            let style = if font.italic { "italic" } else { "normal" };
            let transform_attr = svg_transform(*transform);
            let _ = write!(
                body,
                "<text x=\"{xlist}\" y=\"{oy:.3}\" \
                 font-family=\"{fam}\" font-size=\"{size:.3}\" \
                 font-weight=\"{weight}\" font-style=\"{style}\" \
                 fill=\"{fill}\"{tr}>{content}</text>",
                xlist = xs.join(" "),
                oy = origin_y,
                fam = escape_attr(&font.family),
                fill = hex(color),
                tr = transform_attr,
                content = escape_text(&text),
            );
            let _ = origin_x;
        }
        Primitive::Image {
            source,
            dest,
            opacity,
            transform,
            ..
        } => {
            let href = image_href(source);
            let transform_attr = svg_transform(*transform);
            let mut attrs = format!(
                "x=\"{:.3}\" y=\"{:.3}\" width=\"{:.3}\" height=\"{:.3}\"",
                dest.x(),
                dest.y(),
                dest.width(),
                dest.height()
            );
            if *opacity < 1.0 {
                let _ = write!(attrs, " opacity=\"{opacity:.3}\"");
            }
            if let Some(href) = href {
                let _ = write!(
                    body,
                    "<image {attrs}{transform_attr} xlink:href=\"{href}\"/>"
                );
            } else {
                // Placeholder box for unresolved sources.
                let _ = write!(
                    body,
                    "<rect {attrs}{transform_attr} fill=\"#E0E0E0\" stroke=\"#A0A0A0\"/>"
                );
            }
        }
    }
}

/// Resolve an image source to an SVG `xlink:href`, or `None` for a placeholder.
fn image_href(source: &repub_scene::ImageRef) -> Option<String> {
    use repub_scene::ImageRef;
    match source {
        ImageRef::File(p) => Some(escape_attr(p)),
        ImageRef::Url(u) => Some(escape_attr(u)),
        ImageRef::Embedded { data, mime_type } => {
            Some(format!("data:{mime_type};base64,{data}"))
        }
        ImageRef::Placeholder => None,
    }
}

/// Convert a geometry path (with an optional transform) to an SVG `d` string.
fn path_to_svg_d(path: &GeomPath, transform: Transform2D) -> String {
    let tp = |p: Point| transform.transform_point(p);
    let mut d = String::new();
    for seg in &path.segments {
        match *seg {
            PathSegment::MoveTo(p) => {
                let p = tp(p);
                let _ = write!(d, "M{:.3} {:.3} ", p.x, p.y);
            }
            PathSegment::LineTo(p) => {
                let p = tp(p);
                let _ = write!(d, "L{:.3} {:.3} ", p.x, p.y);
            }
            PathSegment::QuadTo(c, e) => {
                let c = tp(c);
                let e = tp(e);
                let _ = write!(d, "Q{:.3} {:.3} {:.3} {:.3} ", c.x, c.y, e.x, e.y);
            }
            PathSegment::CubicTo(c1, c2, e) => {
                let c1 = tp(c1);
                let c2 = tp(c2);
                let e = tp(e);
                let _ = write!(
                    d,
                    "C{:.3} {:.3} {:.3} {:.3} {:.3} {:.3} ",
                    c1.x, c1.y, c2.x, c2.y, e.x, e.y
                );
            }
            PathSegment::Close => d.push_str("Z "),
        }
    }
    d.trim_end().to_string()
}

/// Build the SVG fill value for a paint, registering gradients in `defs`.
fn paint_to_svg_fill(paint: &Paint, defs: &mut String, grad_id: &mut usize) -> String {
    match paint {
        Paint::Solid(c) => hex(*c),
        Paint::LinearGradient { gradient, bounds } => {
            let id = format!("grad{}", next_id(grad_id));
            let x1 = bounds.x() + gradient.start.x * bounds.width();
            let y1 = bounds.y() + gradient.start.y * bounds.height();
            let x2 = bounds.x() + gradient.end.x * bounds.width();
            let y2 = bounds.y() + gradient.end.y * bounds.height();
            let mut stops = String::new();
            for s in &gradient.stops {
                let _ = write!(
                    stops,
                    "<stop offset=\"{:.3}\" stop-color=\"{}\" stop-opacity=\"{:.3}\"/>",
                    s.position,
                    hex(s.color),
                    s.color.a
                );
            }
            let _ = write!(
                defs,
                "<linearGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" \
                 x1=\"{x1:.3}\" y1=\"{y1:.3}\" x2=\"{x2:.3}\" y2=\"{y2:.3}\">{stops}</linearGradient>"
            );
            format!("url(#{id})")
        }
        Paint::RadialGradient { gradient, bounds } => {
            let id = format!("grad{}", next_id(grad_id));
            let cx = bounds.x() + gradient.center.x * bounds.width();
            let cy = bounds.y() + gradient.center.y * bounds.height();
            let r = gradient.radius * bounds.width().max(bounds.height());
            let mut stops = String::new();
            for s in &gradient.stops {
                let _ = write!(
                    stops,
                    "<stop offset=\"{:.3}\" stop-color=\"{}\" stop-opacity=\"{:.3}\"/>",
                    s.position,
                    hex(s.color),
                    s.color.a
                );
            }
            let _ = write!(
                defs,
                "<radialGradient id=\"{id}\" gradientUnits=\"userSpaceOnUse\" \
                 cx=\"{cx:.3}\" cy=\"{cy:.3}\" r=\"{r:.3}\">{stops}</radialGradient>"
            );
            format!("url(#{id})")
        }
        Paint::Pattern { pattern, .. } => hex(pattern.foreground),
    }
}

/// A paint reduced to a solid color and opacity, for strokes and glyphs.
fn paint_solid(paint: &Paint) -> (Color, f64) {
    let c = paint.primary_color();
    (c, c.a)
}

fn next_id(grad_id: &mut usize) -> usize {
    let id = *grad_id;
    *grad_id += 1;
    id
}

/// Format an SVG `transform` attribute, omitting it for the identity.
fn svg_transform(t: Transform2D) -> String {
    if t == Transform2D::IDENTITY {
        String::new()
    } else {
        format!(
            " transform=\"matrix({:.6} {:.6} {:.6} {:.6} {:.3} {:.3})\"",
            t.a, t.b, t.c, t.d, t.tx, t.ty
        )
    }
}

fn hex(c: Color) -> String {
    c.to_hex()
}

fn escape_text(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    escape_text(s).replace('"', "&quot;")
}
