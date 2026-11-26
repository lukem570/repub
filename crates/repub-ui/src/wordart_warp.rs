//! WordArt text warp rendering.
//!
//! Implements text warping by rendering text as a mesh of triangles and
//! distorting the vertices according to warp envelope paths.
//!
//! This approach mimics how MS Publisher renders WordArt:
//! 1. Text is rendered to fill the bounding box
//! 2. A grid mesh is created over the text
//! 3. Each vertex is mapped from (x%, y%) in the original rectangle to
//!    interpolated position between top and bottom warp paths

use egui::{Color32, Pos2, Rect, Stroke, Vec2};

/// Text warp preset type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WarpPreset {
    Plain,
    Wave1,
    Wave2,
    Wave4,
    DoubleWave1,
    ArchUp,
    ArchDown,
    ArchUpPour,
    ArchDownPour,
    Circle,
    CirclePour,
    Button,
    ButtonPour,
    Triangle,
    TriangleInverted,
    Chevron,
    ChevronInverted,
    FadeRight,
    FadeLeft,
    FadeUp,
    FadeDown,
    SlantUp,
    SlantDown,
    CascadeUp,
    CascadeDown,
    CurveUp,
    CurveDown,
    CanUp,
    CanDown,
    Inflate,
    Deflate,
    InflateBottom,
    InflateTop,
    DeflateBottom,
    DeflateTop,
    DeflateInflate,
    DeflateInflateDeflate,
    RingInside,
    RingOutside,
    Stop,
}

impl Default for WarpPreset {
    fn default() -> Self {
        Self::Plain
    }
}

/// Color source for warped text - either solid or gradient
pub enum WarpFill<'a> {
    Solid(Color32),
    Gradient(&'a repub_model::styles::Fill),
}

/// Sample color from fill at given position (0-1 range)
fn sample_fill_color(fill: &WarpFill<'_>, x_pct: f32, y_pct: f32) -> Color32 {
    match fill {
        WarpFill::Solid(c) => *c,
        WarpFill::Gradient(f) => {
            let color = crate::gradient::color_at(f, x_pct as f64, y_pct as f64);
            crate::canvas::core_color_to_egui(&color, 1.0)
        }
    }
}

/// Get the Y position at a given X percentage (0-1) for top and bottom warp paths.
/// Returns (top_y, bottom_y) in the range that may extend beyond [0, height].
///
/// Per ECMA-376, text warp works by:
/// 1. Each point in the original text rectangle has an (x%, y%) position
/// 2. Find the point at x% along the top path and x% along the bottom path  
/// 3. Draw a line between them, find the point at y% along that line
/// 4. This is the new warped position for that point
pub(crate) fn get_warp_envelope_internal(
    preset: WarpPreset,
    x_pct: f32,
    height: f32,
) -> (f32, f32) {
    let x = x_pct;
    let h = height;

    match preset {
        WarpPreset::Plain => (0.0, h),

        WarpPreset::Wave1 => {
            // Single sine wave
            let amplitude = h * 0.2;
            let wave = (x * std::f32::consts::PI * 2.0).sin() * amplitude;
            (amplitude + wave, h - amplitude + wave)
        }

        WarpPreset::Wave2 => {
            // Single sine wave, opposite phase
            let amplitude = h * 0.2;
            let wave = (x * std::f32::consts::PI * 2.0 + std::f32::consts::PI).sin() * amplitude;
            (amplitude + wave, h - amplitude + wave)
        }

        WarpPreset::Wave4 | WarpPreset::DoubleWave1 => {
            // Double wave
            let amplitude = h * 0.12;
            let wave = (x * std::f32::consts::PI * 4.0).sin() * amplitude;
            (amplitude + wave, h - amplitude + wave)
        }

        WarpPreset::ArchUp | WarpPreset::ArchUpPour => {
            // Arch curving upward - parabola
            let curve = h * 0.4;
            let parabola = 4.0 * curve * x * (1.0 - x);
            (-parabola * 0.3, h - parabola)
        }

        WarpPreset::ArchDown | WarpPreset::ArchDownPour => {
            // Arch curving downward
            let curve = h * 0.4;
            let parabola = 4.0 * curve * x * (1.0 - x);
            (parabola, h + parabola * 0.3)
        }

        WarpPreset::Circle
        | WarpPreset::CirclePour
        | WarpPreset::RingInside
        | WarpPreset::RingOutside => {
            // Both paths curve upward (like text on inside of circle)
            let curve = h * 0.3;
            let arc = 4.0 * curve * x * (1.0 - x);
            (-arc * 0.5, h - arc)
        }

        WarpPreset::Button | WarpPreset::ButtonPour => {
            // Button shape
            let curve = h * 0.25;
            let arc = 4.0 * curve * x * (1.0 - x);
            (-arc * 0.3, h - arc * 0.7)
        }

        WarpPreset::Inflate => {
            // Text bulges outward in the middle
            let bulge = h * 0.3;
            let arc = 4.0 * bulge * x * (1.0 - x);
            (-arc, h + arc)
        }

        WarpPreset::Deflate => {
            // Text pinches inward in the middle
            let pinch = h * 0.25;
            let arc = 4.0 * pinch * x * (1.0 - x);
            (arc, h - arc)
        }

        WarpPreset::InflateBottom => {
            let bulge = h * 0.35;
            let arc = 4.0 * bulge * x * (1.0 - x);
            (0.0, h + arc)
        }

        WarpPreset::InflateTop => {
            let bulge = h * 0.35;
            let arc = 4.0 * bulge * x * (1.0 - x);
            (-arc, h)
        }

        WarpPreset::DeflateBottom => {
            let pinch = h * 0.3;
            let arc = 4.0 * pinch * x * (1.0 - x);
            (0.0, h - arc)
        }

        WarpPreset::DeflateTop => {
            let pinch = h * 0.3;
            let arc = 4.0 * pinch * x * (1.0 - x);
            (arc, h)
        }

        WarpPreset::Triangle => {
            // Top forms a triangle peak
            let peak = h * 0.45;
            let top = if x < 0.5 {
                peak * (1.0 - x * 2.0)
            } else {
                peak * ((x - 0.5) * 2.0)
            };
            (top, h)
        }

        WarpPreset::TriangleInverted => {
            // Bottom forms a triangle
            let peak = h * 0.45;
            let bottom = if x < 0.5 {
                h - peak + peak * x * 2.0
            } else {
                h - peak * (1.0 - (x - 0.5) * 2.0)
            };
            (0.0, bottom)
        }

        WarpPreset::Chevron => {
            // Both top and bottom form V pointing up
            let depth = h * 0.3;
            let v = if x < 0.5 {
                depth * (1.0 - x * 2.0)
            } else {
                depth * ((x - 0.5) * 2.0)
            };
            (v, h - depth + v)
        }

        WarpPreset::ChevronInverted => {
            // Both form V pointing down
            let depth = h * 0.3;
            let v = if x < 0.5 {
                depth * x * 2.0
            } else {
                depth * (1.0 - (x - 0.5) * 2.0)
            };
            (v, h - depth + v)
        }

        WarpPreset::FadeRight => {
            // Shrinks toward the right
            let shrink = h * 0.45;
            let top = shrink * x;
            let bottom = h - shrink * x;
            (top, bottom)
        }

        WarpPreset::FadeLeft => {
            // Shrinks toward the left
            let shrink = h * 0.45;
            let top = shrink * (1.0 - x);
            let bottom = h - shrink * (1.0 - x);
            (top, bottom)
        }

        WarpPreset::FadeUp | WarpPreset::FadeDown => {
            // These affect width, approximate with height
            (0.0, h)
        }

        WarpPreset::SlantUp => {
            // Slants upward
            let slant = h * 0.35;
            let offset = slant * (1.0 - x);
            (offset, h - slant * x)
        }

        WarpPreset::SlantDown => {
            // Slants downward
            let slant = h * 0.35;
            let offset = slant * x;
            (offset, h - slant * (1.0 - x))
        }

        WarpPreset::CascadeUp => {
            // Gets smaller going right, tilting up
            let shrink = h * 0.4;
            (shrink * (1.0 - x), h - shrink * x)
        }

        WarpPreset::CascadeDown => {
            // Gets smaller going right, tilting down
            let shrink = h * 0.4;
            (shrink * x, h - shrink * (1.0 - x))
        }

        WarpPreset::CurveUp => {
            let curve = h * 0.25;
            let arc = 4.0 * curve * x * (1.0 - x);
            (-arc * 0.4, h - arc)
        }

        WarpPreset::CurveDown => {
            let curve = h * 0.25;
            let arc = 4.0 * curve * x * (1.0 - x);
            (arc, h + arc * 0.4)
        }

        WarpPreset::CanUp => {
            // Per ECMA-376 textCanUp: both paths curve upward
            // adj default = 14286 (14.3% of height), curves upward
            let adj = 0.143; // 14.3%
            let dy = h * adj;
            // Parabolic arc approximation curving upward
            let arc = 4.0 * dy * x * (1.0 - x);
            // Top path curves up (negative), bottom path curves up (negative)
            (0.0 - arc, h - arc)
        }

        WarpPreset::CanDown => {
            // Per ECMA-376 textCanDown: both paths curve downward
            // adj default = 14286 (14.3% of height), curves downward
            let adj = 0.143; // 14.3%
            let dy = h * adj;
            // Parabolic arc approximation curving downward
            let arc = 4.0 * dy * x * (1.0 - x);
            // Top path curves down (positive), bottom path curves down (positive)
            (arc, h + arc)
        }

        WarpPreset::Stop => {
            // Octagonal
            let inset = h * 0.2;
            let third = 1.0 / 3.0;
            if x < third {
                let t = x / third;
                (inset * (1.0 - t), h - inset * (1.0 - t))
            } else if x > 1.0 - third {
                let t = (x - (1.0 - third)) / third;
                (inset * t, h - inset * t)
            } else {
                (0.0, h)
            }
        }

        WarpPreset::DeflateInflate | WarpPreset::DeflateInflateDeflate => {
            // Complex wave pattern
            let amp = h * 0.15;
            let wave = (x * std::f32::consts::PI * 3.0).sin() * amp;
            (amp + wave, h - amp + wave)
        }
    }
}

/// Render warped text with solid color (convenience wrapper)
pub fn render_warped_text(
    painter: &egui::Painter,
    text: &str,
    screen_rect: Rect,
    preset: WarpPreset,
    fill_color: Color32,
    outline: Option<(Color32, f32)>,
    _adj1: f32,
    _adj2: f32,
    rotation: f32,
) {
    render_warped_text_with_fill(
        painter,
        text,
        screen_rect,
        preset,
        &WarpFill::Solid(fill_color),
        outline,
        _adj1,
        _adj2,
        rotation,
    );
}

/// Render warped text using a grid-cell clipping approach.
///
/// This approach approximates how MS Publisher renders WordArt:
/// 1. Publisher converts text to paths using GdipAddPathString
/// 2. Each Bezier control point is transformed based on warp envelope
///
/// Since egui doesn't expose glyph paths, we approximate by:
/// 1. Rendering the full text galley at a consistent size
/// 2. Dividing the shape into a grid of small cells
/// 3. Each cell shows a clipped portion of the text, offset according to the warp envelope
/// 4. The y-offset for each cell is calculated based on its (x%, y%) position in the grid
pub fn render_warped_text_with_fill(
    painter: &egui::Painter,
    text: &str,
    screen_rect: Rect,
    preset: WarpPreset,
    fill: &WarpFill<'_>,
    outline: Option<(Color32, f32)>,
    _adj1: f32,
    _adj2: f32,
    _rotation: f32,
) {
    if text.is_empty() || screen_rect.width() < 1.0 || screen_rect.height() < 1.0 {
        return;
    }

    // For plain preset, just render normally
    if preset == WarpPreset::Plain {
        render_plain_text_internal(painter, text, screen_rect, fill, outline);
        return;
    }

    let width = screen_rect.width();
    let height = screen_rect.height();

    // Calculate min envelope height for sizing (text needs to fit in narrowest part)
    let mut min_envelope_height = height;
    let samples = 20;
    for i in 0..samples {
        let x_pct = i as f32 / (samples - 1) as f32;
        let (top, bottom) = get_warp_envelope_internal(preset, x_pct, height);
        min_envelope_height = min_envelope_height.min(bottom - top);
    }
    min_envelope_height = min_envelope_height.max(height * 0.15);

    // Calculate base font size to fill width while fitting in min envelope height
    let measure_size = 100.0_f32;
    let sample_color = sample_fill_color(fill, 0.5, 0.5);

    let measure_galley = painter.layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(measure_size),
        sample_color,
    );

    let text_width = measure_galley.rect.width().max(1.0);
    let text_height = measure_galley.rect.height().max(1.0);

    // Scale to fill width with padding, but cap at min envelope height
    let padding_x = width * 0.03;
    let target_width = width - padding_x * 2.0;
    let scale_w = target_width / text_width;
    let scale_h = min_envelope_height / text_height;
    let scale = scale_w.min(scale_h).min(10.0);
    let base_font_size = (measure_size * scale).clamp(8.0, 400.0);

    // Create the base galley
    let font_id = egui::FontId::proportional(base_font_size);
    let base_galley = painter.layout_no_wrap(text.to_string(), font_id.clone(), sample_color);
    let galley_width = base_galley.rect.width();
    let galley_height = base_galley.rect.height();

    if galley_width < 1.0 || galley_height < 1.0 {
        return;
    }

    // Calculate text position centered in the shape (this is the "nominal" position)
    let text_center_x = screen_rect.center().x;
    let text_center_y = screen_rect.center().y;
    let text_left = text_center_x - galley_width / 2.0;
    let text_top = text_center_y - galley_height / 2.0;

    // Grid dimensions - more cells = smoother warp
    let num_cols = ((galley_width / 3.0) as usize).clamp(30, 150);
    let num_rows = ((galley_height / 3.0) as usize).clamp(10, 50);
    let cell_width = galley_width / num_cols as f32;
    let cell_height = galley_height / num_rows as f32;

    // Render grid cells - iterate by column first for better gradient appearance
    for col in 0..num_cols {
        let x_pct = (col as f32 + 0.5) / num_cols as f32;

        // Get warp envelope at this x position
        let (env_top, env_bottom) = get_warp_envelope_internal(preset, x_pct, height);
        let envelope_height = env_bottom - env_top;

        if envelope_height < 1.0 {
            continue;
        }

        for row in 0..num_rows {
            let y_pct = (row as f32 + 0.5) / num_rows as f32;

            // Position of this cell in the original text coordinates (relative to text_left, text_top)
            let orig_cell_x = col as f32 * cell_width;
            let orig_cell_y = row as f32 * cell_height;

            // Per ECMA-376 warp algorithm:
            // Find point at x% along top envelope path → (x, env_top)
            // Find point at x% along bottom envelope path → (x, env_bottom)
            // Interpolate between them at y% to get the warped position
            let warped_y = env_top + y_pct * envelope_height;

            // Screen position for this cell
            let screen_x = text_left + orig_cell_x;
            let screen_y = screen_rect.top() + warped_y - cell_height / 2.0;

            // Clip rect on screen
            let clip_rect = Rect::from_min_size(
                Pos2::new(screen_x, screen_y),
                Vec2::new(cell_width + 1.0, cell_height + 1.0),
            );

            // Position text so the correct portion appears in the clip rect
            // The text position plus the cell's original position should equal the screen position
            // text_pos + orig_cell = screen_pos
            // text_pos = screen_pos - orig_cell
            let draw_text_x = screen_x - orig_cell_x + text_left;
            let draw_text_y = screen_y - orig_cell_y + text_top;

            // But wait - we want to show cell (col, row) of the TEXT at screen position (screen_x, screen_y)
            // The text is at (text_left, text_top), so cell (col, row) is at (text_left + orig_cell_x, text_top + orig_cell_y)
            // To have that show in clip_rect at (screen_x, screen_y), we need:
            // text_draw_pos + orig_cell = clip_rect.min
            // So: text_draw_pos = clip_rect.min - orig_cell = (screen_x - orig_cell_x, screen_y - orig_cell_y)
            // But the text is already at (text_left, text_top), so relative offset is:
            // Actually simpler: just position the text so its cell aligns with the clip rect
            let final_text_x = screen_x - orig_cell_x;
            let final_text_y = screen_y - orig_cell_y;

            // Sample color for gradient
            let cell_color = sample_fill_color(fill, x_pct, y_pct);

            // Draw outline first
            if let Some((outline_color, outline_width)) = outline {
                let outline_galley =
                    painter.layout_no_wrap(text.to_string(), font_id.clone(), outline_color);

                for (dx, dy) in [(-1.0, 0.0), (1.0, 0.0), (0.0, -1.0), (0.0, 1.0)] {
                    painter.with_clip_rect(clip_rect).add(egui::Shape::Text(
                        egui::epaint::TextShape {
                            pos: Pos2::new(
                                final_text_x + dx * outline_width,
                                final_text_y + dy * outline_width,
                            ),
                            galley: outline_galley.clone(),
                            underline: Stroke::NONE,
                            fallback_color: outline_color,
                            override_text_color: Some(outline_color),
                            opacity_factor: 1.0,
                            angle: 0.0,
                        },
                    ));
                }
            }

            // Draw fill
            let fill_galley = if matches!(fill, WarpFill::Gradient(_)) {
                painter.layout_no_wrap(text.to_string(), font_id.clone(), cell_color)
            } else {
                base_galley.clone()
            };

            painter
                .with_clip_rect(clip_rect)
                .add(egui::Shape::Text(egui::epaint::TextShape {
                    pos: Pos2::new(final_text_x, final_text_y),
                    galley: fill_galley,
                    underline: Stroke::NONE,
                    fallback_color: cell_color,
                    override_text_color: Some(cell_color),
                    opacity_factor: 1.0,
                    angle: 0.0,
                }));
        }
    }
}

/// Render plain (non-warped) text with gradient support
pub(crate) fn render_plain_text_internal(
    painter: &egui::Painter,
    text: &str,
    screen_rect: Rect,
    fill: &WarpFill<'_>,
    outline: Option<(Color32, f32)>,
) {
    // Calculate font size to fill the bounds
    let measure_size = 72.0_f32;
    let sample_color = sample_fill_color(fill, 0.5, 0.5);
    let measure_galley = painter.layout_no_wrap(
        text.to_string(),
        egui::FontId::proportional(measure_size),
        sample_color,
    );

    let text_w = measure_galley.rect.width().max(1.0);
    let text_h = measure_galley.rect.height().max(1.0);

    let padding = screen_rect.width() * 0.02;
    let avail_w = (screen_rect.width() - padding * 2.0).max(1.0);
    let avail_h = (screen_rect.height() - padding * 2.0).max(1.0);
    let scale = (avail_w / text_w).min(avail_h / text_h).min(10.0);
    let font_size = (measure_size * scale).max(6.0);

    let center = screen_rect.center();
    let font_id = egui::FontId::proportional(font_size);

    // Check if gradient
    let is_gradient = matches!(fill, WarpFill::Gradient(_));

    // Draw outline
    if let Some((outline_color, outline_width)) = outline {
        let outline_galley =
            painter.layout_no_wrap(text.to_string(), font_id.clone(), outline_color);
        let gw = outline_galley.rect.width();
        let gh = outline_galley.rect.height();

        let offsets = [
            (-1.0, 0.0),
            (1.0, 0.0),
            (0.0, -1.0),
            (0.0, 1.0),
            (-0.707, -0.707),
            (0.707, -0.707),
            (-0.707, 0.707),
            (0.707, 0.707),
        ];

        for (dx, dy) in offsets {
            painter.add(egui::Shape::Text(egui::epaint::TextShape {
                pos: Pos2::new(
                    center.x - gw / 2.0 + dx * outline_width,
                    center.y - gh / 2.0 + dy * outline_width,
                ),
                galley: outline_galley.clone(),
                underline: Stroke::NONE,
                fallback_color: outline_color,
                override_text_color: Some(outline_color),
                opacity_factor: 1.0,
                angle: 0.0,
            }));
        }
    }

    // Draw fill with gradient slices if needed
    if is_gradient {
        let galley = painter.layout_no_wrap(text.to_string(), font_id.clone(), sample_color);
        let galley_height = galley.rect.height();
        let galley_width = galley.rect.width();

        let num_slices = 25;
        let slice_height = galley_height / num_slices as f32;

        for i in 0..num_slices {
            let v = i as f32 / (num_slices - 1) as f32;
            let slice_color = sample_fill_color(fill, 0.5, v);

            let slice_galley =
                painter.layout_no_wrap(text.to_string(), font_id.clone(), slice_color);
            let slice_top = center.y - galley_height / 2.0 + i as f32 * slice_height;

            let clip_rect = Rect::from_min_size(
                Pos2::new(center.x - galley_width / 2.0 - 10.0, slice_top),
                Vec2::new(galley_width + 20.0, slice_height + 1.0),
            );

            painter
                .with_clip_rect(clip_rect)
                .add(egui::Shape::Text(egui::epaint::TextShape {
                    pos: Pos2::new(
                        center.x - galley_width / 2.0,
                        center.y - galley_height / 2.0,
                    ),
                    galley: slice_galley,
                    underline: Stroke::NONE,
                    fallback_color: slice_color,
                    override_text_color: Some(slice_color),
                    opacity_factor: 1.0,
                    angle: 0.0,
                }));
        }
    } else {
        let fill_galley = painter.layout_no_wrap(text.to_string(), font_id, sample_color);
        let gw = fill_galley.rect.width();
        let gh = fill_galley.rect.height();

        painter.add(egui::Shape::Text(egui::epaint::TextShape {
            pos: Pos2::new(center.x - gw / 2.0, center.y - gh / 2.0),
            galley: fill_galley,
            underline: Stroke::NONE,
            fallback_color: sample_color,
            override_text_color: Some(sample_color),
            opacity_factor: 1.0,
            angle: 0.0,
        }));
    }
}

impl WarpPreset {
    /// Create warp preset from MSOSPT shape type
    pub fn from_spt(spt: u16) -> Option<Self> {
        match spt {
            136 => Some(Self::Plain),
            137 => Some(Self::Stop),
            138 => Some(Self::Triangle),
            139 => Some(Self::TriangleInverted),
            140 => Some(Self::Chevron),
            141 => Some(Self::ChevronInverted),
            142 => Some(Self::RingInside),
            143 => Some(Self::RingOutside),
            144 => Some(Self::ArchUp),
            145 => Some(Self::ArchDown),
            146 => Some(Self::Circle),
            147 => Some(Self::Button),
            148 => Some(Self::ArchUpPour),
            149 => Some(Self::ArchDownPour),
            150 => Some(Self::CirclePour),
            151 => Some(Self::ButtonPour),
            152 => Some(Self::CurveUp),
            153 => Some(Self::CurveDown),
            154 => Some(Self::CascadeUp),
            155 => Some(Self::CascadeDown),
            156 => Some(Self::Wave1),
            157 => Some(Self::Wave2),
            158 => Some(Self::Wave1),
            159 => Some(Self::Wave4),
            160 => Some(Self::Inflate),
            161 => Some(Self::Deflate),
            162 => Some(Self::InflateBottom),
            163 => Some(Self::DeflateBottom),
            164 => Some(Self::InflateTop),
            165 => Some(Self::DeflateTop),
            166 => Some(Self::DeflateInflate),
            167 => Some(Self::DeflateInflateDeflate),
            168 => Some(Self::FadeRight),
            169 => Some(Self::FadeLeft),
            170 => Some(Self::FadeUp),
            171 => Some(Self::FadeDown),
            172 => Some(Self::SlantUp),
            173 => Some(Self::SlantDown),
            174 => Some(Self::CanUp),
            175 => Some(Self::CanDown),
            // Legacy types
            24 => Some(Self::Plain),
            25 => Some(Self::Stop),
            26 => Some(Self::Triangle),
            27 => Some(Self::CurveDown),
            28 => Some(Self::Wave1),
            29 => Some(Self::RingInside),
            30 => Some(Self::CurveUp),
            31 => Some(Self::ArchUp),
            _ => None,
        }
    }
}
