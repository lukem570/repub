//! Composing a document page into a flat [`Scene`].
//!
//! [`compose`] walks a page's layers and shapes in z-order and resolves each
//! [`Shape`] into one or more [`Primitive`]s. Geometry is fully resolved:
//! auto-shape presets become paths (via [`repub_escher`]), text frames and
//! tables are laid out (via [`repub_layout`]) into positioned glyph runs
//! and cell fills/borders, and WordArt is warped into glyph outline paths (via
//! [`crate::wordart`]). Shadow/glow/reflection/bevel/soft-edge effects are
//! carried as [`SceneEffect`] descriptors, not pre-baked.

use repub_color::{Color, ColorScheme};
use repub_geometry::{Path as GeomPath, Point, Rect, Size, Transform2D};
use repub_layout::{TableLayouter, TextLayouter};
use repub_model::shape::{
    LineData, LineEnding, LineEndingStyle, Shape, ShapeKind, TableData, TextFrameData,
};
use repub_model::styles::{Effects, FontStyle, Stroke};
use repub_model::{Document, Page};

use repub_escher::PresetExt;
use repub_fonts::FontProvider;

use crate::convert::{fill_to_paint, model_path_to_geom, paint_with_opacity, stroke_paint};
use crate::primitive::{
    FillRule, FontRef, LineCap, LineJoin, Paint, PositionedGlyph, Primitive, Scene, SceneEffect,
};

/// Font bytes bundled for WordArt glyph-outline extraction when no installed
/// face is found.
static FALLBACK_FONT: &[u8] = include_bytes!("../../../assets/fonts/fallback.ttf");

/// Compose page `page` of `doc` into a fresh [`Scene`], resolving scheme
/// colors against [`ColorScheme::OFFICE`].
///
/// This is the convenience entry point. Use [`compose_with_scheme`] to supply a
/// specific palette.
pub fn compose(doc: &Document, page: usize, fonts: &dyn FontProvider) -> Scene {
    compose_with_scheme(doc, page, fonts, &ColorScheme::OFFICE)
}

/// Compose page `page` of `doc` into a fresh [`Scene`] using `scheme` to
/// resolve scheme-color references.
///
/// Returns an empty scene sized to the document's default page if the index is
/// out of range.
pub fn compose_with_scheme(
    doc: &Document,
    page: usize,
    fonts: &dyn FontProvider,
    scheme: &ColorScheme,
) -> Scene {
    let Some(pg) = doc.page(page) else {
        return Scene::new(doc.default_page_size.to_points());
    };

    let mut scene = Scene::new(pg.size);
    if let Some(bg) = &pg.background {
        let rect = Rect::new(0.0, 0.0, pg.size.width, pg.size.height);
        scene.background = fill_to_paint(bg, rect, scheme);
    }

    let ctx = Ctx { fonts, scheme };
    // Draw master-page layers below the page's own, as the document defines.
    for layer in doc.page_layers(pg) {
        if !layer.visible {
            continue;
        }
        for shape in layer.shapes() {
            compose_shape(&ctx, shape, Transform2D::IDENTITY, &mut scene.primitives);
        }
    }
    scene
}

/// Shared context passed down the shape recursion.
struct Ctx<'a> {
    fonts: &'a dyn FontProvider,
    scheme: &'a ColorScheme,
}

fn compose_shape(ctx: &Ctx<'_>, shape: &Shape, parent: Transform2D, out: &mut Vec<Primitive>) {
    if !shape.common.visible {
        return;
    }
    let transform = parent.concat(&shape.common.full_transform());
    let bounds = shape.common.bounds;
    let effects = map_effects(&shape.effects, ctx.scheme);

    match &shape.kind {
        ShapeKind::AutoShape(auto) => {
            compose_auto_shape(ctx, shape, transform, &effects, out);
            let _ = auto;
        }
        ShapeKind::TextFrame(data) => {
            compose_text_frame(ctx, shape, data, transform, &effects, out);
        }
        ShapeKind::Table(data) => {
            compose_table(ctx, shape, data, transform, out);
        }
        ShapeKind::Picture(data) => {
            out.push(Primitive::Image {
                source: (&data.source).into(),
                dest: bounds,
                crop: data.crop,
                opacity: shape.opacity,
                transform,
                clip: Some(bounds),
                effects,
            });
        }
        ShapeKind::TextEffect(data) => {
            let font_bytes = resolve_font_bytes(&data.style.font);
            crate::wordart::resolve_text_effect(
                data,
                bounds,
                transform,
                None,
                &shape.fill,
                shape.line.as_ref(),
                effects,
                shape.opacity,
                ctx.scheme,
                &font_bytes,
                out,
            );
        }
        ShapeKind::Line(data) => {
            compose_line(shape, data, transform, &effects, out);
        }
        ShapeKind::Group(group) => {
            for child in &group.children {
                compose_shape(ctx, child, transform, out);
            }
        }
    }
}

fn compose_auto_shape(
    ctx: &Ctx<'_>,
    shape: &Shape,
    transform: Transform2D,
    effects: &[SceneEffect],
    out: &mut Vec<Primitive>,
) {
    let bounds = shape.common.bounds;
    let ShapeKind::AutoShape(auto) = &shape.kind else {
        return;
    };

    // Prefer an Office preset outline; otherwise the shape's own geometry.
    let (geom_path, fill_rule) = if let Some(preset) = &auto.preset {
        if let Some(model_path) = preset.path(bounds) {
            let rule = crate::convert::model_path_fill_rule(&model_path);
            (model_path_to_geom(&model_path), rule)
        } else {
            (
                model_path_to_geom(&auto.to_editable_path(bounds)),
                FillRule::NonZero,
            )
        }
    } else {
        let model_path = auto.to_editable_path(bounds);
        let rule = crate::convert::model_path_fill_rule(&model_path);
        (model_path_to_geom(&model_path), rule)
    };

    if geom_path.is_empty() {
        return;
    }

    if let Some(paint) = fill_to_paint(&shape.fill, bounds, ctx.scheme) {
        out.push(Primitive::FillPath {
            path: geom_path.clone(),
            paint: paint_with_opacity(paint, shape.opacity),
            fill_rule,
            transform,
            clip: None,
            effects: effects.to_vec(),
        });
    }
    if let Some(stroke) = &shape.line {
        out.push(stroke_primitive(
            &geom_path,
            stroke,
            transform,
            None,
            Vec::new(),
        ));
    }
}

fn compose_text_frame(
    ctx: &Ctx<'_>,
    shape: &Shape,
    data: &TextFrameData,
    transform: Transform2D,
    effects: &[SceneEffect],
    out: &mut Vec<Primitive>,
) {
    let bounds = shape.common.bounds;

    // Background fill and border come from the enclosing shape.
    if let Some(paint) = fill_to_paint(&shape.fill, bounds, ctx.scheme) {
        out.push(Primitive::FillPath {
            path: GeomPath::from_rect(bounds),
            paint: paint_with_opacity(paint, shape.opacity),
            fill_rule: FillRule::NonZero,
            transform,
            clip: None,
            effects: effects.to_vec(),
        });
    }
    if let Some(stroke) = &shape.line {
        out.push(stroke_primitive(
            &GeomPath::from_rect(bounds),
            stroke,
            transform,
            None,
            Vec::new(),
        ));
    }

    let layouter = TextLayouter::new(ctx.fonts);
    let Some(layout) = layouter.layout_shape(shape) else {
        return;
    };
    let clip = Some(bounds);
    emit_text_layout(&layout, transform, clip, out);
    let _ = data;
}

/// Emit glyph primitives for a laid-out text frame.
fn emit_text_layout(
    layout: &repub_layout::TextFrameLayout,
    transform: Transform2D,
    clip: Option<Rect>,
    out: &mut Vec<Primitive>,
) {
    for column in &layout.columns {
        let col_x = column.bounds.x();
        let col_y = column.bounds.y();
        for line in &column.lines {
            let baseline = col_y + line.baseline_y;
            for run in &line.runs {
                if run.text.trim().is_empty() {
                    continue;
                }
                let style = &run.style;
                let font = FontRef::new(
                    style.font.family.clone(),
                    style.font.weight.weight(),
                    matches!(style.font.style, FontStyle::Italic | FontStyle::Oblique),
                );
                let glyphs = position_run_glyphs(run, col_x, baseline);
                if glyphs.is_empty() {
                    continue;
                }
                out.push(Primitive::Glyphs {
                    font,
                    size: style.font_size,
                    glyphs,
                    paint: Paint::Solid(style.color),
                    transform,
                    clip,
                    effects: Vec::new(),
                });
            }
        }
    }
}

/// Position each glyph of a run along its baseline. When the layouter already
/// positioned glyphs, use those; otherwise advance by each character's width.
fn position_run_glyphs(
    run: &repub_layout::LayoutRun,
    col_x: f64,
    baseline: f64,
) -> Vec<PositionedGlyph> {
    if !run.glyphs.is_empty() {
        return run
            .glyphs
            .iter()
            .map(|g| PositionedGlyph {
                ch: g.char,
                glyph_id: 0,
                x: col_x + run.x + g.x,
                y: baseline,
                advance: g.advance,
            })
            .collect();
    }
    // Fall back to even spacing across the run width.
    let chars: Vec<char> = run.text.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }
    let step = run.width / chars.len() as f64;
    let mut x = col_x + run.x;
    let mut glyphs = Vec::with_capacity(chars.len());
    for ch in chars {
        glyphs.push(PositionedGlyph {
            ch,
            glyph_id: 0,
            x,
            y: baseline,
            advance: step,
        });
        x += step;
    }
    glyphs
}

fn compose_table(
    ctx: &Ctx<'_>,
    shape: &Shape,
    data: &TableData,
    transform: Transform2D,
    out: &mut Vec<Primitive>,
) {
    let layouter = TableLayouter::new();
    let Some(layout) = layouter.layout_shape(shape) else {
        return;
    };

    // Per-cell background fills.
    for cell_layout in &layout.cells {
        if let Some(cell) = data.cell(cell_layout.row as u32, cell_layout.col as u32) {
            if let Some(paint) = fill_to_paint(&cell.fill, cell_layout.bounds, ctx.scheme) {
                out.push(Primitive::FillPath {
                    path: GeomPath::from_rect(cell_layout.bounds),
                    paint,
                    fill_rule: FillRule::NonZero,
                    transform,
                    clip: None,
                    effects: Vec::new(),
                });
            }
        }
    }

    // Borders: outer edges plus inner grid lines, from TableBorders.
    emit_table_borders(data, &layout, transform, out);

    // Cell text, laid out within each cell's content bounds.
    for cell_layout in &layout.cells {
        let Some(cell) = data.cell(cell_layout.row as u32, cell_layout.col as u32) else {
            continue;
        };
        let text = cell.content.plain_text();
        if text.trim().is_empty() {
            continue;
        }
        let frame = Shape::text_frame_with_text(cell_layout.content_bounds, &text);
        let text_layouter = TextLayouter::new(ctx.fonts);
        if let Some(cell_text_layout) = text_layouter.layout_shape(&frame) {
            emit_text_layout(&cell_text_layout, transform, Some(cell_layout.bounds), out);
        }
    }
}

fn emit_table_borders(
    data: &TableData,
    layout: &repub_layout::TableLayout,
    transform: Transform2D,
    out: &mut Vec<Primitive>,
) {
    let b = layout.bounds;
    let total_w = layout.total_width();
    let total_h = layout.total_height();
    let right = b.x() + total_w;
    let bottom = b.y() + total_h;

    let edge = |stroke: &Option<Stroke>, a: Point, c: Point, out: &mut Vec<Primitive>| {
        if let Some(s) = stroke {
            let mut path = GeomPath::new();
            path.move_to(a);
            path.line_to(c);
            out.push(stroke_primitive(&path, s, transform, None, Vec::new()));
        }
    };

    edge(
        &data.borders.top,
        Point::new(b.x(), b.y()),
        Point::new(right, b.y()),
        out,
    );
    edge(
        &data.borders.bottom,
        Point::new(b.x(), bottom),
        Point::new(right, bottom),
        out,
    );
    edge(
        &data.borders.left,
        Point::new(b.x(), b.y()),
        Point::new(b.x(), bottom),
        out,
    );
    edge(
        &data.borders.right,
        Point::new(right, b.y()),
        Point::new(right, bottom),
        out,
    );

    // Inner vertical lines between columns.
    if data.borders.inner_vertical.is_some() {
        let mut x = b.x();
        for w in layout
            .col_widths
            .iter()
            .take(layout.col_widths.len().saturating_sub(1))
        {
            x += w;
            edge(
                &data.borders.inner_vertical,
                Point::new(x, b.y()),
                Point::new(x, bottom),
                out,
            );
        }
    }
    // Inner horizontal lines between rows.
    if data.borders.inner_horizontal.is_some() {
        let mut y = b.y();
        for h in layout
            .row_heights
            .iter()
            .take(layout.row_heights.len().saturating_sub(1))
        {
            y += h;
            edge(
                &data.borders.inner_horizontal,
                Point::new(b.x(), y),
                Point::new(right, y),
                out,
            );
        }
    }
}

fn compose_line(
    shape: &Shape,
    data: &LineData,
    transform: Transform2D,
    effects: &[SceneEffect],
    out: &mut Vec<Primitive>,
) {
    let stroke = shape
        .line
        .clone()
        .unwrap_or_else(|| Stroke::solid(Color::BLACK, 1.0));

    // The line path: start, control points, end.
    let mut path = GeomPath::new();
    path.move_to(data.start);
    for cp in &data.control_points {
        path.line_to(*cp);
    }
    path.line_to(data.end);
    out.push(stroke_primitive(
        &path,
        &stroke,
        transform,
        None,
        effects.to_vec(),
    ));

    // Arrowheads, resolved to small filled triangles/shapes.
    let dir_end = direction(prev_point(data, false), data.end);
    let dir_start = direction(prev_point(data, true), data.start);
    if let Some(prim) = arrowhead(data.end, dir_end, &data.end_ending, &stroke, transform) {
        out.push(prim);
    }
    if let Some(prim) = arrowhead(
        data.start,
        dir_start,
        &data.start_ending,
        &stroke,
        transform,
    ) {
        out.push(prim);
    }
}

/// The point adjacent to an endpoint, used to compute the ending direction.
fn prev_point(data: &LineData, at_start: bool) -> Point {
    if at_start {
        data.control_points.first().copied().unwrap_or(data.end)
    } else {
        data.control_points.last().copied().unwrap_or(data.start)
    }
}

/// Unit direction from `from` toward `to`.
fn direction(from: Point, to: Point) -> repub_geometry::Vector {
    let v = from.to(to).normalized();
    if v == repub_geometry::Vector::ZERO {
        repub_geometry::Vector::new(1.0, 0.0)
    } else {
        v
    }
}

/// Build an arrowhead primitive at `tip` pointing along `dir`, or `None` for a
/// [`LineEndingStyle::None`] ending.
fn arrowhead(
    tip: Point,
    dir: repub_geometry::Vector,
    ending: &LineEnding,
    stroke: &Stroke,
    transform: Transform2D,
) -> Option<Primitive> {
    if ending.style == LineEndingStyle::None {
        return None;
    }
    let size = (stroke.width * ending.size).max(ending.size);
    // Perpendicular to the direction.
    let perp = repub_geometry::Vector::new(-dir.dy, dir.dx);
    let back = Point::new(tip.x - dir.dx * size, tip.y - dir.dy * size);
    let half = size * 0.5;

    let mut path = GeomPath::new();
    match ending.style {
        LineEndingStyle::Arrow | LineEndingStyle::StealthArrow | LineEndingStyle::OpenArrow => {
            let l = Point::new(back.x + perp.dx * half, back.y + perp.dy * half);
            let r = Point::new(back.x - perp.dx * half, back.y - perp.dy * half);
            path.move_to(tip);
            path.line_to(l);
            if ending.style == LineEndingStyle::StealthArrow {
                // Notch the back for a stealth look.
                let notch = Point::new(back.x + dir.dx * size * 0.3, back.y + dir.dy * size * 0.3);
                path.line_to(notch);
            }
            path.line_to(r);
            path.close();
        }
        LineEndingStyle::Diamond => {
            let l = Point::new(back.x + perp.dx * half, back.y + perp.dy * half);
            let r = Point::new(back.x - perp.dx * half, back.y - perp.dy * half);
            let far = Point::new(tip.x - dir.dx * size * 2.0, tip.y - dir.dy * size * 2.0);
            path.move_to(tip);
            path.line_to(l);
            path.line_to(far);
            path.line_to(r);
            path.close();
        }
        LineEndingStyle::Square => {
            let l1 = Point::new(tip.x + perp.dx * half, tip.y + perp.dy * half);
            let l2 = Point::new(back.x + perp.dx * half, back.y + perp.dy * half);
            let r2 = Point::new(back.x - perp.dx * half, back.y - perp.dy * half);
            let r1 = Point::new(tip.x - perp.dx * half, tip.y - perp.dy * half);
            path.move_to(l1);
            path.line_to(l2);
            path.line_to(r2);
            path.line_to(r1);
            path.close();
        }
        LineEndingStyle::Circle => {
            // Approximate a circle of radius `half` centered at `back`.
            let r = half;
            const K: f64 = 0.552_284_749_830_793_6;
            let c = back;
            path.move_to(Point::new(c.x + r, c.y));
            path.cubic_to(
                Point::new(c.x + r, c.y + K * r),
                Point::new(c.x + K * r, c.y + r),
                Point::new(c.x, c.y + r),
            );
            path.cubic_to(
                Point::new(c.x - K * r, c.y + r),
                Point::new(c.x - r, c.y + K * r),
                Point::new(c.x - r, c.y),
            );
            path.cubic_to(
                Point::new(c.x - r, c.y - K * r),
                Point::new(c.x - K * r, c.y - r),
                Point::new(c.x, c.y - r),
            );
            path.cubic_to(
                Point::new(c.x + K * r, c.y - r),
                Point::new(c.x + r, c.y - K * r),
                Point::new(c.x + r, c.y),
            );
            path.close();
        }
        LineEndingStyle::None => unreachable!(),
    }

    Some(Primitive::FillPath {
        path,
        paint: Paint::Solid(stroke.color),
        fill_rule: FillRule::NonZero,
        transform,
        clip: None,
        effects: Vec::new(),
    })
}

/// Build a stroke primitive from a geometry path and a model stroke.
fn stroke_primitive(
    path: &GeomPath,
    stroke: &Stroke,
    transform: Transform2D,
    clip: Option<Rect>,
    effects: Vec<SceneEffect>,
) -> Primitive {
    Primitive::StrokePath {
        path: path.clone(),
        paint: stroke_paint(stroke),
        width: stroke.width,
        cap: LineCap::from(stroke.cap),
        join: LineJoin::from(stroke.join),
        dash: stroke.dash_pattern.clone(),
        transform,
        clip,
        effects,
    }
}

/// Map the model's [`Effects`] bundle to scene [`SceneEffect`] descriptors.
fn map_effects(effects: &Effects, _scheme: &ColorScheme) -> Vec<SceneEffect> {
    let mut out = Vec::new();
    if let Some(s) = &effects.drop_shadow {
        out.push(SceneEffect::DropShadow {
            color: s.color,
            offset_x: s.offset_x,
            offset_y: s.offset_y,
            blur_radius: s.blur_radius,
            spread: s.spread,
        });
    }
    if let Some(s) = &effects.inner_shadow {
        out.push(SceneEffect::InnerShadow {
            color: s.color,
            offset_x: s.offset_x,
            offset_y: s.offset_y,
            blur_radius: s.blur_radius,
            choke: s.choke,
        });
    }
    if let Some(g) = &effects.outer_glow {
        out.push(SceneEffect::OuterGlow {
            color: g.color,
            radius: g.radius,
            spread: g.spread,
        });
    }
    if let Some(g) = &effects.inner_glow {
        out.push(SceneEffect::InnerGlow {
            color: g.color,
            radius: g.radius,
            choke: g.choke,
            from_center: matches!(g.source, repub_model::styles::InnerGlowSource::Center),
        });
    }
    if let Some(r) = &effects.reflection {
        out.push(SceneEffect::Reflection {
            opacity: r.opacity,
            fade_length: r.fade_length,
            gap: r.gap,
            blur: r.blur,
            size: r.size,
        });
    }
    if let Some(e) = &effects.soft_edges {
        out.push(SceneEffect::SoftEdges { radius: e.radius });
    }
    if let Some(b) = &effects.bevel {
        out.push(SceneEffect::Bevel {
            size: b.size,
            depth: b.depth,
            light_angle: b.light_angle,
            highlight: b.highlight_color,
            shadow: b.shadow_color,
        });
    }
    out
}

/// Resolve raw font bytes for a model font spec, for WordArt outline
/// extraction. Tries the installed-font registry, then the bundled fallback.
fn resolve_font_bytes(font: &repub_model::styles::FontSpec) -> Vec<u8> {
    let bold = font.weight.weight() >= 600;
    let italic = matches!(font.style, FontStyle::Italic | FontStyle::Oblique);
    if let Some(reg) = repub_fonts::registry::try_registry() {
        if let Some(file) = reg.find(&font.family, bold, italic) {
            if let Ok(bytes) = std::fs::read(&file.path) {
                return bytes;
            }
        }
    }
    FALLBACK_FONT.to_vec()
}

/// Create an empty scene sized to `size`, for callers wanting a scene without
/// a document.
pub fn empty_scene(size: Size) -> Scene {
    Scene::new(size)
}
