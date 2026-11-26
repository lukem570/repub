//! HTML export, driven by the document model.
//!
//! Each page becomes a positioned `<div>` sized to the page, and each shape an
//! absolutely-positioned child element: text frames become text `<div>`s,
//! pictures become `<img>` (or a placeholder box), auto shapes become styled
//! boxes, and groups recurse. This works from the model (not the scene) so the
//! output keeps live, selectable text and semantic structure rather than
//! baked-in glyph outlines.

use std::fmt::Write as _;
use std::path::Path;

use repub_color::Color;
use repub_geometry::Rect;
use repub_model::shape::{ImageSource, Shape, ShapeKind};
use repub_model::styles::Fill;
use repub_model::Document;

use crate::error::RasterExportResult;

/// Options for HTML export.
#[derive(Debug, Clone)]
pub struct HtmlOptions {
    /// Page title used in `<title>` and the document heading.
    pub title: Option<String>,
    /// Draw a drop shadow and border around each page for a "paper" look.
    pub page_frame: bool,
}

impl Default for HtmlOptions {
    fn default() -> Self {
        Self {
            title: None,
            page_frame: true,
        }
    }
}

/// Export the whole document to a single self-contained HTML string.
pub fn export_html_string(doc: &Document, options: &HtmlOptions) -> String {
    let mut out = String::new();
    let title = options
        .title
        .clone()
        .or_else(|| doc.metadata.title.clone())
        .unwrap_or_else(|| "Repub Document".to_string());

    let _ = write!(
        out,
        "<!DOCTYPE html>\n<html lang=\"en\">\n<head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{}</title>\n<style>{}</style>\n</head>\n<body>\n",
        escape(&title),
        page_css(options)
    );

    for (i, page) in doc.pages().iter().enumerate() {
        let _ = write!(
            out,
            "<div class=\"page\" style=\"width:{w:.2}pt;height:{h:.2}pt;\" \
             data-page=\"{idx}\">",
            w = page.size.width,
            h = page.size.height,
            idx = i + 1
        );
        // Master-page layers below the page's own.
        for layer in doc.page_layers(page) {
            if !layer.visible {
                continue;
            }
            for shape in layer.shapes() {
                emit_shape(&mut out, shape);
            }
        }
        out.push_str("</div>\n");
    }

    out.push_str("</body>\n</html>\n");
    out
}

/// Export the whole document to an HTML file.
pub fn export_html(doc: &Document, path: &Path, options: &HtmlOptions) -> RasterExportResult<()> {
    let html = export_html_string(doc, options);
    std::fs::write(path, html)?;
    Ok(())
}

fn page_css(options: &HtmlOptions) -> String {
    let frame = if options.page_frame {
        "box-shadow:0 2px 8px rgba(0,0,0,0.25);border:1px solid #ccc;"
    } else {
        ""
    };
    format!(
        "body{{background:#525659;margin:0;padding:24px;\
         display:flex;flex-direction:column;align-items:center;gap:24px;}}\
         .page{{position:relative;background:#fff;overflow:hidden;{frame}}}\
         .shape{{position:absolute;box-sizing:border-box;}}\
         .text{{white-space:pre-wrap;overflow:hidden;}}\
         img.shape{{object-fit:contain;}}"
    )
}

fn emit_shape(out: &mut String, shape: &Shape) {
    if !shape.common.visible {
        return;
    }
    let b = shape.common.bounds;
    match &shape.kind {
        ShapeKind::Group(group) => {
            for child in &group.children {
                emit_shape(out, child);
            }
        }
        ShapeKind::TextFrame(tf) => {
            let text = tf.content.plain_text();
            let style = base_box_style(shape, b);
            let color = text_color(shape);
            let _ = write!(
                out,
                "<div class=\"shape text\" style=\"{style}color:{color};\
                 padding:{pad:.2}pt;\">{content}</div>",
                pad = tf.inset,
                content = escape(&text),
            );
        }
        ShapeKind::Picture(pic) => {
            let pos = position_style(b, shape.common.rotation, shape.opacity);
            match img_src(&pic.source) {
                Some(src) => {
                    let _ = write!(
                        out,
                        "<img class=\"shape\" style=\"{pos}\" src=\"{src}\" alt=\"{alt}\">",
                        alt = escape(pic.alt_text.as_deref().unwrap_or("")),
                    );
                }
                None => {
                    let _ = write!(
                        out,
                        "<div class=\"shape\" style=\"{pos}background:#e0e0e0;\
                         border:1px solid #a0a0a0;\"></div>"
                    );
                }
            }
        }
        ShapeKind::Table(_) => {
            // Render the table bounds as a bordered box; full cell layout is a
            // TODO (see docs/TODO.md).
            let style = base_box_style(shape, b);
            let _ = write!(
                out,
                "<div class=\"shape\" style=\"{style}border:0.5pt solid #000;\"></div>"
            );
        }
        ShapeKind::TextEffect(te) => {
            let style = base_box_style(shape, b);
            let color = text_color(shape);
            let _ = write!(
                out,
                "<div class=\"shape text\" style=\"{style}color:{color};\
                 font-weight:bold;\">{content}</div>",
                content = escape(&te.text),
            );
        }
        ShapeKind::Line(line) => {
            // Draw a thin rotated box spanning start→end.
            let dx = line.end.x - line.start.x;
            let dy = line.end.y - line.start.y;
            let len = (dx * dx + dy * dy).sqrt();
            let angle = dy.atan2(dx).to_degrees();
            let width = shape.line.as_ref().map(|s| s.width).unwrap_or(1.0);
            let color = shape
                .line
                .as_ref()
                .map(|s| s.color)
                .unwrap_or(Color::BLACK);
            let _ = write!(
                out,
                "<div class=\"shape\" style=\"left:{x:.2}pt;top:{y:.2}pt;\
                 width:{len:.2}pt;height:{width:.2}pt;background:{col};\
                 transform:rotate({angle:.3}deg);transform-origin:0 50%;\"></div>",
                x = line.start.x,
                y = line.start.y - width / 2.0,
                col = css_color(color),
            );
        }
        ShapeKind::AutoShape(_) => {
            let style = base_box_style(shape, b);
            let _ = write!(out, "<div class=\"shape\" style=\"{style}\"></div>");
        }
    }
}

/// Base positioned-box CSS for a shape: position, fill and border.
fn base_box_style(shape: &Shape, b: Rect) -> String {
    let mut s = position_style(b, shape.common.rotation, shape.opacity);
    match &shape.fill {
        Fill::Solid(c) => {
            let _ = write!(s, "background:{};", css_color(*c));
        }
        Fill::LinearGradient(g) => {
            if let (Some(a), Some(z)) = (g.stops.first(), g.stops.last()) {
                let _ = write!(
                    s,
                    "background:linear-gradient(90deg,{} ,{});",
                    css_color(a.color),
                    css_color(z.color)
                );
            }
        }
        Fill::RadialGradient(g) => {
            if let (Some(a), Some(z)) = (g.stops.first(), g.stops.last()) {
                let _ = write!(
                    s,
                    "background:radial-gradient(circle,{} ,{});",
                    css_color(a.color),
                    css_color(z.color)
                );
            }
        }
        Fill::Pattern(p) => {
            let _ = write!(s, "background:{};", css_color(p.foreground));
        }
        Fill::Texture(_) | Fill::None => {}
    }
    if let Some(stroke) = &shape.line {
        let _ = write!(
            s,
            "border:{:.2}pt solid {};",
            stroke.width,
            css_color(stroke.color)
        );
    }
    s
}

fn position_style(b: Rect, rotation: f64, opacity: f64) -> String {
    let mut s = format!(
        "left:{x:.2}pt;top:{y:.2}pt;width:{w:.2}pt;height:{h:.2}pt;",
        x = b.x(),
        y = b.y(),
        w = b.width(),
        h = b.height()
    );
    if rotation != 0.0 {
        let _ = write!(s, "transform:rotate({rotation:.3}deg);");
    }
    if opacity < 1.0 {
        let _ = write!(s, "opacity:{opacity:.3};");
    }
    s
}

fn text_color(shape: &Shape) -> String {
    // Use the first run's color if present; otherwise black.
    if let ShapeKind::TextFrame(tf) = &shape.kind {
        if let Some(p) = tf.content.paragraphs.first() {
            for el in &p.elements {
                if let repub_model::text::InlineElement::Text(run) = el {
                    return css_color(run.style.color);
                }
            }
        }
    }
    if let ShapeKind::TextEffect(te) = &shape.kind {
        let _ = &te.style;
        return css_color(te.style.color);
    }
    css_color(Color::BLACK)
}

fn img_src(source: &ImageSource) -> Option<String> {
    match source {
        ImageSource::File(p) => Some(escape(p)),
        ImageSource::Url(u) => Some(escape(u)),
        ImageSource::Embedded { data, mime_type } => {
            Some(format!("data:{mime_type};base64,{data}"))
        }
        ImageSource::Placeholder => None,
    }
}

fn css_color(c: Color) -> String {
    let (r, g, b, a) = c.to_rgba8();
    if a == 255 {
        format!("#{r:02X}{g:02X}{b:02X}")
    } else {
        format!("rgba({r},{g},{b},{:.3})", a as f64 / 255.0)
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
