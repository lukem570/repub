//! Realization of [`SceneEffect`] descriptors onto tiny-skia pixmaps.
//!
//! Each primitive that carries effects is first drawn into an isolated layer
//! pixmap sized to the scene; the effects here composite shadows, glows,
//! reflections and edge treatments around or onto that layer before it is drawn
//! back over the scene. The pixmap operations (box blur, colorize, reflection,
//! bevel shading) are ported from the original `repub-render-cpu` effects, with
//! their inputs adapted from the old style structs to [`SceneEffect`] data.
//!
//! ## Realized vs. approximated
//!
//! - **DropShadow**, **OuterGlow**, **InnerShadow**, **Reflection**,
//!   **SoftEdges** are realized.
//! - **InnerGlow** is realized as an inner shadow using the glow color (a close
//!   approximation; the `from_center` distinction is not modeled).
//! - **Bevel** applies a simple edge highlight/shadow shading pass; the
//!   original's chisel/emboss techniques and lighting model are not ported.
//! - The `spread`/`choke`/`depth` parameters are not independently modeled;
//!   only offsets and blur radii drive the result.

use repub_color::Color;
use repub_scene::SceneEffect;
use tiny_skia::{BlendMode, Pixmap, PixmapPaint, Transform};

/// Composite a primitive layer and its effects onto `target`.
///
/// `layer` is the primitive already drawn into a scene-sized pixmap. Behind-the
/// primitive effects (shadows, glow, reflection) are drawn first, then the
/// primitive layer, then on-top edge treatments. `scale` converts effect
/// parameters expressed in points into device pixels.
pub fn composite_with_effects(
    target: &mut Pixmap,
    layer: &Pixmap,
    effects: &[SceneEffect],
    scale: f32,
) {
    for effect in effects {
        match effect {
            SceneEffect::DropShadow {
                color,
                offset_x,
                offset_y,
                blur_radius,
                ..
            } => {
                draw_shadow(
                    target,
                    layer,
                    color,
                    (*offset_x as f32 * scale) as i32,
                    (*offset_y as f32 * scale) as i32,
                    *blur_radius as f32 * scale,
                );
            }
            SceneEffect::OuterGlow { color, radius, .. } => {
                draw_glow(target, layer, color, *radius as f32 * scale);
            }
            SceneEffect::Reflection {
                opacity,
                fade_length,
                gap,
                blur,
                size,
            } => {
                draw_reflection(
                    target,
                    layer,
                    *opacity,
                    *fade_length,
                    *gap as f32 * scale,
                    *blur as f32 * scale,
                    *size,
                );
            }
            _ => {}
        }
    }

    // The primitive itself.
    let mut composited = layer.clone();

    for effect in effects {
        match effect {
            SceneEffect::InnerShadow {
                color,
                offset_x,
                offset_y,
                blur_radius,
                ..
            } => {
                apply_inner_shadow(
                    &mut composited,
                    color,
                    (*offset_x as f32 * scale) as i32,
                    (*offset_y as f32 * scale) as i32,
                    *blur_radius as f32 * scale,
                );
            }
            SceneEffect::InnerGlow { color, radius, .. } => {
                // Approximate an inner glow as a zero-offset inner shadow.
                apply_inner_shadow(&mut composited, color, 0, 0, *radius as f32 * scale);
            }
            SceneEffect::SoftEdges { radius } => {
                apply_soft_edges(&mut composited, (*radius as f32 * scale) as usize);
            }
            SceneEffect::Bevel {
                size,
                highlight,
                shadow,
                ..
            } => {
                apply_bevel(
                    &mut composited,
                    (*size as f32 * scale) as usize,
                    highlight,
                    shadow,
                );
            }
            _ => {}
        }
    }

    target.draw_pixmap(
        0,
        0,
        composited.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

fn draw_shadow(
    target: &mut Pixmap,
    source: &Pixmap,
    color: &Color,
    offset_x: i32,
    offset_y: i32,
    blur: f32,
) {
    let mut shadow = source.clone();
    colorize(&mut shadow, color);
    if blur > 0.0 {
        box_blur(&mut shadow, blur);
    }
    let paint = PixmapPaint {
        opacity: color.a as f32,
        blend_mode: BlendMode::SourceOver,
        ..Default::default()
    };
    target.draw_pixmap(
        offset_x,
        offset_y,
        shadow.as_ref(),
        &paint,
        Transform::identity(),
        None,
    );
}

fn draw_glow(target: &mut Pixmap, source: &Pixmap, color: &Color, radius: f32) {
    let mut glow = source.clone();
    colorize(&mut glow, color);
    if radius > 0.0 {
        box_blur(&mut glow, radius);
    }
    let paint = PixmapPaint {
        opacity: color.a as f32,
        blend_mode: BlendMode::SourceOver,
        ..Default::default()
    };
    target.draw_pixmap(0, 0, glow.as_ref(), &paint, Transform::identity(), None);
}

fn apply_inner_shadow(target: &mut Pixmap, color: &Color, offset_x: i32, offset_y: i32, blur: f32) {
    let (w, h) = (target.width(), target.height());
    let Some(mut shadow) = Pixmap::new(w, h) else {
        return;
    };
    shadow.fill(
        tiny_skia::Color::from_rgba(
            color.r as f32,
            color.g as f32,
            color.b as f32,
            color.a as f32,
        )
        .unwrap_or(tiny_skia::Color::BLACK),
    );
    if blur > 0.0 {
        box_blur(&mut shadow, blur);
    }
    // Keep the shadow only where the primitive is opaque (offset into it).
    let mask_paint = PixmapPaint {
        blend_mode: BlendMode::DestinationIn,
        ..Default::default()
    };
    shadow.draw_pixmap(
        -offset_x,
        -offset_y,
        target.as_ref(),
        &mask_paint,
        Transform::identity(),
        None,
    );
    target.draw_pixmap(
        0,
        0,
        shadow.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

fn apply_soft_edges(pixmap: &mut Pixmap, radius: usize) {
    if radius == 0 {
        return;
    }
    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;
    if width == 0 || height == 0 {
        return;
    }
    let data = pixmap.data_mut();
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 4;
            let dist_x = x.min(width - 1 - x);
            let dist_y = y.min(height - 1 - y);
            let dist = dist_x.min(dist_y);
            if dist < radius {
                let factor = dist as f32 / radius as f32;
                data[idx] = (data[idx] as f32 * factor) as u8;
                data[idx + 1] = (data[idx + 1] as f32 * factor) as u8;
                data[idx + 2] = (data[idx + 2] as f32 * factor) as u8;
                data[idx + 3] = (data[idx + 3] as f32 * factor) as u8;
            }
        }
    }
}

fn apply_bevel(pixmap: &mut Pixmap, size: usize, highlight: &Color, shadow: &Color) {
    if size == 0 {
        return;
    }
    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;
    if width < 3 || height < 3 {
        return;
    }
    let alpha: Vec<u8> = pixmap.data().chunks(4).map(|c| c[3]).collect();
    let (hr, hg, hb) = (
        (highlight.r * 255.0) as u8,
        (highlight.g * 255.0) as u8,
        (highlight.b * 255.0) as u8,
    );
    let (sr, sg, sb) = (
        (shadow.r * 255.0) as u8,
        (shadow.g * 255.0) as u8,
        (shadow.b * 255.0) as u8,
    );
    let data = pixmap.data_mut();
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let idx = (y * width + x) * 4;
            if alpha[y * width + x] == 0 {
                continue;
            }
            let dx = alpha[y * width + x + 1] as f32 - alpha[y * width + x - 1] as f32;
            let dy = alpha[(y + 1) * width + x] as f32 - alpha[(y - 1) * width + x] as f32;
            let slope = (dx - dy) / 255.0;
            let intensity = (slope.abs() * 0.5).min(1.0);
            if slope > 0.0 {
                blend(&mut data[idx..idx + 3], (hr, hg, hb), intensity);
            } else if slope < 0.0 {
                blend(&mut data[idx..idx + 3], (sr, sg, sb), intensity);
            }
        }
    }
}

fn draw_reflection(
    target: &mut Pixmap,
    source: &Pixmap,
    opacity: f64,
    fade_length: f64,
    gap: f32,
    blur: f32,
    size: f64,
) {
    let width = source.width();
    let height = source.height();
    let refl_h = (height as f64 * size.clamp(0.0, 1.0)) as u32;
    if refl_h == 0 {
        return;
    }
    let Some(mut refl) = Pixmap::new(width, refl_h) else {
        return;
    };
    let src = source.data();
    let dst = refl.data_mut();
    for ry in 0..refl_h as usize {
        let src_row = height as usize - 1 - (ry * height as usize / refl_h as usize);
        if src_row >= height as usize {
            continue;
        }
        let progress = ry as f64 / refl_h as f64;
        let fade = if fade_length > 0.0 && progress < fade_length {
            opacity * (1.0 - progress / fade_length)
        } else if fade_length <= 0.0 {
            opacity
        } else {
            0.0
        };
        for rx in 0..width as usize {
            let si = (src_row * width as usize + rx) * 4;
            let di = (ry * width as usize + rx) * 4;
            let a = (src[si + 3] as f64 * fade) as u8;
            if a > 0 {
                let f = a as f64 / 255.0;
                dst[di] = (src[si] as f64 * f) as u8;
                dst[di + 1] = (src[si + 1] as f64 * f) as u8;
                dst[di + 2] = (src[si + 2] as f64 * f) as u8;
                dst[di + 3] = a;
            }
        }
    }
    if blur > 0.0 {
        box_blur(&mut refl, blur);
    }
    let refl_y = height as i32 + gap as i32;
    target.draw_pixmap(
        0,
        refl_y,
        refl.as_ref(),
        &PixmapPaint::default(),
        Transform::identity(),
        None,
    );
}

fn blend(channels: &mut [u8], overlay: (u8, u8, u8), intensity: f32) {
    channels[0] = ((channels[0] as f32) * (1.0 - intensity) + overlay.0 as f32 * intensity) as u8;
    channels[1] = ((channels[1] as f32) * (1.0 - intensity) + overlay.1 as f32 * intensity) as u8;
    channels[2] = ((channels[2] as f32) * (1.0 - intensity) + overlay.2 as f32 * intensity) as u8;
}

/// Tint a pixmap's opaque pixels to `color`, preserving coverage (alpha).
fn colorize(pixmap: &mut Pixmap, color: &Color) {
    let (r, g, b, _) = color.to_rgba8();
    let data = pixmap.data_mut();
    for chunk in data.chunks_exact_mut(4) {
        let a = chunk[3];
        if a > 0 {
            let af = a as f32 / 255.0;
            chunk[0] = (r as f32 * af) as u8;
            chunk[1] = (g as f32 * af) as u8;
            chunk[2] = (b as f32 * af) as u8;
        }
    }
}

/// A two-pass box blur approximating a Gaussian blur.
pub fn box_blur(pixmap: &mut Pixmap, radius: f32) {
    if radius <= 0.0 {
        return;
    }
    let r = radius.ceil() as usize;
    if r == 0 {
        return;
    }
    let width = pixmap.width() as usize;
    let height = pixmap.height() as usize;
    if width == 0 || height == 0 {
        return;
    }
    let mut temp = vec![0u8; width * height * 4];
    {
        let src = pixmap.data();
        for y in 0..height {
            for x in 0..width {
                let mut sum = [0u32; 4];
                let mut count = 0u32;
                for dx in 0..=r * 2 {
                    let sx = (x + dx).saturating_sub(r);
                    if sx < width {
                        let idx = (y * width + sx) * 4;
                        for c in 0..4 {
                            sum[c] += src[idx + c] as u32;
                        }
                        count += 1;
                    }
                }
                let idx = (y * width + x) * 4;
                for c in 0..4 {
                    temp[idx + c] = (sum[c] / count) as u8;
                }
            }
        }
    }
    let dst = pixmap.data_mut();
    for y in 0..height {
        for x in 0..width {
            let mut sum = [0u32; 4];
            let mut count = 0u32;
            for dy in 0..=r * 2 {
                let sy = (y + dy).saturating_sub(r);
                if sy < height {
                    let idx = (sy * width + x) * 4;
                    for c in 0..4 {
                        sum[c] += temp[idx + c] as u32;
                    }
                    count += 1;
                }
            }
            let idx = (y * width + x) * 4;
            for c in 0..4 {
                dst[idx + c] = (sum[c] / count) as u8;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colorize_sets_premultiplied_color() {
        let mut pixmap = Pixmap::new(2, 2).unwrap();
        pixmap.fill(tiny_skia::Color::WHITE);
        colorize(&mut pixmap, &Color::RED);
        let data = pixmap.data();
        assert_eq!(data[0], 255);
        assert_eq!(data[1], 0);
        assert_eq!(data[2], 0);
    }

    #[test]
    fn blur_does_not_panic() {
        let mut pixmap = Pixmap::new(8, 8).unwrap();
        pixmap.fill(tiny_skia::Color::WHITE);
        box_blur(&mut pixmap, 2.0);
    }

    #[test]
    fn drop_shadow_adds_ink_behind() {
        let mut target = Pixmap::new(40, 40).unwrap();
        let mut layer = Pixmap::new(40, 40).unwrap();
        let rect = tiny_skia::Rect::from_xywh(8.0, 8.0, 10.0, 10.0).unwrap();
        let mut paint = tiny_skia::Paint::default();
        paint.set_color(tiny_skia::Color::from_rgba8(0, 0, 255, 255));
        layer.fill_path(
            &tiny_skia::PathBuilder::from_rect(rect),
            &paint,
            tiny_skia::FillRule::Winding,
            Transform::identity(),
            None,
        );
        let effects = vec![SceneEffect::DropShadow {
            color: Color::from_rgba8(0, 0, 0, 200),
            offset_x: 4.0,
            offset_y: 4.0,
            blur_radius: 1.0,
            spread: 0.0,
        }];
        composite_with_effects(&mut target, &layer, &effects, 1.0);
        // Pixel below/right of the shape should now have shadow ink.
        let idx = (20 * 40 + 20) * 4;
        assert!(target.data()[idx + 3] > 0);
    }

    #[test]
    fn soft_edges_fades_corner() {
        let mut target = Pixmap::new(20, 20).unwrap();
        let mut layer = Pixmap::new(20, 20).unwrap();
        layer.fill(tiny_skia::Color::WHITE);
        composite_with_effects(
            &mut target,
            &layer,
            &[SceneEffect::SoftEdges { radius: 5.0 }],
            1.0,
        );
        let corner = 0;
        assert!(target.data()[corner + 3] < 255);
    }
}
