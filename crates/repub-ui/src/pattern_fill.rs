//! Pattern and picture (texture) fills for the canvas.
//!
//! A pattern is an 8x8 two-colour tile (`PatternType::bits`). The flat fill is
//! tessellated as usual (keeping its anti-aliased edges), then each vertex gets
//! texture coordinates on a repeat-wrapped tile texture. Coordinates are the
//! shape's own unrotated frame, so patterns turn with the shape. One tile
//! pixel is one point (times the pattern's scale).
//!
//! Textures need the egui context, which drawing code doesn't carry, so the
//! canvas registers it each frame ([`set_context`]). Without one (unit tests),
//! or when tiles would be too small to see, the pattern is drawn as its
//! average colour.

use std::cell::RefCell;
use std::collections::HashMap;

use egui::{Color32, ColorImage, Mesh, Pos2, Shape, TextureHandle, TextureOptions};
use repub_geometry::Rect;
use repub_model::styles::{Fill, Pattern, TexturePattern, TileMode};
use repub_color::Color;

use crate::canvas::DocToScreen;

/// Tiles smaller than this on screen (points) are drawn as their average colour.
const MIN_TILE: f32 = 4.0;

type Key = ([u8; 8], [u8; 4], [u8; 4]);

struct Textures {
    ctx: egui::Context,
    tiles: HashMap<Key, TextureHandle>,
}

thread_local! {
    // Per thread: the UI thread's context (test harnesses each have their own)
    static TEXTURES: RefCell<Option<Textures>> = const { RefCell::new(None) };
}

/// Whether `fill` is drawn by this module.
pub fn is_pattern(fill: &Fill) -> bool {
    matches!(fill, Fill::Pattern(_))
}

/// Register the context pattern textures are made in (once per frame).
pub fn set_context(ctx: &egui::Context) {
    TEXTURES.with_borrow_mut(|textures| {
        if textures.as_ref().map_or(true, |t| t.ctx != *ctx) {
            // A new context: earlier textures belong to the old one
            *textures = Some(Textures {
                ctx: ctx.clone(),
                tiles: HashMap::new(),
            });
        }
    });
}

/// The pattern's colours mixed in proportion to its coverage.
pub fn average_color(pattern: &Pattern) -> Color {
    let on = pattern.pattern_type.bits().iter().map(|r| r.count_ones()).sum::<u32>() as f64 / 64.0;
    let (f, b) = (pattern.foreground, pattern.background);
    let mix = |x: f64, y: f64| x * on + y * (1.0 - on);
    Color::new(mix(f.r, b.r), mix(f.g, b.g), mix(f.b, b.b), mix(f.a, b.a))
}

fn rgba(c: &Color) -> [u8; 4] {
    [c.r, c.g, c.b, c.a].map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8)
}

/// The tile image: foreground where the bit is set, background elsewhere.
pub fn tile_image(pattern: &Pattern) -> ColorImage {
    let (fg, bg) = (rgba(&pattern.foreground), rgba(&pattern.background));
    let bits = pattern.pattern_type.bits();
    let mut pixels = Vec::with_capacity(64);
    for row in bits {
        for x in 0..8 {
            let [r, g, b, a] = if row & (0x80 >> x) != 0 { fg } else { bg };
            pixels.push(Color32::from_rgba_unmultiplied(r, g, b, a));
        }
    }
    ColorImage { size: [8, 8], pixels }
}

fn texture(pattern: &Pattern) -> Option<egui::TextureId> {
    TEXTURES.with_borrow_mut(|textures| {
        let textures = textures.as_mut()?;
        let key = (pattern.pattern_type.bits(), rgba(&pattern.foreground), rgba(&pattern.background));
        let ctx = textures.ctx.clone();
        let handle = textures.tiles.entry(key).or_insert_with(|| {
            let options = TextureOptions {
                magnification: egui::TextureFilter::Nearest,
                minification: egui::TextureFilter::Linear,
                wrap_mode: egui::TextureWrapMode::Repeat,
                mipmap_mode: None,
            };
            ctx.load_texture(format!("pattern {key:?}"), tile_image(pattern), options)
        });
        Some(handle.id())
    })
}

/// Tessellate `outline` (screen space, any flat colour) into one mesh whose
/// vertex alpha is anti-aliasing coverage.
fn coverage_mesh(outline: Vec<Shape>) -> Mesh {
    let mut tessellator = egui::epaint::Tessellator::new(1.0, Default::default(), [1, 1], Vec::new());
    let mut flat = Mesh::default();
    for shape in outline {
        tessellator.tessellate_shape(crate::gradient::recolor(shape), &mut flat);
    }
    flat
}

/// Screen point -> position in `frame`'s unrotated space, in screen units
/// from its top-left corner.
fn frame_space(frame: Rect, rotation: f64, map: DocToScreen) -> impl Fn(Pos2) -> (f32, f32) {
    let centre = map.point(frame.origin.x + frame.size.width / 2.0, frame.origin.y + frame.size.height / 2.0);
    let half = egui::Vec2::new(frame.size.width as f32, frame.size.height as f32) * map.scale / 2.0;
    let (sin, cos) = (-rotation as f32).to_radians().sin_cos();
    move |p: Pos2| {
        let (dx, dy) = (p.x - centre.x, p.y - centre.y);
        (dx * cos - dy * sin + half.x, dx * sin + dy * cos + half.y)
    }
}

/// Fill `outline` shapes with a picture: `texture` (`size` pixels; wrapping if
/// tiled) stretched over `frame` or tiled at its natural size (96 dpi) times
/// the fill's scale, per `fill.tile`.
#[allow(clippy::too_many_arguments)]
pub fn texture_fill_shapes(
    outline: Vec<Shape>,
    fill: &TexturePattern,
    texture: egui::TextureId,
    size: [usize; 2],
    frame: Rect,
    rotation: f64,
    map: DocToScreen,
    alpha: f32,
) -> Shape {
    let mut mesh = coverage_mesh(outline);
    let to_frame = frame_space(frame, rotation, map);
    let (fw, fh) = (
        (frame.size.width as f32 * map.scale).max(1e-3),
        (frame.size.height as f32 * map.scale).max(1e-3),
    );
    let tile_w = (size[0] as f32 * 0.75 * fill.scale_x.max(0.01) as f32 * map.scale).max(1e-3);
    let tile_h = (size[1] as f32 * 0.75 * fill.scale_y.max(0.01) as f32 * map.scale).max(1e-3);
    let (tile_x, tile_y) = match fill.tile {
        TileMode::Tile => (true, true),
        TileMode::TileHorizontal => (true, false),
        TileMode::TileVertical => (false, true),
        TileMode::Stretch | TileMode::Center => (false, false),
    };
    let opacity = alpha * (1.0 - fill.transparency.clamp(0.0, 1.0) as f32);
    mesh.texture_id = texture;
    for v in &mut mesh.vertices {
        let (x, y) = to_frame(v.pos);
        let u = if tile_x {
            x / tile_w
        } else if fill.tile == TileMode::Center {
            (x - (fw - tile_w) / 2.0) / tile_w
        } else {
            x / fw
        };
        let w = if tile_y {
            y / tile_h
        } else if fill.tile == TileMode::Center {
            (y - (fh - tile_h) / 2.0) / tile_h
        } else {
            y / fh
        };
        v.uv = Pos2::new(u + fill.offset_x as f32, w + fill.offset_y as f32);
        let coverage = v.color.a() as f32 / 255.0;
        v.color = Color32::from_white_alpha((coverage * opacity * 255.0).round() as u8);
    }
    Shape::mesh(mesh)
}

/// Fill `outline` shapes (screen space, any flat colour) with `pattern`.
/// `frame` is the object's bounds and `rotation` its rotation in degrees;
/// `alpha` fades the result.
pub fn fill_shapes(outline: Vec<Shape>, pattern: &Pattern, frame: Rect, rotation: f64, map: DocToScreen, alpha: f32) -> Shape {
    let tile = 8.0 * pattern.scale.max(0.01) as f32 * map.scale;
    let texture = if tile >= MIN_TILE { texture(pattern) } else { None };

    let mut flat = coverage_mesh(outline);

    let Some(texture) = texture else {
        let c = average_color(pattern);
        for v in &mut flat.vertices {
            let coverage = v.color.a() as f32 / 255.0;
            v.color = Color32::from_rgba_unmultiplied(
                (c.r * 255.0).round() as u8,
                (c.g * 255.0).round() as u8,
                (c.b * 255.0).round() as u8,
                (c.a as f32 * coverage * alpha * 255.0).round() as u8,
            );
        }
        return Shape::mesh(flat);
    };

    // Screen point -> tile coordinates in the shape's frame (undoing its
    // rotation, then applying the pattern's own)
    let to_frame = frame_space(frame, rotation, map);
    let (psin, pcos) = (-pattern.rotation as f32).to_radians().sin_cos();
    let to_uv = |p: Pos2| -> Pos2 {
        let (x, y) = to_frame(p);
        let (x, y) = (x * pcos - y * psin, x * psin + y * pcos);
        Pos2::new(x / tile, y / tile)
    };

    flat.texture_id = texture;
    for v in &mut flat.vertices {
        v.uv = to_uv(v.pos);
        let coverage = v.color.a() as f32 / 255.0;
        v.color = Color32::from_white_alpha((coverage * alpha * 255.0).round() as u8);
    }
    Shape::mesh(flat)
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_model::styles::PatternType;

    #[test]
    fn tiles_and_average_colour() {
        let p = Pattern::new(PatternType::LightVertical, Color::BLACK, Color::WHITE);
        let img = tile_image(&p);
        assert_eq!(img.pixels[0], Color32::BLACK);
        assert_eq!(img.pixels[1], Color32::WHITE);
        let avg = average_color(&p);
        assert!((avg.r - 0.75).abs() < 1e-9, "a quarter black: {avg:?}");
    }

    #[test]
    fn picture_fills_stretch_or_tile() {
        let map = DocToScreen {
            origin: Pos2::ZERO,
            scale: 1.0,
        };
        let frame = Rect::new(0.0, 0.0, 150.0, 75.0);
        let rect = || vec![Shape::rect_filled(map.rect(&frame), 0.0, Color32::GRAY)];
        let far = |shape: Shape| match shape {
            Shape::Mesh(m) => m
                .vertices
                .iter()
                .map(|v| v.uv)
                .fold(Pos2::ZERO, |a, b| Pos2::new(a.x.max(b.x), a.y.max(b.y))),
            _ => panic!("mesh"),
        };
        let id = egui::TextureId::User(7);
        let mut fill = TexturePattern::new("x.png");
        fill.tile = TileMode::Stretch;
        let uv = far(texture_fill_shapes(rect(), &fill, id, [100, 100], frame, 0.0, map, 1.0));
        assert!((uv.x - 1.0).abs() < 0.01 && (uv.y - 1.0).abs() < 0.01, "stretched: {uv:?}");
        fill.tile = TileMode::Tile; // 100 px = 75 pt tiles: 2 x 1
        let uv = far(texture_fill_shapes(rect(), &fill, id, [100, 100], frame, 0.0, map, 1.0));
        assert!((uv.x - 2.0).abs() < 0.02 && (uv.y - 1.0).abs() < 0.02, "tiled: {uv:?}");
    }

    #[test]
    fn mesh_is_textured_in_the_shapes_frame() {
        let ctx = egui::Context::default();
        let _ = ctx.run(Default::default(), |_| {});
        set_context(&ctx);
        let p = Pattern::new(PatternType::Cross, Color::BLACK, Color::WHITE);
        let map = DocToScreen {
            origin: Pos2::new(50.0, 0.0),
            scale: 2.0,
        };
        let frame = Rect::new(10.0, 10.0, 40.0, 16.0);
        let outline = Shape::rect_filled(map.rect(&frame), 0.0, Color32::GRAY);
        let Shape::Mesh(mesh) = fill_shapes(vec![outline], &p, frame, 0.0, map, 1.0) else {
            panic!("mesh")
        };
        assert_ne!(mesh.texture_id, egui::TextureId::default());
        // The frame's far corner is 5 x 2 tiles from its origin
        let far = mesh
            .vertices
            .iter()
            .map(|v| v.uv)
            .fold(Pos2::ZERO, |a, b| Pos2::new(a.x.max(b.x), a.y.max(b.y)));
        assert!((far.x - 5.0).abs() < 0.1 && (far.y - 2.0).abs() < 0.1, "{far:?}");
    }
}
