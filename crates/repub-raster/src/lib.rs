//! # Repub Raster
//!
//! A CPU rasterizer that executes a [`repub_scene::Scene`] into an RGBA pixel
//! buffer with [tiny-skia], implementing the [`repub_render::Renderer`] trait.
//!
//! This is Layer 4 of the layered rewrite: a **dumb executor** of an already
//! resolved scene. It knows nothing about the document model, layout, WordArt,
//! text-on-path, or file formats — all geometry is resolved into concrete paths
//! and glyph runs by [`repub_scene`]'s `compose`. This crate simply walks
//! [`scene.primitives`](repub_scene::Scene::primitives) in back-to-front order
//! and draws each one.
//!
//! ## What is realized
//!
//! - **Fills and strokes** — paths converted to tiny-skia and filled/stroked,
//!   honoring fill rule, line cap/join and dash patterns.
//! - **Paint** — solid colors and linear/radial gradients (mapped onto the
//!   paint's resolved on-page bounds). Pattern fills are **approximated** by
//!   their foreground color; see [`paint`].
//! - **Glyph runs** — resolved against the installed-font registry (with a
//!   bundled fallback) and rasterized with [`fontdue`]; see [`glyph`].
//! - **Images** — `File` and base64 `Embedded` sources are decoded; `Url` is
//!   never fetched and `Placeholder` draws a placeholder box; see [`image`].
//! - **Effects** — drop shadow, outer glow, inner shadow, reflection and soft
//!   edges are realized; inner glow and bevel are approximated; see [`effects`].
//!
//! ## Exporting
//!
//! The rendered [`tiny_skia::Pixmap`] can be written to PNG/JPEG/TIFF/WebP/BMP
//! with the [`export`] module.
//!
//! [tiny-skia]: https://docs.rs/tiny-skia

#![warn(missing_docs)]

pub mod effects;
pub mod error;
pub mod export;
pub mod glyph;
pub mod glyph_cache;
pub mod image;
pub mod paint;
pub mod path;
pub mod raster;

pub use error::{RasterError, RasterResult};
pub use export::{export_png, export_to_bytes, export_to_file, ExportFormat, ExportOptions};
pub use raster::CpuRasterizer;

#[cfg(test)]
mod tests {
    use super::*;
    use repub_color::Color;
    use repub_geometry::{Path, Point, Size, Transform2D};
    use repub_render::{RenderOptions, Renderer};
    use repub_scene::{
        FillRule, FontRef, LineCap, LineJoin, Paint as ScenePaint, PositionedGlyph, Primitive,
        Scene,
    };

    fn px(pixmap: &tiny_skia::Pixmap, x: u32, y: u32) -> (u8, u8, u8, u8) {
        let idx = ((y * pixmap.width() + x) * 4) as usize;
        let d = pixmap.data();
        (d[idx], d[idx + 1], d[idx + 2], d[idx + 3])
    }

    #[test]
    fn background_paints() {
        let mut scene = Scene::new(Size::new(20.0, 20.0));
        scene.background = Some(ScenePaint::Solid(Color::from_rgb8(10, 20, 30)));
        let pixmap = CpuRasterizer::from_scene(&scene, &RenderOptions::default()).unwrap();
        assert_eq!(px(&pixmap, 5, 5), (10, 20, 30, 255));
    }

    #[test]
    fn red_fill_rect_fills_region() {
        let mut scene = Scene::new(Size::new(40.0, 40.0));
        let path = Path::from_rect(repub_geometry::Rect::new(10.0, 10.0, 20.0, 20.0));
        scene.push(Primitive::FillPath {
            path,
            paint: ScenePaint::Solid(Color::RED),
            fill_rule: FillRule::NonZero,
            transform: Transform2D::IDENTITY,
            clip: None,
            effects: vec![],
        });
        let pixmap = CpuRasterizer::from_scene(&scene, &RenderOptions::default()).unwrap();
        // Inside the rect is red.
        assert_eq!(px(&pixmap, 20, 20), (255, 0, 0, 255));
        // Outside (top-left corner) is untouched/transparent.
        assert_eq!(px(&pixmap, 2, 2).3, 0);
    }

    #[test]
    fn stroke_path_draws_a_line() {
        let mut scene = Scene::new(Size::new(40.0, 40.0));
        let mut path = Path::new();
        path.move_to(Point::new(5.0, 20.0))
            .line_to(Point::new(35.0, 20.0));
        scene.push(Primitive::StrokePath {
            path,
            paint: ScenePaint::Solid(Color::BLACK),
            width: 3.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            dash: None,
            transform: Transform2D::IDENTITY,
            clip: None,
            effects: vec![],
        });
        let pixmap = CpuRasterizer::from_scene(&scene, &RenderOptions::default()).unwrap();
        // The line runs along y=20; a pixel on it should have ink.
        assert!(px(&pixmap, 20, 20).3 > 0);
        // Well above the line should be empty.
        assert_eq!(px(&pixmap, 20, 5).3, 0);
    }

    #[test]
    fn glyph_run_produces_ink() {
        let mut scene = Scene::new(Size::new(80.0, 40.0));
        let glyphs = vec![
            PositionedGlyph {
                ch: 'R',
                glyph_id: 0,
                x: 10.0,
                y: 30.0,
                advance: 12.0,
            },
            PositionedGlyph {
                ch: 'e',
                glyph_id: 0,
                x: 22.0,
                y: 30.0,
                advance: 10.0,
            },
        ];
        scene.push(Primitive::Glyphs {
            font: FontRef::new("Selawik", 400, false),
            size: 24.0,
            glyphs,
            paint: ScenePaint::Solid(Color::BLACK),
            transform: Transform2D::IDENTITY,
            clip: None,
            effects: vec![],
        });
        let pixmap = CpuRasterizer::from_scene(&scene, &RenderOptions::default()).unwrap();
        let ink: usize = pixmap.data().chunks(4).filter(|p| p[3] > 0).count();
        assert!(ink > 0, "glyph run should produce non-empty ink");
    }

    #[test]
    fn export_png_has_valid_header() {
        let mut scene = Scene::new(Size::new(10.0, 10.0));
        scene.background = Some(ScenePaint::Solid(Color::WHITE));
        let pixmap = CpuRasterizer::from_scene(&scene, &RenderOptions::default()).unwrap();
        let png = export_png(&pixmap).unwrap();
        assert_eq!(
            &png[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]
        );
    }

    #[test]
    fn scale_grows_pixmap() {
        let scene = Scene::new(Size::new(10.0, 10.0));
        let opts = RenderOptions::at_dpi(144.0); // 2x
        let pixmap = CpuRasterizer::from_scene(&scene, &opts).unwrap();
        assert_eq!(pixmap.width(), 20);
        assert_eq!(pixmap.height(), 20);
    }

    #[test]
    fn renderer_trait_reports_options() {
        let r = CpuRasterizer::new(10, 10)
            .unwrap()
            .with_options(RenderOptions::at_dpi(216.0));
        assert_eq!(r.options().dpi, 216.0);
    }
}
