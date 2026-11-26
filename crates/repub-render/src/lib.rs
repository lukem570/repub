//! # Repub Render
//!
//! The thin backend-abstraction layer: a [`Renderer`] trait that executes a
//! [`Scene`](repub_scene::Scene), and shared [`RenderOptions`].
//!
//! This crate defines *how* a scene is drawn, not *what* is drawn. A CPU
//! rasterizer, a GPU backend, or a PDF writer each implement [`Renderer`] to
//! realize the scene's primitives and their [`SceneEffect`](repub_scene::SceneEffect)
//! descriptors.
//!
//! It depends on [`repub_scene`] for the [`Scene`](repub_scene::Scene) type.
//! The dependency runs one way — render builds on scene — so there is no cycle.

#![warn(missing_docs)]
#![warn(clippy::all)]

use repub_color::Color;
use repub_scene::Scene;

/// Options shared by renderers: output scale, background and antialiasing.
///
/// A renderer may use these to size its output surface and decide how finely to
/// rasterize. They are advisory; backends that cannot honor a field ignore it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderOptions {
    /// Output resolution in dots per inch (72.0 = 1 device pixel per point).
    pub dpi: f64,
    /// Additional scale factor applied on top of [`RenderOptions::dpi`].
    pub scale: f64,
    /// Background color drawn before the scene, if the scene has none.
    pub background: Option<Color>,
    /// Whether to antialias edges.
    pub antialias: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            dpi: 72.0,
            scale: 1.0,
            background: None,
            antialias: true,
        }
    }
}

impl RenderOptions {
    /// Create options at the given DPI with defaults elsewhere.
    pub fn at_dpi(dpi: f64) -> Self {
        Self {
            dpi,
            ..Self::default()
        }
    }

    /// The device-pixels-per-point factor implied by DPI and scale.
    pub fn pixels_per_point(&self) -> f64 {
        (self.dpi / 72.0) * self.scale
    }

    /// Set the background color.
    pub fn with_background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    /// Set the antialias flag.
    pub fn with_antialias(mut self, antialias: bool) -> Self {
        self.antialias = antialias;
        self
    }
}

/// A backend that executes a [`Scene`], drawing its primitives.
///
/// Implementations draw each [`Primitive`](repub_scene::Primitive) in order,
/// honoring its transform and clip, and realize the attached
/// [`SceneEffect`](repub_scene::SceneEffect)s as they are able. The trait is
/// deliberately small: the scene carries everything needed to draw a frame.
pub trait Renderer {
    /// Draw the scene. Primitives are executed in the scene's back-to-front
    /// order.
    fn render_scene(&mut self, scene: &Scene);

    /// The options this renderer draws with. The default returns
    /// [`RenderOptions::default`]; backends that carry options override this.
    fn options(&self) -> RenderOptions {
        RenderOptions::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct CountingRenderer {
        drawn: usize,
    }

    impl Renderer for CountingRenderer {
        fn render_scene(&mut self, scene: &Scene) {
            self.drawn = scene.primitives.len();
        }
    }

    #[test]
    fn renders_a_scene() {
        let scene = Scene::new(repub_geometry::Size::new(100.0, 100.0));
        let mut renderer = CountingRenderer { drawn: 99 };
        renderer.render_scene(&scene);
        assert_eq!(renderer.drawn, 0);
    }

    #[test]
    fn default_options() {
        let opts = RenderOptions::default();
        assert_eq!(opts.dpi, 72.0);
        assert!((opts.pixels_per_point() - 1.0).abs() < 1e-9);
        assert_eq!(RenderOptions::at_dpi(144.0).pixels_per_point(), 2.0);
    }
}
