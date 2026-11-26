//! Create a document with comprehensive WordArt tests.
//!
//! Run with: cargo run -p repub-mcp --example create_wordart_tests

use repub_color::Color;
use repub_geometry::{Rect, Point as GeoPoint};
use repub_model::shape::{
    BevelType, Extrusion3D, Preset3DStyle, Rotation3D, Shape, ShapeKind, TextEffectData,
    TextTransform, WordArtPreset,
};
use repub_model::styles::{Fill, GradientStop, LinearGradient, Stroke};
use repub_model::{Document, PageSize, PageOrientation};

/// A small builder mirroring the old `WordArt` object's setters, producing a
/// unified text-effect [`Shape`]. Fill and outline live on the shape; the
/// text-effect payload (transform, 3D rotation, extrusion) lives on its kind.
struct WordArt {
    shape: Shape,
}

impl WordArt {
    fn new(bounds: Rect, text: &str) -> Self {
        Self { shape: Shape::text_effect(bounds, text) }
    }

    fn effect_mut(&mut self) -> &mut TextEffectData {
        match &mut self.shape.kind {
            ShapeKind::TextEffect(data) => data,
            _ => unreachable!("text_effect always builds a TextEffect kind"),
        }
    }

    fn set_fill(&mut self, fill: Fill) {
        self.shape.fill = fill;
    }

    fn set_outline(&mut self, outline: Stroke) {
        self.shape.line = Some(outline);
    }

    fn set_transform(&mut self, transform: TextTransform) {
        self.effect_mut().transform = transform;
    }

    fn set_extrusion(&mut self, extrusion: Extrusion3D) {
        self.effect_mut().extrusion = Some(extrusion);
    }

    fn set_rotation_3d(&mut self, rotation: Rotation3D) {
        self.effect_mut().rotation_3d = rotation;
    }

    fn into_shape(self) -> Shape {
        self.shape
    }
}

fn main() {
    // Create a multi-page document for WordArt tests
    let mut doc = Document::new(PageSize::Letter, PageOrientation::Landscape);
    
    // Page 1: Basic presets (Row 1)
    create_preset_page(&mut doc, 0, &[
        ("Plain", WordArtPreset::Plain, solid_blue()),
        ("Wave", WordArtPreset::Wave, gradient_rainbow()),
        ("Arch Up", WordArtPreset::ArchUp, gradient_gold()),
        ("Arch Down", WordArtPreset::ArchDown, gradient_silver()),
    ], "WordArt Presets - Page 1: Basic Transforms");
    
    // Page 2: Circle and Can presets
    doc.add_page();
    create_preset_page(&mut doc, 1, &[
        ("Circle", WordArtPreset::Circle, gradient_purple()),
        ("Can Up", WordArtPreset::CanUp, gradient_sunset()),
        ("Can Down", WordArtPreset::CanDown, gradient_ocean()),
        ("Button", WordArtPreset::Button, gradient_fire()),
    ], "WordArt Presets - Page 2: Curved Effects");
    
    // Page 3: Inflate/Deflate presets
    doc.add_page();
    create_preset_page(&mut doc, 2, &[
        ("Inflate", WordArtPreset::Inflate, gradient_green()),
        ("Deflate", WordArtPreset::Deflate, gradient_cyan()),
        ("Slant Up", WordArtPreset::SlantUp, solid_red()),
        ("Slant Down", WordArtPreset::SlantDown, solid_orange()),
    ], "WordArt Presets - Page 3: Inflate & Slant");
    
    // Page 4: Fade presets
    doc.add_page();
    create_preset_page(&mut doc, 3, &[
        ("Fade Right", WordArtPreset::FadeRight, gradient_hot()),
        ("Fade Left", WordArtPreset::FadeLeft, gradient_cool()),
        ("Chevron Up", WordArtPreset::ChevronUp, gradient_neon()),
        ("Chevron Down", WordArtPreset::ChevronDown, gradient_earth()),
    ], "WordArt Presets - Page 4: Fade & Chevron");
    
    // Page 5: Triangle and geometric presets  
    doc.add_page();
    create_preset_page(&mut doc, 4, &[
        ("Triangle Up", WordArtPreset::TriangleUp, solid_magenta()),
        ("Triangle Down", WordArtPreset::TriangleDown, solid_teal()),
        ("Cascade Up", WordArtPreset::CascadeUp, gradient_pastel()),
        ("Cascade Down", WordArtPreset::CascadeDown, gradient_dark()),
    ], "WordArt Presets - Page 5: Triangle & Cascade");
    
    // Page 6: Ring and special presets
    doc.add_page();
    create_preset_page(&mut doc, 5, &[
        ("Stop", WordArtPreset::Stop, solid_red()),
        ("Ring Inside", WordArtPreset::RingInside, gradient_metal()),
        ("Ring Outside", WordArtPreset::RingOutside, gradient_candy()),
        ("Shadow", WordArtPreset::Shadow, solid_gray()),
    ], "WordArt Presets - Page 6: Special Effects");
    
    // Page 7: Gradient showcase
    doc.add_page();
    create_gradient_showcase(&mut doc, 6);
    
    // Page 8: Outline showcase
    doc.add_page();
    create_outline_showcase(&mut doc, 7);
    
    // Page 9: 3D Effects showcase
    doc.add_page();
    create_3d_showcase(&mut doc, 8);
    
    // Page 10: Bevel Effects showcase
    doc.add_page();
    create_bevel_showcase(&mut doc, 9);
    
    // Page 11: 3D Presets showcase (part 1)
    doc.add_page();
    create_3d_presets_showcase_1(&mut doc, 10);
    
    // Page 12: 3D Presets showcase (part 2)
    doc.add_page();
    create_3d_presets_showcase_2(&mut doc, 11);
    
    // Save as .repub file using the native format
    let output_path = std::env::current_dir()
        .unwrap()
        .join("wordart_tests.repub");
    
    // Use the native format which includes version wrapper
    repub_format_native::save(&doc, &output_path, &repub_format_native::SaveOptions::default())
        .expect("Failed to save document");
    
    println!("Created WordArt test document: {}", output_path.display());
    println!("Document has {} pages with various WordArt presets.", doc.pages().len());
}

fn create_preset_page(doc: &mut Document, page_idx: usize, presets: &[(&str, WordArtPreset, Fill)], title: &str) {
    let page = &mut doc.pages_mut()[page_idx];
    
    // Add title
    let mut title_wordart = WordArt::new(Rect::new(72.0, 36.0, 648.0, 48.0), title);
    title_wordart.set_fill(Fill::Solid(Color::new(0.2, 0.2, 0.2, 1.0)));
    title_wordart.set_transform(TextTransform::Preset(WordArtPreset::Plain));
    page.add_shape(title_wordart.into_shape());
    
    // Create 2x2 grid of WordArt
    let positions = [
        (72.0, 100.0),   // Top left
        (396.0, 100.0),  // Top right
        (72.0, 310.0),   // Bottom left
        (396.0, 310.0),  // Bottom right
    ];
    
    for (i, (text, preset, fill)) in presets.iter().enumerate() {
        if i >= 4 { break; }
        
        let (x, y) = positions[i];
        let mut wordart = WordArt::new(Rect::new(x, y, 300.0, 180.0), text);
        wordart.set_fill(fill.clone());
        wordart.set_transform(TextTransform::Preset(*preset));
        
        // Add outline for better visibility
        wordart.set_outline(Stroke::new(Color::new(0.1, 0.1, 0.1, 0.8), 1.5));
        
        page.add_shape(wordart.into_shape());
    }
}

fn create_gradient_showcase(doc: &mut Document, page_idx: usize) {
    let page = &mut doc.pages_mut()[page_idx];
    
    // Add title
    let mut title = WordArt::new(Rect::new(72.0, 36.0, 648.0, 48.0), "Gradient Showcase");
    title.set_fill(gradient_rainbow());
    title.set_transform(TextTransform::Preset(WordArtPreset::Wave));
    page.add_shape(title.into_shape());
    
    let gradients = [
        ("Rainbow", gradient_rainbow()),
        ("Sunset", gradient_sunset()),
        ("Ocean", gradient_ocean()),
        ("Fire", gradient_fire()),
        ("Neon", gradient_neon()),
        ("Metal", gradient_metal()),
    ];
    
    for (i, (text, fill)) in gradients.iter().enumerate() {
        let row = i / 2;
        let col = i % 2;
        let x = 72.0 + col as f64 * 324.0;
        let y = 100.0 + row as f64 * 150.0;
        
        let mut wordart = WordArt::new(Rect::new(x, y, 300.0, 120.0), text);
        wordart.set_fill(fill.clone());
        wordart.set_transform(TextTransform::Preset(WordArtPreset::Inflate));
        wordart.set_outline(Stroke::new(Color::BLACK, 2.0));
        
        page.add_shape(wordart.into_shape());
    }
}

fn create_outline_showcase(doc: &mut Document, page_idx: usize) {
    let page = &mut doc.pages_mut()[page_idx];
    
    // Add title
    let mut title = WordArt::new(Rect::new(72.0, 36.0, 648.0, 48.0), "Outline Showcase");
    title.set_fill(Fill::Solid(Color::new(0.2, 0.2, 0.4, 1.0)));
    title.set_transform(TextTransform::Preset(WordArtPreset::Plain));
    page.add_shape(title.into_shape());
    
    let outlines = [
        ("Thin Black", Color::BLACK, 1.0),
        ("Thick Red", Color::RED, 4.0),
        ("Blue Medium", Color::BLUE, 2.5),
        ("Green Bold", Color::new(0.0, 0.6, 0.0, 1.0), 3.0),
        ("Gold Accent", Color::new(0.85, 0.65, 0.0, 1.0), 2.0),
        ("Purple Glow", Color::new(0.5, 0.0, 0.8, 1.0), 3.5),
    ];
    
    for (i, (text, outline_color, width)) in outlines.iter().enumerate() {
        let row = i / 2;
        let col = i % 2;
        let x = 72.0 + col as f64 * 324.0;
        let y = 100.0 + row as f64 * 150.0;
        
        let mut wordart = WordArt::new(Rect::new(x, y, 300.0, 120.0), text);
        wordart.set_fill(Fill::Solid(Color::WHITE));
        wordart.set_transform(TextTransform::Preset(WordArtPreset::ArchUp));
        wordart.set_outline(Stroke::new(*outline_color, *width));
        
        page.add_shape(wordart.into_shape());
    }
}

fn create_3d_showcase(doc: &mut Document, page_idx: usize) {
    let page = &mut doc.pages_mut()[page_idx];
    
    // Add title
    let mut title = WordArt::new(Rect::new(72.0, 36.0, 648.0, 48.0), "3D Effects Showcase");
    title.set_fill(Fill::Solid(Color::new(0.2, 0.2, 0.4, 1.0)));
    title.set_transform(TextTransform::Preset(WordArtPreset::Plain));
    page.add_shape(title.into_shape());
    
    // Different 3D extrusion depths and angles
    let effects_3d: [(&str, f64, f64, f64, Color); 6] = [
        ("Shallow 3D", 10.0, 0.0, 0.0, Color::new(0.3, 0.5, 0.9, 1.0)),
        ("Deep 3D", 30.0, 0.0, 0.0, Color::new(0.9, 0.3, 0.3, 1.0)),
        ("Y Rotated", 20.0, 0.0, 15.0, Color::new(0.3, 0.8, 0.3, 1.0)),
        ("X Rotated", 20.0, 15.0, 0.0, Color::new(0.9, 0.7, 0.2, 1.0)),
        ("XY Rotated", 25.0, 10.0, 10.0, Color::new(0.6, 0.3, 0.9, 1.0)),
        ("Extreme", 40.0, 20.0, 20.0, Color::new(0.9, 0.5, 0.7, 1.0)),
    ];
    
    for (i, (text, depth, x_rot, y_rot, fill_color)) in effects_3d.iter().enumerate() {
        let row = i / 2;
        let col = i % 2;
        let x = 72.0 + col as f64 * 324.0;
        let y = 100.0 + row as f64 * 150.0;
        
        let mut wordart = WordArt::new(Rect::new(x, y, 300.0, 120.0), text);
        wordart.set_fill(Fill::Solid(*fill_color));
        wordart.set_transform(TextTransform::Preset(WordArtPreset::Plain));
        wordart.set_outline(Stroke::new(Color::new(0.1, 0.1, 0.1, 1.0), 1.5));
        
        // Apply 3D extrusion
        wordart.set_extrusion(Extrusion3D {
            depth: *depth,
            color: Some(Color::new(0.3, 0.3, 0.3, 1.0)), // Dark gray extrusion
            contour_width: 0.0,
            bevel: None,
            bevel_size: 0.0,
        });
        
        // Apply 3D rotation
        wordart.set_rotation_3d(Rotation3D::new(*x_rot, *y_rot, 0.0));
        
        page.add_shape(wordart.into_shape());
    }
}

fn create_bevel_showcase(doc: &mut Document, page_idx: usize) {
    let page = &mut doc.pages_mut()[page_idx];
    
    // Add title
    let mut title = WordArt::new(Rect::new(72.0, 36.0, 648.0, 48.0), "Bevel Effects Showcase");
    title.set_fill(Fill::Solid(Color::new(0.2, 0.2, 0.4, 1.0)));
    title.set_transform(TextTransform::Preset(WordArtPreset::Plain));
    page.add_shape(title.into_shape());
    
    // Different bevel types
    let bevels = [
        ("Circle Bevel", BevelType::Circle, Color::new(0.3, 0.6, 0.9, 1.0)),
        ("Angle Bevel", BevelType::Angle, Color::new(0.9, 0.5, 0.2, 1.0)),
        ("Convex Bevel", BevelType::Convex, Color::new(0.3, 0.9, 0.5, 1.0)),
        ("Soft Round", BevelType::SoftRound, Color::new(0.9, 0.3, 0.6, 1.0)),
        ("Hard Edge", BevelType::HardEdge, Color::new(0.6, 0.6, 0.9, 1.0)),
        ("Cool Slant", BevelType::CoolSlant, Color::new(0.9, 0.9, 0.3, 1.0)),
    ];
    
    for (i, (text, bevel_type, fill_color)) in bevels.iter().enumerate() {
        let row = i / 2;
        let col = i % 2;
        let x = 72.0 + col as f64 * 324.0;
        let y = 100.0 + row as f64 * 150.0;
        
        let mut wordart = WordArt::new(Rect::new(x, y, 300.0, 120.0), text);
        wordart.set_fill(Fill::Solid(*fill_color));
        wordart.set_transform(TextTransform::Preset(WordArtPreset::Plain));
        
        // Apply 3D extrusion with bevel
        wordart.set_extrusion(Extrusion3D {
            depth: 20.0,
            color: Some(Color::new(0.4, 0.4, 0.4, 1.0)),
            contour_width: 0.0,
            bevel: Some(bevel_type.clone()),
            bevel_size: 8.0,
        });
        
        // Slight rotation for better 3D visibility
        wordart.set_rotation_3d(Rotation3D::new(5.0, 10.0, 0.0));
        
        page.add_shape(wordart.into_shape());
    }
}

fn create_3d_presets_showcase_1(doc: &mut Document, page_idx: usize) {
    let page = &mut doc.pages_mut()[page_idx];
    
    // Add title
    let mut title = WordArt::new(Rect::new(72.0, 36.0, 648.0, 36.0), "3D Presets - Parallel & Perspective");
    title.set_fill(Fill::Solid(Color::new(0.2, 0.2, 0.4, 1.0)));
    title.set_transform(TextTransform::Preset(WordArtPreset::Plain));
    page.add_shape(title.into_shape());
    
    // First 10 presets
    let presets: [(&str, Preset3DStyle, Color); 10] = [
        ("Parallel R", Preset3DStyle::ParallelRight, Color::new(0.3, 0.5, 0.9, 1.0)),
        ("Parallel L", Preset3DStyle::ParallelLeft, Color::new(0.9, 0.3, 0.3, 1.0)),
        ("Parallel T", Preset3DStyle::ParallelTop, Color::new(0.3, 0.9, 0.5, 1.0)),
        ("Parallel B", Preset3DStyle::ParallelBottom, Color::new(0.9, 0.7, 0.2, 1.0)),
        ("Persp R", Preset3DStyle::PerspectiveRight, Color::new(0.6, 0.3, 0.9, 1.0)),
        ("Persp L", Preset3DStyle::PerspectiveLeft, Color::new(0.9, 0.5, 0.7, 1.0)),
        ("Persp CL", Preset3DStyle::PerspectiveContrastingLeft, Color::new(0.3, 0.7, 0.9, 1.0)),
        ("Persp CR", Preset3DStyle::PerspectiveContrastingRight, Color::new(0.9, 0.5, 0.3, 1.0)),
        ("Hero R", Preset3DStyle::PerspectiveHeroicRight, Color::new(0.5, 0.9, 0.5, 1.0)),
        ("Hero L", Preset3DStyle::PerspectiveHeroicLeft, Color::new(0.9, 0.3, 0.9, 1.0)),
    ];
    
    for (i, (text, preset, fill_color)) in presets.iter().enumerate() {
        let row = i / 2;
        let col = i % 2;
        let x = 72.0 + col as f64 * 324.0;
        let y = 80.0 + row as f64 * 90.0;
        
        let mut wordart = WordArt::new(Rect::new(x, y, 300.0, 80.0), text);
        wordart.set_fill(Fill::Solid(*fill_color));
        wordart.set_transform(TextTransform::Preset(WordArtPreset::Plain));
        
        // Apply 3D preset
        if let Some(ext) = preset.to_extrusion() {
            wordart.set_extrusion(ext);
        }
        wordart.set_rotation_3d(preset.to_rotation());
        
        page.add_shape(wordart.into_shape());
    }
}

fn create_3d_presets_showcase_2(doc: &mut Document, page_idx: usize) {
    let page = &mut doc.pages_mut()[page_idx];
    
    // Add title
    let mut title = WordArt::new(Rect::new(72.0, 36.0, 648.0, 36.0), "3D Presets - Off-Axis & Isometric");
    title.set_fill(Fill::Solid(Color::new(0.2, 0.2, 0.4, 1.0)));
    title.set_transform(TextTransform::Preset(WordArtPreset::Plain));
    page.add_shape(title.into_shape());
    
    // Remaining presets
    let presets: [(&str, Preset3DStyle, Color); 10] = [
        ("Off1 R", Preset3DStyle::OffAxis1Right, Color::new(0.3, 0.5, 0.9, 1.0)),
        ("Off1 L", Preset3DStyle::OffAxis1Left, Color::new(0.9, 0.3, 0.3, 1.0)),
        ("Off1 T", Preset3DStyle::OffAxis1Top, Color::new(0.3, 0.9, 0.5, 1.0)),
        ("Off1 B", Preset3DStyle::OffAxis1Bottom, Color::new(0.9, 0.7, 0.2, 1.0)),
        ("Off2 R", Preset3DStyle::OffAxis2Right, Color::new(0.6, 0.3, 0.9, 1.0)),
        ("Off2 L", Preset3DStyle::OffAxis2Left, Color::new(0.9, 0.5, 0.7, 1.0)),
        ("Iso LD", Preset3DStyle::IsometricLeftDown, Color::new(0.3, 0.7, 0.9, 1.0)),
        ("Iso RD", Preset3DStyle::IsometricRightDown, Color::new(0.9, 0.5, 0.3, 1.0)),
        ("Iso LU", Preset3DStyle::IsometricLeftUp, Color::new(0.5, 0.9, 0.5, 1.0)),
        ("Iso RU", Preset3DStyle::IsometricRightUp, Color::new(0.9, 0.3, 0.9, 1.0)),
    ];
    
    for (i, (text, preset, fill_color)) in presets.iter().enumerate() {
        let row = i / 2;
        let col = i % 2;
        let x = 72.0 + col as f64 * 324.0;
        let y = 80.0 + row as f64 * 90.0;
        
        let mut wordart = WordArt::new(Rect::new(x, y, 300.0, 80.0), text);
        wordart.set_fill(Fill::Solid(*fill_color));
        wordart.set_transform(TextTransform::Preset(WordArtPreset::Plain));
        
        // Apply 3D preset
        if let Some(ext) = preset.to_extrusion() {
            wordart.set_extrusion(ext);
        }
        wordart.set_rotation_3d(preset.to_rotation());
        
        page.add_shape(wordart.into_shape());
    }
}

// Fill helper functions
fn solid_blue() -> Fill {
    Fill::Solid(Color::new(0.1, 0.3, 0.8, 1.0))
}

fn solid_red() -> Fill {
    Fill::Solid(Color::new(0.9, 0.1, 0.1, 1.0))
}

fn solid_orange() -> Fill {
    Fill::Solid(Color::new(0.95, 0.5, 0.1, 1.0))
}

fn solid_magenta() -> Fill {
    Fill::Solid(Color::new(0.8, 0.1, 0.6, 1.0))
}

fn solid_teal() -> Fill {
    Fill::Solid(Color::new(0.0, 0.6, 0.6, 1.0))
}

fn solid_gray() -> Fill {
    Fill::Solid(Color::new(0.4, 0.4, 0.4, 1.0))
}

fn gradient_rainbow() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.0, 0.0),
        end: GeoPoint::new(1.0, 1.0),
        stops: vec![
            GradientStop::new(0.0, Color::new(1.0, 0.0, 0.0, 1.0)),
            GradientStop::new(0.2, Color::new(1.0, 0.5, 0.0, 1.0)),
            GradientStop::new(0.4, Color::new(1.0, 1.0, 0.0, 1.0)),
            GradientStop::new(0.6, Color::new(0.0, 1.0, 0.0, 1.0)),
            GradientStop::new(0.8, Color::new(0.0, 0.0, 1.0, 1.0)),
            GradientStop::new(1.0, Color::new(0.5, 0.0, 1.0, 1.0)),
        ],
    })
}

fn gradient_gold() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.5, 0.0),
        end: GeoPoint::new(0.5, 1.0),
        stops: vec![
            GradientStop::new(0.0, Color::new(1.0, 0.95, 0.5, 1.0)),
            GradientStop::new(0.5, Color::new(0.85, 0.65, 0.0, 1.0)),
            GradientStop::new(1.0, Color::new(0.6, 0.4, 0.0, 1.0)),
        ],
    })
}

fn gradient_silver() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.5, 0.0),
        end: GeoPoint::new(0.5, 1.0),
        stops: vec![
            GradientStop::new(0.0, Color::new(0.95, 0.95, 0.95, 1.0)),
            GradientStop::new(0.5, Color::new(0.75, 0.75, 0.8, 1.0)),
            GradientStop::new(1.0, Color::new(0.5, 0.5, 0.55, 1.0)),
        ],
    })
}

fn gradient_purple() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.0, 0.5),
        end: GeoPoint::new(1.0, 0.5),
        stops: vec![
            GradientStop::new(0.0, Color::new(0.3, 0.0, 0.5, 1.0)),
            GradientStop::new(0.5, Color::new(0.6, 0.0, 0.8, 1.0)),
            GradientStop::new(1.0, Color::new(0.9, 0.3, 1.0, 1.0)),
        ],
    })
}

fn gradient_sunset() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.0, 0.0),
        end: GeoPoint::new(1.0, 1.0),
        stops: vec![
            GradientStop::new(0.0, Color::new(1.0, 0.3, 0.0, 1.0)),
            GradientStop::new(0.5, Color::new(1.0, 0.0, 0.3, 1.0)),
            GradientStop::new(1.0, Color::new(0.5, 0.0, 0.5, 1.0)),
        ],
    })
}

fn gradient_ocean() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.5, 0.0),
        end: GeoPoint::new(0.5, 1.0),
        stops: vec![
            GradientStop::new(0.0, Color::new(0.0, 0.8, 1.0, 1.0)),
            GradientStop::new(0.5, Color::new(0.0, 0.4, 0.8, 1.0)),
            GradientStop::new(1.0, Color::new(0.0, 0.1, 0.4, 1.0)),
        ],
    })
}

fn gradient_fire() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.5, 1.0),
        end: GeoPoint::new(0.5, 0.0),
        stops: vec![
            GradientStop::new(0.0, Color::new(1.0, 1.0, 0.0, 1.0)),
            GradientStop::new(0.3, Color::new(1.0, 0.6, 0.0, 1.0)),
            GradientStop::new(0.7, Color::new(1.0, 0.2, 0.0, 1.0)),
            GradientStop::new(1.0, Color::new(0.5, 0.0, 0.0, 1.0)),
        ],
    })
}

fn gradient_green() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.5, 0.0),
        end: GeoPoint::new(0.5, 1.0),
        stops: vec![
            GradientStop::new(0.0, Color::new(0.6, 1.0, 0.4, 1.0)),
            GradientStop::new(0.5, Color::new(0.0, 0.7, 0.2, 1.0)),
            GradientStop::new(1.0, Color::new(0.0, 0.4, 0.1, 1.0)),
        ],
    })
}

fn gradient_cyan() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.0, 0.5),
        end: GeoPoint::new(1.0, 0.5),
        stops: vec![
            GradientStop::new(0.0, Color::new(0.0, 1.0, 1.0, 1.0)),
            GradientStop::new(1.0, Color::new(0.0, 0.5, 0.8, 1.0)),
        ],
    })
}

fn gradient_hot() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.0, 0.5),
        end: GeoPoint::new(1.0, 0.5),
        stops: vec![
            GradientStop::new(0.0, Color::new(1.0, 0.0, 0.0, 1.0)),
            GradientStop::new(0.5, Color::new(1.0, 0.5, 0.0, 1.0)),
            GradientStop::new(1.0, Color::new(1.0, 1.0, 0.0, 1.0)),
        ],
    })
}

fn gradient_cool() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.0, 0.5),
        end: GeoPoint::new(1.0, 0.5),
        stops: vec![
            GradientStop::new(0.0, Color::new(0.0, 0.0, 0.8, 1.0)),
            GradientStop::new(0.5, Color::new(0.0, 0.5, 1.0, 1.0)),
            GradientStop::new(1.0, Color::new(0.5, 1.0, 1.0, 1.0)),
        ],
    })
}

fn gradient_neon() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.0, 0.0),
        end: GeoPoint::new(1.0, 1.0),
        stops: vec![
            GradientStop::new(0.0, Color::new(1.0, 0.0, 1.0, 1.0)),
            GradientStop::new(0.5, Color::new(0.0, 1.0, 1.0, 1.0)),
            GradientStop::new(1.0, Color::new(1.0, 1.0, 0.0, 1.0)),
        ],
    })
}

fn gradient_earth() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.5, 0.0),
        end: GeoPoint::new(0.5, 1.0),
        stops: vec![
            GradientStop::new(0.0, Color::new(0.6, 0.4, 0.2, 1.0)),
            GradientStop::new(0.5, Color::new(0.4, 0.5, 0.2, 1.0)),
            GradientStop::new(1.0, Color::new(0.2, 0.3, 0.1, 1.0)),
        ],
    })
}

fn gradient_pastel() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.0, 0.5),
        end: GeoPoint::new(1.0, 0.5),
        stops: vec![
            GradientStop::new(0.0, Color::new(1.0, 0.8, 0.9, 1.0)),
            GradientStop::new(0.33, Color::new(0.8, 0.9, 1.0, 1.0)),
            GradientStop::new(0.66, Color::new(0.9, 1.0, 0.8, 1.0)),
            GradientStop::new(1.0, Color::new(1.0, 0.9, 0.8, 1.0)),
        ],
    })
}

fn gradient_dark() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.5, 0.0),
        end: GeoPoint::new(0.5, 1.0),
        stops: vec![
            GradientStop::new(0.0, Color::new(0.3, 0.3, 0.4, 1.0)),
            GradientStop::new(0.5, Color::new(0.15, 0.15, 0.2, 1.0)),
            GradientStop::new(1.0, Color::new(0.05, 0.05, 0.1, 1.0)),
        ],
    })
}

fn gradient_metal() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.5, 0.0),
        end: GeoPoint::new(0.5, 1.0),
        stops: vec![
            GradientStop::new(0.0, Color::new(0.9, 0.9, 0.95, 1.0)),
            GradientStop::new(0.3, Color::new(0.6, 0.6, 0.65, 1.0)),
            GradientStop::new(0.5, Color::new(0.9, 0.9, 0.9, 1.0)),
            GradientStop::new(0.7, Color::new(0.5, 0.5, 0.55, 1.0)),
            GradientStop::new(1.0, Color::new(0.7, 0.7, 0.75, 1.0)),
        ],
    })
}

fn gradient_candy() -> Fill {
    Fill::LinearGradient(LinearGradient {
        start: GeoPoint::new(0.0, 0.0),
        end: GeoPoint::new(1.0, 1.0),
        stops: vec![
            GradientStop::new(0.0, Color::new(1.0, 0.4, 0.6, 1.0)),
            GradientStop::new(0.25, Color::new(1.0, 1.0, 1.0, 1.0)),
            GradientStop::new(0.5, Color::new(1.0, 0.4, 0.6, 1.0)),
            GradientStop::new(0.75, Color::new(1.0, 1.0, 1.0, 1.0)),
            GradientStop::new(1.0, Color::new(1.0, 0.4, 0.6, 1.0)),
        ],
    })
}
