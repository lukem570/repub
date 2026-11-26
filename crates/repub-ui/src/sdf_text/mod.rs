//! Signed-distance-field text for the canvas.
//!
//! Document text (text frames) is drawn from a distance-field glyph atlas by a
//! WGSL shader that antialiases in screen space, so it stays crisp at every
//! zoom level without rasterizing glyphs per size. UI text is unaffected and
//! still uses egui's own renderer.
//!
//! Requires eframe's wgpu backend. Until [`install`] has run and the atlas has
//! finished building in the background, [`draw_text_frame`] returns `false`
//! and the caller draws text with egui instead.
//!
//! A text frame is a [`repub_model::shape::Shape`] whose
//! [`kind`](repub_model::shape::ShapeKind) is
//! [`TextFrame`](repub_model::shape::ShapeKind::TextFrame): the frame geometry
//! (bounds, rotation, id) lives on `shape.common`, and the text content on the
//! [`TextFrameData`](repub_model::shape::TextFrameData) inside. The functions
//! here take the whole `Shape`.

#![allow(missing_docs)]

mod atlas;
pub mod fonts;
mod layout;
mod render;

use eframe::egui_wgpu;
use egui::{Painter, Pos2, Rect};
use repub_model::shape::{Shape, ShapeKind, TextFitMode, TextFrameData};

pub use layout::{set_exclusions, set_file_name, set_page_context, CaretLine, Exclusion, FrameLayout};
pub use render::install;
use render::{TextCallback, Vertex};

/// The `TextFrameData` of a text-frame shape, or `None` for other kinds.
fn text_data(shape: &Shape) -> Option<&TextFrameData> {
    match &shape.kind {
        ShapeKind::TextFrame(d) => Some(d),
        _ => None,
    }
}

/// Lay out a text frame (document points, unrotated). Builds the glyph atlas on
/// this thread if it isn't ready yet, so it works without a GPU (editing, tests).
///
/// `shape` must be a [`ShapeKind::TextFrame`]; other kinds lay out as empty.
pub fn layout_frame(shape: &Shape) -> FrameLayout {
    prepare(shape);
    layout::layout_text_frame(&atlas::get_or_build(), shape)
}

/// Load the fonts `shape`'s text uses and bake any glyphs it needs that aren't
/// in the atlas yet. Cheap when nothing is missing (a read-only check), and a
/// no-op for shapes that are not text frames.
pub fn prepare(shape: &Shape) {
    use repub_model::text::InlineElement;
    let Some(tf) = text_data(shape) else { return };
    // Every (style, text) pair the frame draws
    let mut runs: Vec<(&repub_model::styles::CharacterStyle, std::borrow::Cow<str>)> = Vec::new();
    for para in &tf.content.paragraphs {
        // Spaces (tabs), list markers and field text (page numbers) use the
        // paragraph's style
        let mut extra = String::from(" 0123456789");
        match &para.style.list_style {
            repub_model::styles::ListStyle::Bullet { character, .. } => extra.push(*character),
            repub_model::styles::ListStyle::Numbered { .. } => extra.push_str(".abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"),
            repub_model::styles::ListStyle::None => {}
        }
        runs.push((&para.style.default_char_style, extra.into()));
        for element in &para.elements {
            match element {
                InlineElement::Text(run) => runs.push((&run.style, run.text.as_str().into())),
                InlineElement::Link(link) => runs.push((&link.style, link.text.as_str().into())),
                _ => {}
            }
        }
    }
    let missing = {
        let atlas = atlas::get_or_build();
        runs.iter().any(|(style, text)| {
            !atlas.knows(style) || {
                let (face, _) = atlas.resolve(style);
                text.chars().any(|c| !atlas.has_glyph(face, c) || ((style.all_caps || style.small_caps) && c.to_uppercase().any(|u| !atlas.has_glyph(face, u))))
            }
        })
    };
    if !missing {
        return;
    }
    let mut atlas = atlas::write();
    // Characters per face, baked together (in parallel)
    let mut wanted: std::collections::BTreeMap<usize, Vec<char>> = Default::default();
    for (style, text) in runs {
        atlas.load(style);
        let (face, _) = atlas.resolve(style);
        let chars = wanted.entry(face).or_default();
        // '?' stands in for characters a face lacks
        chars.push('?');
        for c in text.chars() {
            chars.push(c);
            if style.all_caps || style.small_caps {
                chars.extend(c.to_uppercase());
            }
        }
    }
    for (face, chars) in wanted {
        atlas.bake_many(face, &chars);
    }
}

/// Height in document points that a text frame's content needs when laid out at
/// the sizes as written, with no shrink/grow fitting.
///
/// Table rows use this so a row can grow to hold its text instead of the text
/// being scaled down or clipped. The probe frame is given enough height that
/// every line is laid out, whichever way the text wraps. Returns `0.0` for a
/// shape that is not a text frame.
pub fn content_height(shape: &Shape) -> f64 {
    const PROBE_HEIGHT: f64 = 100_000.0;
    let Some(tf) = text_data(shape) else { return 0.0 };
    let mut probe = shape.clone();
    let probe_data = TextFrameData {
        text_fit: TextFitMode::None,
        ..tf.clone()
    };
    probe.common.bounds.size.height = PROBE_HEIGHT;
    probe.kind = ShapeKind::TextFrame(probe_data);
    prepare(&probe);
    let layout = layout::layout_text_frame(&atlas::get_or_build(), &probe);
    let top = shape.common.bounds.origin.y;
    let m = tf.effective_margins();
    let bottom = layout
        .lines
        .iter()
        .map(|l| l.bottom as f64)
        .fold(top + m.top, f64::max);
    (bottom - top + m.bottom).clamp(0.0, PROBE_HEIGHT)
}

/// Call once per frame before drawing any text frames.
pub fn begin_frame() {
    render::begin_frame();
}

/// Draw a text frame's text with SDF rendering.
///
/// `page_origin` is the page's top-left on screen and `zoom` the points-to-screen
/// scale. Text is clipped to the (rotated) frame and to `painter`'s clip rect.
/// Returns `false` when SDF text isn't available yet; the caller should draw
/// the text another way for this frame. A shape that is not a text frame draws
/// nothing and returns `true`.
pub fn draw_text_frame(painter: &Painter, page_origin: Pos2, zoom: f32, shape: &Shape) -> bool {
    if !render::installed() {
        return false;
    }
    if text_data(shape).is_none() {
        return true;
    }
    // The atlas starts empty and builds in milliseconds; this frame's callback
    // uploads it to the GPU before drawing, so text never waits a frame

    // Document space -> screen, rotating about the frame centre
    let bounds = shape.common.bounds;
    let center = Pos2::new((bounds.origin.x + bounds.size.width / 2.0) as f32, (bounds.origin.y + bounds.size.height / 2.0) as f32);
    let (sin, cos) = (shape.common.rotation as f32).to_radians().sin_cos();
    let to_screen = |x: f32, y: f32| -> Pos2 {
        let (dx, dy) = (x - center.x, y - center.y);
        let rotated = Pos2::new(center.x + dx * cos - dy * sin, center.y + dx * sin + dy * cos);
        page_origin + rotated.to_vec2() * zoom
    };

    // Clip to the frame's screen-space bounding box
    let corners = [
        to_screen(bounds.origin.x as f32, bounds.origin.y as f32),
        to_screen((bounds.origin.x + bounds.size.width) as f32, bounds.origin.y as f32),
        to_screen((bounds.origin.x + bounds.size.width) as f32, (bounds.origin.y + bounds.size.height) as f32),
        to_screen(bounds.origin.x as f32, (bounds.origin.y + bounds.size.height) as f32),
    ];
    // egui-wgpu rounds the callback viewport to whole physical pixels. Snap the
    // clip the same way so the NDC mapping below matches the viewport the GPU
    // actually uses; otherwise text drifts against the page by up to half a
    // pixel, differently every frame, and jitters while zooming.
    let ppp = painter.ctx().pixels_per_point();
    let snap = |p: Pos2| Pos2::new((p.x * ppp).round() / ppp, (p.y * ppp).round() / ppp);
    let clip = Rect::from_points(&corners).intersect(painter.clip_rect());
    let clip = Rect::from_min_max(snap(clip.min), snap(clip.max));
    if clip.width() <= 0.0 || clip.height() <= 0.0 {
        return true;
    }

    prepare(shape);
    let quads = layout::layout_text_frame(&atlas::get_or_build(), shape).quads;
    if quads.is_empty() {
        return true;
    }

    // The callback viewport is `clip`, so positions are NDC relative to it
    let to_ndc = |p: Pos2| -> [f32; 2] {
        [(p.x - clip.min.x) / clip.width() * 2.0 - 1.0, 1.0 - (p.y - clip.min.y) / clip.height() * 2.0]
    };
    let mut vertices = Vec::with_capacity(quads.len() * 6);
    for q in &quads {
        let [x0, y0, x1, y1] = q.rect;
        let [u0, v0, u1, v1] = q.uv;
        // Synthetic italic: shear horizontally about the baseline
        let shear = |x: f32, y: f32| to_screen(x + q.skew * (q.baseline - y), y);
        let corner = |x: f32, y: f32, u: f32, v: f32| Vertex { position: to_ndc(shear(x, y)), uv: [u, v], color: q.color };
        let (tl, tr, br, bl) = (corner(x0, y0, u0, v0), corner(x1, y0, u1, v0), corner(x1, y1, u1, v1), corner(x0, y1, u0, v1));
        vertices.extend_from_slice(&[tl, tr, br, tl, br, bl]);
    }

    let slot = render::next_slot();
    painter
        .with_clip_rect(clip)
        .add(egui_wgpu::Callback::new_paint_callback(clip, TextCallback { slot: Some(slot), vertices }));
    true
}
