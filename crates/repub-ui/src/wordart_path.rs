//! WordArt rendering via glyph outline transformation.
//!
//! This module implements true WordArt text warping by:
//! 1. Extracting glyph outlines as Bezier curves using ttf-parser
//! 2. Transforming each control point according to the warp envelope (ECMA-376 algorithm)
//! 3. Rendering the transformed paths using egui's mesh/shape system
//!
//! This matches how MS Publisher renders WordArt: Publisher uses GdipAddPathString
//! to get glyph paths, transforms the control points, then uses GdipFillPath/
//! GdipDrawPath to render.

use egui::{Color32, Mesh, Pos2, Rect, Shape, Stroke, Vec2};
use std::sync::Arc;

use crate::wordart_warp::{WarpFill, WarpPreset};

/// A point in a glyph outline path
#[derive(Debug, Clone, Copy)]
struct PathPoint {
    x: f32,
    y: f32,
}

impl PathPoint {
    fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// A segment of a glyph outline
#[derive(Debug, Clone)]
enum PathSegment {
    MoveTo(PathPoint),
    LineTo(PathPoint),
    QuadTo(PathPoint, PathPoint),  // control, end
    CubicTo(PathPoint, PathPoint, PathPoint),  // control1, control2, end
    Close,
}

/// A complete glyph outline (list of contours)
#[derive(Debug, Clone)]
struct GlyphOutline {
    segments: Vec<PathSegment>,
    advance: f32,
    bounds: Rect,
}

/// Builder to collect outline segments from ttf-parser
struct OutlineBuilder {
    segments: Vec<PathSegment>,
    min_x: f32,
    min_y: f32,
    max_x: f32,
    max_y: f32,
}

impl OutlineBuilder {
    fn new() -> Self {
        Self {
            segments: Vec::new(),
            min_x: f32::MAX,
            min_y: f32::MAX,
            max_x: f32::MIN,
            max_y: f32::MIN,
        }
    }

    fn update_bounds(&mut self, x: f32, y: f32) {
        self.min_x = self.min_x.min(x);
        self.min_y = self.min_y.min(y);
        self.max_x = self.max_x.max(x);
        self.max_y = self.max_y.max(y);
    }

    fn bounds(&self) -> Rect {
        if self.min_x > self.max_x {
            Rect::NOTHING
        } else {
            Rect::from_min_max(
                Pos2::new(self.min_x, self.min_y),
                Pos2::new(self.max_x, self.max_y),
            )
        }
    }
}

impl ttf_parser::OutlineBuilder for OutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        self.update_bounds(x, y);
        self.segments.push(PathSegment::MoveTo(PathPoint::new(x, y)));
    }

    fn line_to(&mut self, x: f32, y: f32) {
        self.update_bounds(x, y);
        self.segments.push(PathSegment::LineTo(PathPoint::new(x, y)));
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.update_bounds(x1, y1);
        self.update_bounds(x, y);
        self.segments.push(PathSegment::QuadTo(
            PathPoint::new(x1, y1),
            PathPoint::new(x, y),
        ));
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.update_bounds(x1, y1);
        self.update_bounds(x2, y2);
        self.update_bounds(x, y);
        self.segments.push(PathSegment::CubicTo(
            PathPoint::new(x1, y1),
            PathPoint::new(x2, y2),
            PathPoint::new(x, y),
        ));
    }

    fn close(&mut self) {
        self.segments.push(PathSegment::Close);
    }
}

/// Extract glyph outline from a font face
fn extract_glyph_outline(face: &ttf_parser::Face, c: char) -> Option<GlyphOutline> {
    let glyph_id = face.glyph_index(c)?;
    let units_per_em = face.units_per_em() as f32;
    let advance = face.glyph_hor_advance(glyph_id).unwrap_or(0) as f32 / units_per_em;

    let mut builder = OutlineBuilder::new();
    face.outline_glyph(glyph_id, &mut builder)?;

    // Normalize to em units (0-1 range roughly)
    let scale = 1.0 / units_per_em;
    let segments: Vec<PathSegment> = builder.segments.iter().map(|seg| {
        match seg {
            PathSegment::MoveTo(p) => PathSegment::MoveTo(PathPoint::new(p.x * scale, p.y * scale)),
            PathSegment::LineTo(p) => PathSegment::LineTo(PathPoint::new(p.x * scale, p.y * scale)),
            PathSegment::QuadTo(c, e) => PathSegment::QuadTo(
                PathPoint::new(c.x * scale, c.y * scale),
                PathPoint::new(e.x * scale, e.y * scale),
            ),
            PathSegment::CubicTo(c1, c2, e) => PathSegment::CubicTo(
                PathPoint::new(c1.x * scale, c1.y * scale),
                PathPoint::new(c2.x * scale, c2.y * scale),
                PathPoint::new(e.x * scale, e.y * scale),
            ),
            PathSegment::Close => PathSegment::Close,
        }
    }).collect();

    let bounds = Rect::from_min_max(
        Pos2::new(builder.min_x * scale, builder.min_y * scale),
        Pos2::new(builder.max_x * scale, builder.max_y * scale),
    );

    Some(GlyphOutline { segments, advance, bounds })
}

/// Get the warp envelope Y positions at a given X percentage
fn get_warp_envelope(preset: WarpPreset, x_pct: f32, height: f32) -> (f32, f32) {
    crate::wordart_warp::get_warp_envelope_internal(preset, x_pct, height)
}

/// Transform a point according to the warp envelope
/// 
/// Per ECMA-376 text warp algorithm:
/// 1. Find the point's (x%, y%) position in the original text rectangle
/// 2. Find point at x% along top path, and x% along bottom path
/// 3. Interpolate between them at y% to get the warped position
///
/// Font coordinates: Y increases upward, ascent is positive, descent is negative
/// Screen coordinates: Y increases downward
/// text_rect: in font units where min_y=descent (negative), max_y=ascent (positive)
fn warp_point(
    point: PathPoint,
    text_min_x: f32,  // 0 for text starting at origin
    text_max_x: f32,  // total advance width
    text_min_y: f32,  // descent (negative, e.g., -0.25)
    text_max_y: f32,  // ascent (positive, e.g., 0.75)
    screen_rect: Rect,
    preset: WarpPreset,
) -> Pos2 {
    let text_width = text_max_x - text_min_x;
    let text_height = text_max_y - text_min_y;
    
    // Calculate x% (0 = left, 1 = right)
    let x_pct = if text_width > 0.0 {
        (point.x - text_min_x) / text_width
    } else {
        0.5
    };
    
    // Calculate y% in font coordinates (0 = bottom/descent, 1 = top/ascent)
    // Then invert for screen coordinates (0 = top of text, 1 = bottom of text)
    let y_pct_font = if text_height > 0.0 {
        (point.y - text_min_y) / text_height  // 0 at descent, 1 at ascent
    } else {
        0.5
    };
    // Invert: in screen space, top of text (ascent) should be y_pct=0
    let y_pct = 1.0 - y_pct_font;
    
    // Get warp envelope at this x position
    let (env_top, env_bottom) = get_warp_envelope(preset, x_pct.clamp(0.0, 1.0), screen_rect.height());
    
    // Interpolate Y position between top and bottom of envelope
    let warped_y = screen_rect.top() + env_top + y_pct.clamp(0.0, 1.0) * (env_bottom - env_top);
    
    // X position maps linearly across the screen rect width
    let warped_x = screen_rect.left() + x_pct * screen_rect.width();
    
    Pos2::new(warped_x, warped_y)
}

/// Transform an entire glyph outline according to the warp envelope
fn warp_outline(
    outline: &GlyphOutline,
    glyph_offset_x: f32,  // Offset of this glyph from text start (em units)
    text_min_x: f32,      // 0 for text starting at origin
    text_max_x: f32,      // total advance width  
    text_min_y: f32,      // descent (negative)
    text_max_y: f32,      // ascent (positive)
    screen_rect: Rect,    // Target screen rect
    preset: WarpPreset,
) -> Vec<PathSegment> {
    let text_width = text_max_x - text_min_x;
    let text_height = text_max_y - text_min_y;
    
    // For Plain preset, use simple linear transform (no warp distortion)
    // This avoids any numerical precision issues from the warp algorithm
    if preset == WarpPreset::Plain {
        let scale_x = if text_width > 0.0 { screen_rect.width() / text_width } else { 1.0 };
        let scale_y = if text_height > 0.0 { screen_rect.height() / text_height } else { 1.0 };
        
        return outline.segments.iter().map(|seg| {
            let transform = |p: &PathPoint| -> PathPoint {
                let text_x = glyph_offset_x + p.x;
                let text_y = p.y;
                
                // Simple linear transform: scale and translate
                let screen_x = screen_rect.left() + (text_x - text_min_x) * scale_x;
                // Font Y is up, screen Y is down: flip and offset
                let screen_y = screen_rect.top() + (text_max_y - text_y) * scale_y;
                
                PathPoint::new(screen_x, screen_y)
            };
            
            match seg {
                PathSegment::MoveTo(p) => PathSegment::MoveTo(transform(p)),
                PathSegment::LineTo(p) => PathSegment::LineTo(transform(p)),
                PathSegment::QuadTo(c, e) => PathSegment::QuadTo(transform(c), transform(e)),
                PathSegment::CubicTo(c1, c2, e) => PathSegment::CubicTo(transform(c1), transform(c2), transform(e)),
                PathSegment::Close => PathSegment::Close,
            }
        }).collect();
    }
    
    // For warped presets, use the full warp algorithm
    outline.segments.iter().map(|seg| {
        let transform = |p: &PathPoint| -> PathPoint {
            // Position the point in text coordinate system
            // glyph_offset_x positions the glyph, p.x/p.y are relative to glyph origin
            let text_x = glyph_offset_x + p.x;
            let text_y = p.y;  // Already in font coordinates (Y up, baseline at 0)
            
            // Transform through warp
            let warped = warp_point(
                PathPoint::new(text_x, text_y),
                text_min_x, text_max_x,
                text_min_y, text_max_y,
                screen_rect,
                preset,
            );
            
            PathPoint::new(warped.x, warped.y)
        };
        
        match seg {
            PathSegment::MoveTo(p) => PathSegment::MoveTo(transform(p)),
            PathSegment::LineTo(p) => PathSegment::LineTo(transform(p)),
            PathSegment::QuadTo(c, e) => PathSegment::QuadTo(transform(c), transform(e)),
            PathSegment::CubicTo(c1, c2, e) => PathSegment::CubicTo(transform(c1), transform(c2), transform(e)),
            PathSegment::Close => PathSegment::Close,
        }
    }).collect()
}

/// Build a lyon Path directly from path segments, preserving curves
/// Lyon's tessellator will handle curve subdivision with proper tolerance
fn build_lyon_path_from_segments(segments: &[PathSegment]) -> lyon::path::Path {
    use lyon::path::Path;
    use lyon::geom::point;
    
    let mut builder = Path::builder();
    
    for seg in segments {
        match seg {
            PathSegment::MoveTo(p) => {
                builder.begin(point(p.x, p.y));
            }
            PathSegment::LineTo(p) => {
                builder.line_to(point(p.x, p.y));
            }
            PathSegment::QuadTo(c, e) => {
                builder.quadratic_bezier_to(point(c.x, c.y), point(e.x, e.y));
            }
            PathSegment::CubicTo(c1, c2, e) => {
                builder.cubic_bezier_to(point(c1.x, c1.y), point(c2.x, c2.y), point(e.x, e.y));
            }
            PathSegment::Close => {
                builder.close();
            }
        }
    }
    
    // End any unclosed path
    builder.build()
}

/// Color source for triangulation - either solid or gradient
enum TriangulationFill<'a> {
    Solid(Color32),
    Gradient {
        fill: &'a repub_model::styles::Fill,
        bounds: Rect,  // Bounding rect for gradient calculation
    },
}



/// Triangulate path segments into a filled mesh using lyon
/// Simple solid fill - antialiasing handled separately or by MSAA if enabled
fn triangulate_path_fill(segments: &[PathSegment], fill: &TriangulationFill<'_>, tolerance: f32) -> Mesh {
    use lyon::tessellation::{
        BuffersBuilder, FillOptions, FillTessellator, FillVertex, VertexBuffers,
    };
    
    let mut mesh = Mesh::default();
    
    if segments.is_empty() {
        return mesh;
    }
    
    let path = build_lyon_path_from_segments(segments);
    
    // Tessellate the fill - lyon handles curve subdivision
    let mut geometry: VertexBuffers<Pos2, u32> = VertexBuffers::new();
    let mut tessellator = FillTessellator::new();
    
    let fill_options = FillOptions::default()
        .with_fill_rule(lyon::tessellation::FillRule::NonZero)
        .with_tolerance(tolerance);
    
    let result = tessellator.tessellate_path(
        &path,
        &fill_options,
        &mut BuffersBuilder::new(&mut geometry, |vertex: FillVertex| {
            Pos2::new(vertex.position().x, vertex.position().y)
        }),
    );
    
    if result.is_err() {
        log::warn!("WordArt fill tessellation failed");
        return mesh;
    }
    
    // Convert fill to egui mesh with appropriate colors
    let base_idx = mesh.vertices.len() as u32;
    
    for pos in geometry.vertices {
        let color = match fill {
            TriangulationFill::Solid(c) => *c,
            TriangulationFill::Gradient { fill: gradient_fill, bounds } => {
                let x_pct = if bounds.width() > 0.0 {
                    (pos.x - bounds.left()) / bounds.width()
                } else {
                    0.5
                };
                let y_pct = if bounds.height() > 0.0 {
                    (pos.y - bounds.top()) / bounds.height()
                } else {
                    0.5
                };
                let core_color = crate::gradient::color_at(gradient_fill, x_pct as f64, y_pct as f64);
                crate::canvas::core_color_to_egui(&core_color, 1.0)
            }
        };
        
        mesh.vertices.push(egui::epaint::Vertex {
            pos,
            uv: egui::epaint::WHITE_UV,
            color,
        });
    }
    
    for idx in geometry.indices {
        mesh.indices.push(base_idx + idx);
    }
    
    mesh
}

/// Tessellate path segments as a stroke/outline using lyon
/// Solid stroke with round joins for smooth corners
fn tessellate_path_stroke(segments: &[PathSegment], stroke_color: Color32, stroke_width: f32, tolerance: f32) -> Mesh {
    use lyon::tessellation::{
        BuffersBuilder, StrokeOptions, StrokeTessellator, StrokeVertex, VertexBuffers,
    };
    
    let mut mesh = Mesh::default();
    
    if segments.is_empty() || stroke_width < 0.1 {
        return mesh;
    }
    
    let path = build_lyon_path_from_segments(segments);
    
    let mut geometry: VertexBuffers<Pos2, u32> = VertexBuffers::new();
    let mut tessellator = StrokeTessellator::new();
    
    // Use round joins for smooth corners
    let stroke_options = StrokeOptions::default()
        .with_line_width(stroke_width)
        .with_line_join(lyon::tessellation::LineJoin::Round)
        .with_line_cap(lyon::tessellation::LineCap::Round)
        .with_miter_limit(4.0)
        .with_tolerance(tolerance);
    
    let result = tessellator.tessellate_path(
        &path,
        &stroke_options,
        &mut BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| {
            Pos2::new(vertex.position().x, vertex.position().y)
        }),
    );
    
    if result.is_err() {
        log::debug!("WordArt stroke tessellation failed");
        return mesh;
    }
    
    let base_idx = mesh.vertices.len() as u32;
    
    for pos in geometry.vertices {
        mesh.vertices.push(egui::epaint::Vertex {
            pos,
            uv: egui::epaint::WHITE_UV,
            color: stroke_color,
        });
    }
    
    for idx in geometry.indices {
        mesh.indices.push(base_idx + idx);
    }
    
    mesh
}

/// Bevel types for 3D effects
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bevel3D {
    /// No bevel
    None,
    /// Flat angled bevel
    Angle,
    /// Rounded bevel
    Circle,
    /// Convex bevel
    Convex,
    /// Cool slant bevel
    CoolSlant,
    /// Cross bevel
    Cross,
    /// Hard edge bevel
    HardEdge,
    /// Relaxed inset
    RelaxedInset,
    /// Soft round
    SoftRound,
}

/// Lighting preset for 3D effects
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lighting3D {
    /// Bright lighting from top-left
    #[default]
    Bright,
    /// Normal balanced lighting
    Normal,
    /// Harsh directional lighting
    Harsh,
    /// Soft diffuse lighting
    Soft,
    /// Flat uniform lighting
    Flat,
    /// Three-point studio lighting
    ThreePoint,
    /// Contrasting two-point lighting
    Contrasting,
}

/// Material type for 3D surface
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Material3D {
    /// Standard matte material
    #[default]
    Matte,
    /// Warm matte
    WarmMatte,
    /// Plastic shiny surface
    Plastic,
    /// Metallic reflective surface
    Metal,
    /// Dark edge effect
    DarkEdge,
    /// Soft edge effect
    SoftEdge,
    /// Wireframe
    Wireframe,
    /// Flat coloring
    Flat,
}

/// 3D extrusion parameters for WordArt
pub struct Extrusion3DParams {
    /// Extrusion depth in points
    pub depth: f32,
    /// Extrusion direction (X, Y offset per depth unit)
    pub direction: (f32, f32),
    /// Extrusion color (None = derive from fill)
    pub color: Option<Color32>,
    /// X rotation in degrees (-90 to 90)
    pub rotation_x: f32,
    /// Y rotation in degrees (-90 to 90)
    pub rotation_y: f32,
    /// Perspective amount (0 = parallel, higher = more perspective)
    pub perspective: f32,
    /// Bevel type for front face
    pub bevel: Bevel3D,
    /// Bevel size in points
    pub bevel_size: f32,
    /// Lighting preset
    pub lighting: Lighting3D,
    /// Material type
    pub material: Material3D,
    /// Contour/edge highlight width
    pub contour_width: f32,
    /// Contour color (None = derive from fill)
    pub contour_color: Option<Color32>,
}

impl Default for Extrusion3DParams {
    fn default() -> Self {
        Self {
            depth: 36.0,
            direction: (0.3, 0.3),
            color: None,
            rotation_x: 0.0,
            rotation_y: 0.0,
            perspective: 0.0,
            bevel: Bevel3D::None,
            bevel_size: 6.0,
            lighting: Lighting3D::default(),
            material: Material3D::default(),
            contour_width: 0.0,
            contour_color: None,
        }
    }
}

/// Render warped WordArt text using glyph outline transformation
pub fn render_wordart_path(
    painter: &egui::Painter,
    text: &str,
    screen_rect: Rect,
    preset: WarpPreset,
    fill: &WarpFill<'_>,
    outline: Option<(Color32, f32)>,
    font_family: Option<&str>,
    bold: bool,
    italic: bool,
) {
    render_wordart_path_3d(painter, text, screen_rect, preset, fill, outline, font_family, bold, italic, None);
}

/// Render warped WordArt text with optional 3D extrusion
pub fn render_wordart_path_3d(
    painter: &egui::Painter,
    text: &str,
    screen_rect: Rect,
    preset: WarpPreset,
    fill: &WarpFill<'_>,
    outline: Option<(Color32, f32)>,
    font_family: Option<&str>,
    bold: bool,
    italic: bool,
    extrusion: Option<&Extrusion3DParams>,
) {
    if text.is_empty() || screen_rect.width() < 1.0 || screen_rect.height() < 1.0 {
        return;
    }
    
    // All presets now use path-based rendering for proper outlines
    
    // Try to load the specified font from system fonts
    let font_data: Option<Vec<u8>> = font_family.and_then(|family| {
        crate::sdf_text::fonts::try_registry().and_then(|reg| {
            reg.find(family, bold, italic).and_then(|font_file| {
                log::debug!("WordArt: Loading font from {:?}", font_file.path);
                std::fs::read(&font_file.path).ok()
            })
        })
    });
    
    // Fall back to bundled Selawik
    static BUNDLED_FONT: &[u8] = include_bytes!("../assets/fonts/selawk.ttf");
    let font_bytes: &[u8] = font_data.as_deref().unwrap_or_else(|| {
        if font_family.is_some() {
            log::debug!("WordArt: Font {:?} not found, using Selawik fallback", font_family);
        }
        BUNDLED_FONT
    });
    
    let Ok(face) = ttf_parser::Face::parse(font_bytes, 0) else {
        // Fall back to grid-cell renderer if font parsing fails
        crate::wordart_warp::render_warped_text_with_fill(
            painter, text, screen_rect, preset, fill, outline, 0.0, 0.0, 0.0,
        );
        return;
    };
    
    // Extract outlines for all characters
    let mut glyph_outlines: Vec<(char, GlyphOutline)> = Vec::new();
    let mut total_advance = 0.0_f32;
    
    for c in text.chars() {
        if let Some(outline) = extract_glyph_outline(&face, c) {
            glyph_outlines.push((c, outline.clone()));
            total_advance += outline.advance;
        }
    }
    
    if glyph_outlines.is_empty() || total_advance < 0.001 {
        return;
    }
    
    // Calculate text bounding box in em units
    let ascent = face.ascender() as f32 / face.units_per_em() as f32;
    let descent = face.descender() as f32 / face.units_per_em() as f32;  // Negative
    let _text_height = ascent - descent;
    let text_width = total_advance;
    
    // Text bounds in em coordinates
    let text_min_x = 0.0_f32;
    let text_max_x = text_width;
    let text_min_y = descent;  // Negative (bottom of descenders)
    let text_max_y = ascent;   // Positive (top of ascenders)
    
    // Apply padding to screen rect
    let padding = screen_rect.width() * 0.03;
    let padded_rect = Rect::from_min_max(
        screen_rect.min + Vec2::new(padding, padding),
        screen_rect.max - Vec2::new(padding, padding),
    );
    
    // Tessellation tolerance - smaller = smoother curves but more triangles
    // Scale based on screen size for consistent visual quality
    let tolerance = (screen_rect.width() / 2000.0).clamp(0.05, 0.5);
    
    // Prepare fill for triangulation
    let triangulation_fill = match fill {
        WarpFill::Solid(c) => TriangulationFill::Solid(*c),
        WarpFill::Gradient(f) => TriangulationFill::Gradient {
            fill: f,
            bounds: padded_rect,  // Use the padded rect as bounds for gradient
        },
    };
    
    // Calculate 3D parameters if effect is enabled
    let extrusion_config = extrusion.map(|ext| {
        // Get base colors
        let fill_color = match fill {
            WarpFill::Solid(c) => *c,
            WarpFill::Gradient(f) => {
                let c = crate::gradient::color_at(f, 0.5, 0.5);
                crate::canvas::core_color_to_egui(&c, 1.0)
            }
        };
        
        let extrusion_base_color = ext.color.unwrap_or_else(|| darken_color(fill_color, 0.5));
        
        // Calculate direction based on rotation angles
        // Convert angles to direction vector (simulate 3D projection)
        let rot_x_rad = ext.rotation_x.to_radians();
        let rot_y_rad = ext.rotation_y.to_radians();
        
        // Direction is derived from rotation:
        // - Y rotation makes extrusion go left/right
        // - X rotation makes extrusion go up/down
        let dir_x = rot_y_rad.sin() * 0.8;
        let dir_y = -rot_x_rad.sin() * 0.6 + 0.4;  // Default slight down-right
        let direction = if ext.direction != (0.0, 0.0) && (ext.rotation_x == 0.0 && ext.rotation_y == 0.0) {
            ext.direction
        } else if ext.rotation_x != 0.0 || ext.rotation_y != 0.0 {
            (dir_x, dir_y)
        } else {
            (0.3, 0.3)  // Default direction
        };
        
        // Calculate lighting factors based on preset
        let (ambient, diffuse, specular) = match ext.lighting {
            Lighting3D::Bright => (0.5, 0.6, 0.4),
            Lighting3D::Normal => (0.4, 0.5, 0.3),
            Lighting3D::Harsh => (0.2, 0.7, 0.5),
            Lighting3D::Soft => (0.6, 0.3, 0.1),
            Lighting3D::Flat => (0.8, 0.2, 0.0),
            Lighting3D::ThreePoint => (0.3, 0.6, 0.4),
            Lighting3D::Contrasting => (0.2, 0.8, 0.5),
        };
        
        // Material affects specularity and edge appearance
        let material_shininess = match ext.material {
            Material3D::Matte => 0.1,
            Material3D::WarmMatte => 0.15,
            Material3D::Plastic => 0.6,
            Material3D::Metal => 0.9,
            Material3D::DarkEdge => 0.3,
            Material3D::SoftEdge => 0.2,
            Material3D::Wireframe => 0.0,
            Material3D::Flat => 0.0,
        };
        
        (extrusion_base_color, direction, ambient, diffuse, specular, material_shininess)
    });
    
    // Calculate extrusion layers if 3D effect is enabled
    let extrusion_layers: Vec<(f32, Color32)> = if let Some(ext) = extrusion {
        if ext.depth > 0.5 {
            let (base_color, direction, ambient, diffuse, _specular, _material_shininess) = extrusion_config.unwrap();
            
            // Create layers from back to front
            // More layers for deeper extrusions for smooth appearance
            let num_layers = ((ext.depth / 0.5).ceil() as usize).min(100);
            
            (0..num_layers).map(|i| {
                let t = (num_layers - 1 - i) as f32 / num_layers.max(1) as f32;
                let offset = ext.depth * t;
                
                // Calculate lighting based on layer depth
                // Surfaces facing the light are brighter
                let light_dir = (-0.5_f32, -0.5_f32, 0.7_f32);  // Light from upper-left-front
                
                // Approximate normal for this extrusion direction
                let extrude_normal = (direction.0, direction.1, -1.0_f32);
                let normal_len = (extrude_normal.0.powi(2) + extrude_normal.1.powi(2) + extrude_normal.2.powi(2)).sqrt();
                let normal = (extrude_normal.0 / normal_len, extrude_normal.1 / normal_len, extrude_normal.2 / normal_len);
                
                // Dot product for diffuse lighting
                let n_dot_l = (normal.0 * light_dir.0 + normal.1 * light_dir.1 + normal.2 * light_dir.2).max(0.0);
                
                let brightness = ambient + diffuse * n_dot_l;
                let layer_color = adjust_brightness(base_color, brightness);
                (offset, layer_color)
            }).collect()
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };
    
    // Transform and render each glyph
    let mut glyph_x = 0.0_f32;
    
    for (_c, glyph_outline) in &glyph_outlines {
        // Warp the outline - this preserves Bezier curves
        let warped_segments = warp_outline(
            glyph_outline, 
            glyph_x, 
            text_min_x, text_max_x,
            text_min_y, text_max_y,
            padded_rect, 
            preset
        );
        
        if warped_segments.is_empty() {
            glyph_x += glyph_outline.advance;
            continue;
        }
        
        // Render extrusion layers (back to front)
        if let Some(ext) = extrusion {
            let (base_color, direction, ..) = extrusion_config.as_ref().unwrap();
            
            for (offset, layer_color) in &extrusion_layers {
                // Offset the segments
                let offset_segments = offset_path_segments(&warped_segments, direction.0 * offset, direction.1 * offset);
                
                // Draw extrusion layer fill
                let ext_fill = TriangulationFill::Solid(*layer_color);
                let ext_mesh = triangulate_path_fill(&offset_segments, &ext_fill, tolerance);
                if !ext_mesh.indices.is_empty() {
                    painter.add(Shape::mesh(ext_mesh));
                }
            }
            
            // Draw extrusion edge strips (connecting back to front)
            if !extrusion_layers.is_empty() {
                render_extrusion_edges(painter, &warped_segments, ext, *direction, *base_color, tolerance);
            }
            
            // Draw contour/edge highlight if specified
            if ext.contour_width > 0.5 {
                let contour_color = ext.contour_color.unwrap_or_else(|| {
                    // Lighter version of fill for highlight
                    match fill {
                        WarpFill::Solid(c) => lighten_color(*c, 1.5),
                        WarpFill::Gradient(f) => {
                            let c = crate::gradient::color_at(f, 0.5, 0.5);
                            lighten_color(crate::canvas::core_color_to_egui(&c, 1.0), 1.5)
                        }
                    }
                });
                let contour_mesh = tessellate_path_stroke(&warped_segments, contour_color, ext.contour_width, tolerance);
                if !contour_mesh.indices.is_empty() {
                    painter.add(Shape::mesh(contour_mesh));
                }
            }
        }
        
        // Draw stroke outline first (behind fill) using lyon tessellation
        // Lyon handles curves natively with configurable tolerance
        if let Some((stroke_color, stroke_width)) = outline {
            let stroke_mesh = tessellate_path_stroke(&warped_segments, stroke_color, stroke_width, tolerance);
            if !stroke_mesh.indices.is_empty() {
                painter.add(Shape::mesh(stroke_mesh));
            }
        }
        
        // Triangulate and fill - lyon tessellates curves smoothly
        let fill_mesh = triangulate_path_fill(&warped_segments, &triangulation_fill, tolerance);
        if !fill_mesh.indices.is_empty() {
            painter.add(Shape::mesh(fill_mesh));
        }
        
        // Render bevel highlight effect on front face
        if let Some(ext) = extrusion {
            if ext.bevel != Bevel3D::None && ext.bevel_size > 0.5 {
                render_bevel_effect(painter, &warped_segments, ext, fill, tolerance);
            }
        }
        
        glyph_x += glyph_outline.advance;
    }
}

/// Darken a color by a factor (0.0 = black, 1.0 = no change)
fn darken_color(color: Color32, factor: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        (color.r() as f32 * factor) as u8,
        (color.g() as f32 * factor) as u8,
        (color.b() as f32 * factor) as u8,
        color.a(),
    )
}

/// Adjust brightness of a color
fn adjust_brightness(color: Color32, factor: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        ((color.r() as f32 * factor).min(255.0)) as u8,
        ((color.g() as f32 * factor).min(255.0)) as u8,
        ((color.b() as f32 * factor).min(255.0)) as u8,
        color.a(),
    )
}

/// Lighten a color (factor > 1.0 = lighter)
fn lighten_color(color: Color32, factor: f32) -> Color32 {
    let r = color.r() as f32;
    let g = color.g() as f32;
    let b = color.b() as f32;
    
    // Blend toward white
    let t = (factor - 1.0).clamp(0.0, 1.0);
    Color32::from_rgba_unmultiplied(
        (r + (255.0 - r) * t).min(255.0) as u8,
        (g + (255.0 - g) * t).min(255.0) as u8,
        (b + (255.0 - b) * t).min(255.0) as u8,
        color.a(),
    )
}

/// Offset all points in path segments by (dx, dy)
fn offset_path_segments(segments: &[PathSegment], dx: f32, dy: f32) -> Vec<PathSegment> {
    segments.iter().map(|seg| {
        match seg {
            PathSegment::MoveTo(p) => PathSegment::MoveTo(PathPoint::new(p.x + dx, p.y + dy)),
            PathSegment::LineTo(p) => PathSegment::LineTo(PathPoint::new(p.x + dx, p.y + dy)),
            PathSegment::QuadTo(c, e) => PathSegment::QuadTo(
                PathPoint::new(c.x + dx, c.y + dy),
                PathPoint::new(e.x + dx, e.y + dy),
            ),
            PathSegment::CubicTo(c1, c2, e) => PathSegment::CubicTo(
                PathPoint::new(c1.x + dx, c1.y + dy),
                PathPoint::new(c2.x + dx, c2.y + dy),
                PathPoint::new(e.x + dx, e.y + dy),
            ),
            PathSegment::Close => PathSegment::Close,
        }
    }).collect()
}

/// Render the edge strips connecting the extrusion layers
fn render_extrusion_edges(
    painter: &egui::Painter,
    front_segments: &[PathSegment],
    ext: &Extrusion3DParams,
    direction: (f32, f32),
    base_color: Color32,
    tolerance: f32,
) {
    // Get the offset back position
    let (dx, dy) = (direction.0 * ext.depth, direction.1 * ext.depth);
    
    // Edge color - use provided extrusion base color
    let edge_color = darken_color(base_color, 0.8);
    
    // Convert segments to points for edge strips
    let front_points = flatten_to_points(front_segments, tolerance);
    
    for contour in front_points {
        if contour.len() < 2 {
            continue;
        }
        
        // Create quad strips connecting front to back
        let mut mesh = Mesh::default();
        
        for i in 0..contour.len() {
            let front_pt = contour[i];
            let back_pt = Pos2::new(front_pt.x + dx, front_pt.y + dy);
            
            // Calculate lighting based on edge orientation
            let next_i = (i + 1) % contour.len();
            let edge_dir = contour[next_i] - front_pt;
            // Simple lighting: edges facing the light (up-left) are brighter
            let light_factor = 0.7 + 0.3 * (-edge_dir.x + edge_dir.y).signum() * 0.5 + 0.5;
            let lit_color = adjust_brightness(edge_color, light_factor.clamp(0.5, 1.2));
            
            let idx = mesh.vertices.len() as u32;
            
            mesh.vertices.push(egui::epaint::Vertex {
                pos: front_pt,
                uv: egui::epaint::WHITE_UV,
                color: lit_color,
            });
            mesh.vertices.push(egui::epaint::Vertex {
                pos: back_pt,
                uv: egui::epaint::WHITE_UV,
                color: darken_color(lit_color, 0.7),
            });
            
            if i < contour.len() - 1 {
                // Two triangles for the quad
                mesh.indices.push(idx);
                mesh.indices.push(idx + 1);
                mesh.indices.push(idx + 2);
                
                mesh.indices.push(idx + 1);
                mesh.indices.push(idx + 3);
                mesh.indices.push(idx + 2);
            }
        }
        
        if !mesh.indices.is_empty() {
            painter.add(Shape::mesh(mesh));
        }
    }
}

/// Flatten path segments to point contours for edge rendering
fn flatten_to_points(segments: &[PathSegment], tolerance: f32) -> Vec<Vec<Pos2>> {
    let mut contours: Vec<Vec<Pos2>> = Vec::new();
    let mut current: Vec<Pos2> = Vec::new();
    let mut pos = Pos2::ZERO;
    
    for seg in segments {
        match seg {
            PathSegment::MoveTo(p) => {
                if !current.is_empty() {
                    contours.push(std::mem::take(&mut current));
                }
                pos = Pos2::new(p.x, p.y);
                current.push(pos);
            }
            PathSegment::LineTo(p) => {
                pos = Pos2::new(p.x, p.y);
                current.push(pos);
            }
            PathSegment::QuadTo(c, e) => {
                let ctrl = Pos2::new(c.x, c.y);
                let end = Pos2::new(e.x, e.y);
                flatten_quadratic_to_points(&mut current, pos, ctrl, end, tolerance);
                pos = end;
            }
            PathSegment::CubicTo(c1, c2, e) => {
                let ctrl1 = Pos2::new(c1.x, c1.y);
                let ctrl2 = Pos2::new(c2.x, c2.y);
                let end = Pos2::new(e.x, e.y);
                flatten_cubic_to_points(&mut current, pos, ctrl1, ctrl2, end, tolerance);
                pos = end;
            }
            PathSegment::Close => {
                if !current.is_empty() {
                    contours.push(std::mem::take(&mut current));
                }
            }
        }
    }
    
    if !current.is_empty() {
        contours.push(current);
    }
    
    contours
}

fn flatten_quadratic_to_points(points: &mut Vec<Pos2>, p0: Pos2, p1: Pos2, p2: Pos2, tolerance: f32) {
    // Simple subdivision
    let mid = Pos2::new(
        0.25 * p0.x + 0.5 * p1.x + 0.25 * p2.x,
        0.25 * p0.y + 0.5 * p1.y + 0.25 * p2.y,
    );
    let line_mid = p0 + (p2 - p0) * 0.5;
    
    if (mid - line_mid).length() <= tolerance {
        points.push(p2);
    } else {
        let p01 = p0 + (p1 - p0) * 0.5;
        let p12 = p1 + (p2 - p1) * 0.5;
        let p012 = p01 + (p12 - p01) * 0.5;
        flatten_quadratic_to_points(points, p0, p01, p012, tolerance);
        flatten_quadratic_to_points(points, p012, p12, p2, tolerance);
    }
}

fn flatten_cubic_to_points(points: &mut Vec<Pos2>, p0: Pos2, p1: Pos2, p2: Pos2, p3: Pos2, tolerance: f32) {
    // Simple subdivision based on control point distance
    let d1 = point_line_dist(p1, p0, p3);
    let d2 = point_line_dist(p2, p0, p3);
    
    if d1 <= tolerance && d2 <= tolerance {
        points.push(p3);
    } else {
        let p01 = p0 + (p1 - p0) * 0.5;
        let p12 = p1 + (p2 - p1) * 0.5;
        let p23 = p2 + (p3 - p2) * 0.5;
        let p012 = p01 + (p12 - p01) * 0.5;
        let p123 = p12 + (p23 - p12) * 0.5;
        let p0123 = p012 + (p123 - p012) * 0.5;
        
        flatten_cubic_to_points(points, p0, p01, p012, p0123, tolerance);
        flatten_cubic_to_points(points, p0123, p123, p23, p3, tolerance);
    }
}

fn point_line_dist(p: Pos2, line_start: Pos2, line_end: Pos2) -> f32 {
    let d = line_end - line_start;
    let len_sq = d.length_sq();
    if len_sq < 0.0001 {
        return (p - line_start).length();
    }
    let t = ((p - line_start).dot(d) / len_sq).clamp(0.0, 1.0);
    let proj = line_start + d * t;
    (p - proj).length()
}

/// Render bevel highlight effect on the front face edges
fn render_bevel_effect(
    painter: &egui::Painter,
    segments: &[PathSegment],
    ext: &Extrusion3DParams,
    fill: &WarpFill<'_>,
    tolerance: f32,
) {
    let bevel_size = ext.bevel_size;
    if bevel_size < 0.5 {
        return;
    }
    
    // Get the fill color for highlight calculation
    let fill_color = match fill {
        WarpFill::Solid(c) => *c,
        WarpFill::Gradient(f) => {
            let c = crate::gradient::color_at(f, 0.5, 0.5);
            crate::canvas::core_color_to_egui(&c, 1.0)
        }
    };
    
    // Light direction (normalized)
    let light_dir = (-0.6_f32, -0.4_f32);  // Upper-left
    
    // Get flattened contours
    let contours = flatten_to_points(segments, tolerance);
    
    for contour in contours {
        if contour.len() < 3 {
            continue;
        }
        
        let mut mesh = Mesh::default();
        
        for i in 0..contour.len() {
            let prev_i = if i == 0 { contour.len() - 1 } else { i - 1 };
            let next_i = (i + 1) % contour.len();
            
            let p = contour[i];
            let prev = contour[prev_i];
            let next = contour[next_i];
            
            // Calculate edge normals (pointing inward)
            let edge1 = p - prev;
            let edge2 = next - p;
            
            // Average normal (bisector)
            let n1 = Vec2::new(-edge1.y, edge1.x);
            let n2 = Vec2::new(-edge2.y, edge2.x);
            let avg_normal = (n1.normalized() + n2.normalized()).normalized();
            
            // Inset point
            let inset = p + avg_normal * bevel_size;
            
            // Calculate lighting based on edge normal vs light direction
            let n_dot_l = avg_normal.x * light_dir.0 + avg_normal.y * light_dir.1;
            
            // Bevel color: highlight (bright) or shadow (dark) based on facing
            let bevel_color = if n_dot_l > 0.0 {
                // Facing light - highlight
                lighten_color(fill_color, 1.0 + n_dot_l * 0.6)
            } else {
                // Facing away - shadow  
                darken_color(fill_color, 1.0 + n_dot_l.abs() * 0.3)
            };
            
            // Apply bevel type modulation
            let modulated_color = match ext.bevel {
                Bevel3D::Angle | Bevel3D::HardEdge => bevel_color,
                Bevel3D::Circle | Bevel3D::SoftRound => {
                    // Softer gradient
                    let softness = 0.7;
                    Color32::from_rgba_unmultiplied(
                        ((fill_color.r() as f32 * (1.0 - softness) + bevel_color.r() as f32 * softness).min(255.0)) as u8,
                        ((fill_color.g() as f32 * (1.0 - softness) + bevel_color.g() as f32 * softness).min(255.0)) as u8,
                        ((fill_color.b() as f32 * (1.0 - softness) + bevel_color.b() as f32 * softness).min(255.0)) as u8,
                        fill_color.a(),
                    )
                }
                Bevel3D::Convex => {
                    // Extra bright on edges facing light
                    if n_dot_l > 0.3 {
                        lighten_color(bevel_color, 1.2)
                    } else {
                        bevel_color
                    }
                }
                Bevel3D::CoolSlant => {
                    // Metallic-ish effect
                    lighten_color(darken_color(bevel_color, 0.8), if n_dot_l > 0.0 { 1.4 } else { 1.0 })
                }
                Bevel3D::Cross => {
                    // Sharp contrast
                    if n_dot_l > 0.1 {
                        Color32::WHITE.gamma_multiply(0.8)
                    } else if n_dot_l < -0.1 {
                        darken_color(fill_color, 0.5)
                    } else {
                        fill_color
                    }
                }
                Bevel3D::RelaxedInset => {
                    // Subtle effect
                    let blend = 0.5;
                    Color32::from_rgba_unmultiplied(
                        ((fill_color.r() as f32 * (1.0 - blend) + bevel_color.r() as f32 * blend).min(255.0)) as u8,
                        ((fill_color.g() as f32 * (1.0 - blend) + bevel_color.g() as f32 * blend).min(255.0)) as u8,
                        ((fill_color.b() as f32 * (1.0 - blend) + bevel_color.b() as f32 * blend).min(255.0)) as u8,
                        fill_color.a(),
                    )
                }
                _ => bevel_color,
            };
            
            let idx = mesh.vertices.len() as u32;
            
            // Outer vertex (original edge)
            mesh.vertices.push(egui::epaint::Vertex {
                pos: p,
                uv: egui::epaint::WHITE_UV,
                color: modulated_color,
            });
            
            // Inner vertex (inset)
            mesh.vertices.push(egui::epaint::Vertex {
                pos: inset,
                uv: egui::epaint::WHITE_UV,
                color: fill_color,  // Transition to fill color
            });
            
            // Connect to next edge point
            if i < contour.len() - 1 {
                mesh.indices.push(idx);
                mesh.indices.push(idx + 1);
                mesh.indices.push(idx + 2);
                
                mesh.indices.push(idx + 1);
                mesh.indices.push(idx + 3);
                mesh.indices.push(idx + 2);
            }
        }
        
        // Close the loop
        if contour.len() >= 2 {
            let last_idx = (mesh.vertices.len() - 2) as u32;
            mesh.indices.push(last_idx);
            mesh.indices.push(last_idx + 1);
            mesh.indices.push(0);
            
            mesh.indices.push(last_idx + 1);
            mesh.indices.push(1);
            mesh.indices.push(0);
        }
        
        if !mesh.indices.is_empty() {
            painter.add(Shape::mesh(mesh));
        }
    }
}
