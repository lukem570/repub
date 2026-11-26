//! # Repub GPU
//!
//! A windowless [wgpu]-based renderer that executes a [`repub_scene::Scene`]
//! into an RGBA8 pixel buffer, implementing the [`repub_render::Renderer`]
//! trait.
//!
//! This is Layer 4 of the layered rewrite, the GPU sibling of the
//! [`repub_raster`](https://docs.rs/repub-raster) CPU backend. Like it, this
//! crate is a **dumb executor** of an already-resolved scene: it knows nothing
//! about the document model, layout, WordArt or text-on-path. All geometry is
//! resolved into concrete paths and glyph runs by `compose`; this crate walks
//! [`scene.primitives`](repub_scene::Scene::primitives) in back-to-front order
//! and draws each with wgpu.
//!
//! ## Headless by design
//!
//! There is no window, [`Surface`](wgpu::Surface) or pan/zoom: those belong to
//! the GUI. The context requests an adapter with no compatible surface, renders
//! to an offscreen texture sized to the scene (× device scale), and copies it
//! back to pixels. See [`GpuRenderer::render_to_pixels`].
//!
//! ## What is realized
//!
//! - **Fills and strokes** — tessellated with [lyon], honoring the scene fill
//!   rule (nonzero / even-odd) and stroke width, cap and join. See
//!   [`tessellation`]. Stroke **dash** patterns are not yet applied.
//! - **Paint** — solid colors and linear/radial gradients, baked per-vertex;
//!   pattern fills approximated by their foreground color. See [`paint`].
//! - **Glyph runs** — resolved against the installed-font registry (bundled
//!   fallback) and rendered by **outline tessellation** (ttf-parser outlines →
//!   lyon fill). The GUI's SDF atlas is **deferred**; see [`text`].
//! - **Images** — `File` and base64 `Embedded` sources decoded and drawn as a
//!   textured quad; `Url` is never fetched and draws a placeholder. See
//!   [`image`] and [`texture`].
//! - **Effects** — only an unblurred drop-shadow approximation is realized; all
//!   other [`SceneEffect`](repub_scene::SceneEffect)s are deferred (drawn as if
//!   absent). See [`effects`].
//! - **Clips** — axis-aligned clip rectangles are applied with the scissor
//!   rect.
//!
//! [wgpu]: https://docs.rs/wgpu
//! [lyon]: https://docs.rs/lyon

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod context;
pub mod effects;
pub mod error;
pub mod image;
pub mod paint;
pub mod renderer;
pub mod tessellation;
pub mod text;
pub mod texture;
pub mod vertex;

pub use context::GpuContext;
pub use error::{GpuError, GpuResult};
pub use renderer::{GpuRenderer, RenderedImage};

#[cfg(test)]
mod tests {
    use super::*;
    use repub_color::Color;
    use repub_geometry::{Path, Point, Rect, Size, Transform2D};
    use repub_render::{RenderOptions, Renderer};
    use repub_scene::{FillRule, LineCap, LineJoin, Paint, Primitive, Scene};

    /// Try to build a renderer; return `None` (skip) when no GPU adapter is
    /// available, as on headless CI.
    fn try_renderer() -> Option<GpuRenderer> {
        match GpuRenderer::new_blocking() {
            Ok(r) => Some(r),
            Err(GpuError::NoAdapter) => None,
            Err(e) => panic!("unexpected GPU init error: {e}"),
        }
    }

    fn px(pixels: &[u8], width: u32, x: u32, y: u32) -> (u8, u8, u8, u8) {
        let i = ((y * width + x) * 4) as usize;
        (pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3])
    }

    #[test]
    fn red_fill_rect_over_background() {
        let Some(mut r) = try_renderer() else {
            return;
        };
        let mut scene = Scene::new(Size::new(40.0, 40.0));
        scene.background = Some(Paint::Solid(Color::from_rgb8(0, 0, 255)));
        scene.push(Primitive::FillPath {
            path: Path::from_rect(Rect::new(10.0, 10.0, 20.0, 20.0)),
            paint: Paint::Solid(Color::RED),
            fill_rule: FillRule::NonZero,
            transform: Transform2D::IDENTITY,
            clip: None,
            effects: vec![],
        });

        let pixels = r.render_to_pixels(&scene).unwrap();
        assert_eq!(pixels.len(), 40 * 40 * 4);

        // Center is red.
        let (cr, cg, cb, ca) = px(&pixels, 40, 20, 20);
        assert!(cr > 200 && cg < 60 && cb < 60 && ca > 200, "center {:?}", (cr, cg, cb, ca));

        // A corner is the blue background.
        let (br, bg, bb, ba) = px(&pixels, 40, 2, 2);
        assert!(bb > 200 && br < 60 && bg < 60 && ba > 200, "corner {:?}", (br, bg, bb, ba));
    }

    #[test]
    fn stroke_path_draws_a_line() {
        let Some(mut r) = try_renderer() else {
            return;
        };
        let mut scene = Scene::new(Size::new(40.0, 40.0));
        let mut path = Path::new();
        path.move_to(Point::new(5.0, 20.0))
            .line_to(Point::new(35.0, 20.0));
        scene.push(Primitive::StrokePath {
            path,
            paint: Paint::Solid(Color::from_rgb8(0, 0, 0)),
            width: 4.0,
            cap: LineCap::Butt,
            join: LineJoin::Miter,
            dash: None,
            transform: Transform2D::IDENTITY,
            clip: None,
            effects: vec![],
        });

        let pixels = r.render_to_pixels(&scene).unwrap();
        // On the line (y=20) there is ink.
        assert!(px(&pixels, 40, 20, 20).3 > 0, "line should be inked");
        // Well above the line is empty.
        assert_eq!(px(&pixels, 40, 20, 4).3, 0, "above line should be empty");
    }
}
