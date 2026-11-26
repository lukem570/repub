//! Image decoding and drawing for [`Primitive::Image`](repub_scene::Primitive::Image).
//!
//! Supported sources:
//! - [`ImageRef::File`] — decoded from the filesystem via the `image` crate.
//! - [`ImageRef::Embedded`] — base64-decoded then decoded via the `image`
//!   crate (format auto-detected, with a magic-byte fallback).
//! - [`ImageRef::Url`] — **not fetched**. No network access is performed; a
//!   placeholder is drawn instead. Resolving URLs to bytes is the caller's
//!   responsibility and is out of scope for this backend.
//! - [`ImageRef::Placeholder`] — draws the missing-image placeholder.

use base64::Engine;
use image::{DynamicImage, ImageFormat};
use repub_geometry::Rect;
use tiny_skia::{Paint, PathBuilder, Pixmap, PixmapPaint, Stroke, Transform};

use repub_scene::ImageRef;

/// Decoded image pixels in straight (non-premultiplied) RGBA order.
struct DecodedImage {
    data: Vec<u8>,
    width: u32,
    height: u32,
}

impl DecodedImage {
    fn from_dynamic(img: DynamicImage) -> Self {
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Self {
            data: rgba.into_raw(),
            width,
            height,
        }
    }

    /// Convert to a premultiplied-alpha tiny-skia pixmap.
    fn to_pixmap(&self) -> Option<Pixmap> {
        let mut pixmap = Pixmap::new(self.width, self.height)?;
        let dst = pixmap.data_mut();
        for (i, chunk) in self.data.chunks(4).enumerate() {
            let (r, g, b, a) = (chunk[0], chunk[1], chunk[2], chunk[3]);
            let idx = i * 4;
            match a {
                255 => {
                    dst[idx] = r;
                    dst[idx + 1] = g;
                    dst[idx + 2] = b;
                    dst[idx + 3] = a;
                }
                0 => {
                    dst[idx] = 0;
                    dst[idx + 1] = 0;
                    dst[idx + 2] = 0;
                    dst[idx + 3] = 0;
                }
                _ => {
                    let af = a as f32 / 255.0;
                    dst[idx] = (r as f32 * af) as u8;
                    dst[idx + 1] = (g as f32 * af) as u8;
                    dst[idx + 2] = (b as f32 * af) as u8;
                    dst[idx + 3] = a;
                }
            }
        }
        Some(pixmap)
    }
}

/// Decode an image source into pixels, or `None` if it cannot be resolved.
///
/// Returns `None` for [`ImageRef::Url`] (never fetched) and
/// [`ImageRef::Placeholder`], which signal the caller to draw a placeholder.
fn decode(source: &ImageRef) -> Option<DecodedImage> {
    match source {
        ImageRef::File(path) => image::open(path).ok().map(DecodedImage::from_dynamic),
        ImageRef::Embedded { data, .. } => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data.trim())
                .ok()?;
            decode_bytes(&bytes).map(DecodedImage::from_dynamic)
        }
        // URLs are intentionally not fetched by this backend.
        ImageRef::Url(_) => None,
        ImageRef::Placeholder => None,
    }
}

fn decode_bytes(data: &[u8]) -> Option<DynamicImage> {
    if let Ok(img) = image::load_from_memory(data) {
        return Some(img);
    }
    if let Some(format) = detect_format(data) {
        if let Ok(img) = image::load_from_memory_with_format(data, format) {
            return Some(img);
        }
    }
    None
}

fn detect_format(data: &[u8]) -> Option<ImageFormat> {
    if data.len() < 12 {
        return None;
    }
    if data.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        return Some(ImageFormat::Png);
    }
    if data.starts_with(&[0xFF, 0xD8, 0xFF]) {
        return Some(ImageFormat::Jpeg);
    }
    if data.starts_with(b"GIF87a") || data.starts_with(b"GIF89a") {
        return Some(ImageFormat::Gif);
    }
    if data.starts_with(&[0x49, 0x49, 0x2A, 0x00]) || data.starts_with(&[0x4D, 0x4D, 0x00, 0x2A]) {
        return Some(ImageFormat::Tiff);
    }
    None
}

/// Draw an image primitive into `pixmap`.
///
/// `dest` is the on-page destination rectangle (before `transform`), `crop` is
/// an optional normalized (0–1) source crop, `opacity` scales the whole image,
/// and `transform` is the device transform to draw under. Sources that cannot
/// be resolved draw a missing-image placeholder.
pub fn draw_image(
    pixmap: &mut Pixmap,
    source: &ImageRef,
    dest: &Rect,
    crop: Option<&Rect>,
    opacity: f64,
    transform: &Transform,
) {
    let Some(decoded) = decode(source) else {
        draw_placeholder(pixmap, dest, transform);
        return;
    };
    let Some(img_pixmap) = decoded.to_pixmap() else {
        draw_placeholder(pixmap, dest, transform);
        return;
    };

    // Source sub-rectangle in pixels, from the normalized crop.
    let (src_x, src_y, src_w, src_h) = match crop {
        Some(c) => (
            c.x() * decoded.width as f64,
            c.y() * decoded.height as f64,
            (c.width() * decoded.width as f64).max(1.0),
            (c.height() * decoded.height as f64).max(1.0),
        ),
        None => (0.0, 0.0, decoded.width as f64, decoded.height as f64),
    };

    let scale_x = dest.width() / src_w;
    let scale_y = dest.height() / src_h;

    let placement = transform
        .pre_translate(dest.x() as f32, dest.y() as f32)
        .pre_scale(scale_x as f32, scale_y as f32)
        .pre_translate(-src_x as f32, -src_y as f32);

    let paint = PixmapPaint {
        opacity: opacity.clamp(0.0, 1.0) as f32,
        ..Default::default()
    };

    pixmap.draw_pixmap(0, 0, img_pixmap.as_ref(), &paint, placement, None);
}

/// Draw a gray placeholder box with an X for an unresolved image.
fn draw_placeholder(pixmap: &mut Pixmap, dest: &Rect, transform: &Transform) {
    let Some(rect) = tiny_skia::Rect::from_xywh(
        dest.x() as f32,
        dest.y() as f32,
        dest.width() as f32,
        dest.height() as f32,
    ) else {
        return;
    };

    let mut fill = Paint::default();
    fill.set_color(tiny_skia::Color::from_rgba8(220, 220, 220, 255));
    fill.anti_alias = true;
    let path = PathBuilder::from_rect(rect);
    pixmap.fill_path(&path, &fill, tiny_skia::FillRule::Winding, *transform, None);

    let mut stroke_paint = Paint::default();
    stroke_paint.set_color(tiny_skia::Color::from_rgba8(150, 150, 150, 255));
    stroke_paint.anti_alias = true;
    let stroke = Stroke {
        width: 1.0,
        ..Default::default()
    };
    pixmap.stroke_path(&path, &stroke_paint, &stroke, *transform, None);

    let mut builder = PathBuilder::new();
    builder.move_to(dest.x() as f32, dest.y() as f32);
    builder.line_to(dest.right() as f32, dest.bottom() as f32);
    builder.move_to(dest.right() as f32, dest.y() as f32);
    builder.line_to(dest.x() as f32, dest.bottom() as f32);
    if let Some(x_path) = builder.finish() {
        pixmap.stroke_path(&x_path, &stroke_paint, &stroke, *transform, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_source_decodes_to_none() {
        assert!(decode(&ImageRef::Url("http://example.com/x.png".into())).is_none());
    }

    #[test]
    fn placeholder_source_decodes_to_none() {
        assert!(decode(&ImageRef::Placeholder).is_none());
    }

    #[test]
    fn embedded_png_decodes() {
        // 1x1 red PNG, base64.
        let mut pixmap = Pixmap::new(1, 1).unwrap();
        pixmap.fill(tiny_skia::Color::from_rgba8(255, 0, 0, 255));
        let png = pixmap.encode_png().unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&png);
        let decoded = decode(&ImageRef::Embedded {
            data: b64,
            mime_type: "image/png".into(),
        });
        assert!(decoded.is_some());
        let d = decoded.unwrap();
        assert_eq!((d.width, d.height), (1, 1));
    }

    #[test]
    fn draws_placeholder_for_url() {
        let mut pixmap = Pixmap::new(20, 20).unwrap();
        draw_image(
            &mut pixmap,
            &ImageRef::Url("http://x".into()),
            &Rect::new(2.0, 2.0, 16.0, 16.0),
            None,
            1.0,
            &Transform::identity(),
        );
        // The placeholder fill leaves ink somewhere in the box.
        let any_ink = pixmap.data().chunks(4).any(|p| p[3] > 0);
        assert!(any_ink);
    }
}
