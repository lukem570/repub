//! Decoding an [`ImageRef`] into straight-RGBA8 pixels.
//!
//! - [`ImageRef::File`] is read and decoded from disk with the `image` crate.
//! - [`ImageRef::Embedded`] is base64-decoded then decoded, with a magic-byte
//!   format fallback.
//! - [`ImageRef::Url`] is **never fetched**; it resolves to `None` so the
//!   caller draws a placeholder. Network access is out of scope.
//! - [`ImageRef::Placeholder`] resolves to `None`.

use base64::Engine;
use image::{DynamicImage, ImageFormat};
use repub_scene::ImageRef;

/// Decoded straight (non-premultiplied) RGBA8 pixels.
pub struct DecodedImage {
    /// RGBA8 pixel data, row-major.
    pub data: Vec<u8>,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
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
}

/// Decode an image source, or `None` when it cannot be resolved locally.
pub fn decode(source: &ImageRef) -> Option<DecodedImage> {
    match source {
        ImageRef::File(path) => image::open(path).ok().map(DecodedImage::from_dynamic),
        ImageRef::Embedded { data, .. } => {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(data.trim())
                .ok()?;
            decode_bytes(&bytes).map(DecodedImage::from_dynamic)
        }
        ImageRef::Url(_) => None,
        ImageRef::Placeholder => None,
    }
}

fn decode_bytes(data: &[u8]) -> Option<DynamicImage> {
    if let Ok(img) = image::load_from_memory(data) {
        return Some(img);
    }
    let format = detect_format(data)?;
    image::load_from_memory_with_format(data, format).ok()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_and_placeholder_are_none() {
        assert!(decode(&ImageRef::Url("http://x/y.png".into())).is_none());
        assert!(decode(&ImageRef::Placeholder).is_none());
    }

    #[test]
    fn embedded_png_decodes() {
        // Encode a 2×1 red PNG with the image crate, then decode it back.
        let buf = image::RgbaImage::from_pixel(2, 1, image::Rgba([255, 0, 0, 255]));
        let mut bytes: Vec<u8> = Vec::new();
        image::DynamicImage::ImageRgba8(buf)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
        let decoded = decode(&ImageRef::Embedded {
            data: b64,
            mime_type: "image/png".into(),
        });
        assert!(decoded.is_some());
        let d = decoded.unwrap();
        assert_eq!((d.width, d.height), (2, 1));
    }
}
