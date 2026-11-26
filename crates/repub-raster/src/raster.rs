//! The CPU rasterizer: executes a [`Scene`] into a tiny-skia pixmap.

use repub_geometry::Rect;
use repub_render::{RenderOptions, Renderer};
use repub_scene::{FillRule, LineCap, LineJoin, Paint as ScenePaint, Primitive, Scene};
use tiny_skia::{
    FillRule as SkFillRule, LineCap as SkLineCap, LineJoin as SkLineJoin, Mask, Pixmap, Stroke,
    StrokeDash, Transform,
};

use crate::effects::composite_with_effects;
use crate::glyph::GlyphRenderer;
use crate::paint::{build_paint, convert_color};
use crate::path::{convert_path, convert_transform, transform_scale};

/// A CPU rasterizer that draws a [`Scene`] into an RGBA pixel buffer.
///
/// The rasterizer is a dumb executor: it walks the scene's already-resolved
/// primitives in back-to-front order and draws each with tiny-skia. It has no
/// knowledge of the document model, layout, or formats.
pub struct CpuRasterizer {
    pixmap: Pixmap,
    options: RenderOptions,
    glyphs: GlyphRenderer,
}

impl CpuRasterizer {
    /// Create a rasterizer with a blank pixmap of the given pixel size.
    ///
    /// Returns `None` if the dimensions are zero or too large to allocate.
    pub fn new(width: u32, height: u32) -> Option<Self> {
        Some(Self {
            pixmap: Pixmap::new(width, height)?,
            options: RenderOptions::default(),
            glyphs: GlyphRenderer::new(),
        })
    }

    /// Set the render options the rasterizer draws with.
    pub fn with_options(mut self, options: RenderOptions) -> Self {
        self.options = options;
        self
    }

    /// Create a rasterizer sized for a scene at the given options, render the
    /// scene into it, and return the resulting pixmap.
    ///
    /// The pixmap size is the scene's point size scaled by
    /// [`RenderOptions::pixels_per_point`]. Returns `None` if the pixmap cannot
    /// be allocated.
    pub fn from_scene(scene: &Scene, options: &RenderOptions) -> Option<Pixmap> {
        let ppp = options.pixels_per_point();
        let width = (scene.size.width * ppp).ceil().max(1.0) as u32;
        let height = (scene.size.height * ppp).ceil().max(1.0) as u32;
        let mut rasterizer = Self::new(width, height)?.with_options(*options);
        rasterizer.render_scene(scene);
        Some(rasterizer.into_pixmap())
    }

    /// A shared reference to the rendered pixmap.
    pub fn pixmap(&self) -> &Pixmap {
        &self.pixmap
    }

    /// A mutable reference to the pixmap.
    pub fn pixmap_mut(&mut self) -> &mut Pixmap {
        &mut self.pixmap
    }

    /// Consume the rasterizer, returning the pixmap.
    pub fn into_pixmap(self) -> Pixmap {
        self.pixmap
    }

    fn device_scale(&self) -> Transform {
        let ppp = self.options.pixels_per_point() as f32;
        Transform::from_scale(ppp, ppp)
    }

    fn paint_background(&mut self, scene: &Scene) {
        let color = match &scene.background {
            Some(ScenePaint::Solid(c)) => Some(*c),
            Some(other) => Some(other.primary_color()),
            None => self.options.background,
        };
        if let Some(color) = color {
            self.pixmap.fill(convert_color(&color));
        }
    }

    fn clip_mask(&self, clip: Option<Rect>) -> Option<Mask> {
        let clip = clip?;
        let ppp = self.options.pixels_per_point() as f32;
        let rect = tiny_skia::Rect::from_xywh(
            clip.x() as f32 * ppp,
            clip.y() as f32 * ppp,
            clip.width() as f32 * ppp,
            clip.height() as f32 * ppp,
        )?;
        let mut mask = Mask::new(self.pixmap.width(), self.pixmap.height())?;
        let path = tiny_skia::PathBuilder::from_rect(rect);
        mask.fill_path(&path, SkFillRule::Winding, true, Transform::identity());
        Some(mask)
    }

    fn draw_primitive(&mut self, prim: &Primitive) {
        let device = self.device_scale();
        let mask = self.clip_mask(prim.clip());
        let anti_alias = self.options.antialias;
        let effects = prim.effects();

        if effects.is_empty() {
            draw_primitive_onto(
                &mut self.pixmap,
                prim,
                device,
                mask.as_ref(),
                anti_alias,
                &mut self.glyphs,
            );
        } else {
            let Some(mut layer) = Pixmap::new(self.pixmap.width(), self.pixmap.height()) else {
                return;
            };
            draw_primitive_onto(
                &mut layer,
                prim,
                device,
                mask.as_ref(),
                anti_alias,
                &mut self.glyphs,
            );
            let scale = transform_scale(&device);
            composite_with_effects(&mut self.pixmap, &layer, effects, scale);
        }
    }
}

impl Renderer for CpuRasterizer {
    fn render_scene(&mut self, scene: &Scene) {
        self.paint_background(scene);
        for prim in &scene.primitives {
            self.draw_primitive(prim);
        }
    }

    fn options(&self) -> RenderOptions {
        self.options
    }
}

/// Draw a single primitive onto `target` under `device` scale.
///
/// Free function so the pixmap, clip mask and glyph renderer can be borrowed
/// independently from the rasterizer.
fn draw_primitive_onto(
    target: &mut Pixmap,
    prim: &Primitive,
    device: Transform,
    mask: Option<&Mask>,
    anti_alias: bool,
    glyphs: &mut GlyphRenderer,
) {
    let transform = device.pre_concat(convert_transform(&prim.transform()));

    match prim {
        Primitive::FillPath {
            path,
            paint,
            fill_rule,
            ..
        } => {
            let Some(sk_path) = convert_path(path) else {
                return;
            };
            let fill_paint = build_paint(paint, anti_alias, 1.0);
            target.fill_path(
                &sk_path,
                &fill_paint,
                convert_fill_rule(*fill_rule),
                transform,
                mask,
            );
        }
        Primitive::StrokePath {
            path,
            paint,
            width,
            cap,
            join,
            dash,
            ..
        } => {
            let Some(sk_path) = convert_path(path) else {
                return;
            };
            let stroke_paint = build_paint(paint, anti_alias, 1.0);
            let mut stroke = Stroke {
                width: *width as f32,
                line_cap: convert_cap(*cap),
                line_join: convert_join(*join),
                ..Default::default()
            };
            if let Some(pattern) = dash {
                let dashes: Vec<f32> = pattern.iter().map(|d| *d as f32).collect();
                stroke.dash = StrokeDash::new(dashes, 0.0);
            }
            target.stroke_path(&sk_path, &stroke_paint, &stroke, transform, mask);
        }
        Primitive::Glyphs {
            font,
            size,
            glyphs: run,
            paint,
            ..
        } => {
            // Glyphs are composited via draw_pixmap, which does not honor the
            // tiny-skia clip mask; they are bounded by the pixmap edges.
            glyphs.draw_run(target, font, *size, run, paint, &transform);
        }
        Primitive::Image {
            source,
            dest,
            crop,
            opacity,
            ..
        } => {
            crate::image::draw_image(target, source, dest, crop.as_ref(), *opacity, &transform);
        }
    }
}

fn convert_fill_rule(rule: FillRule) -> SkFillRule {
    match rule {
        FillRule::NonZero => SkFillRule::Winding,
        FillRule::EvenOdd => SkFillRule::EvenOdd,
    }
}

fn convert_cap(cap: LineCap) -> SkLineCap {
    match cap {
        LineCap::Butt => SkLineCap::Butt,
        LineCap::Round => SkLineCap::Round,
        LineCap::Square => SkLineCap::Square,
    }
}

fn convert_join(join: LineJoin) -> SkLineJoin {
    match join {
        LineJoin::Miter => SkLineJoin::Miter,
        LineJoin::Round => SkLineJoin::Round,
        LineJoin::Bevel => SkLineJoin::Bevel,
    }
}
