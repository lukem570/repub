//! Procedural background texture generation.
//!
//! Generates a seamless, grainy background texture similar to Gwenview's,
//! tinted light gray to match Publisher's workspace.
//! The noise pattern uses a spatial filter that makes each pixel tend to be
//! opposite of its cardinal neighbors and similar to diagonal ones, creating
//! a fine, even grain without clumps or blotches.

use egui::{Color32, ColorImage, TextureHandle, TextureOptions};
use std::sync::OnceLock;

/// Cached background texture.
static BACKGROUND_TEXTURE: OnceLock<TextureData> = OnceLock::new();

struct TextureData {
    pixels: Vec<u8>,
    size: usize,
}

/// Background texture parameters.
#[derive(Debug, Clone, Copy)]
pub struct BackgroundParams {
    /// Texture tile size in pixels.
    pub size: usize,
    /// Base brightness (0-255). Default 228, just below Publisher's #F1F1F1
    /// so the white page still stands out.
    pub base: f32,
    /// Noise grain strength. Default ~3.2.
    pub grain: f32,
    /// Random seed for reproducible textures.
    pub seed: u64,
}

impl Default for BackgroundParams {
    fn default() -> Self {
        Self {
            size: 256,
            base: 228.0,
            grain: 4.0,
            seed: 42,
        }
    }
}

/// Simple xorshift64 PRNG for reproducible noise.
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed.max(1) }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// Generate a value from standard normal distribution using Box-Muller.
    fn normal(&mut self) -> f32 {
        let u1 = (self.next_u64() as f64 / u64::MAX as f64).max(1e-10);
        let u2 = self.next_u64() as f64 / u64::MAX as f64;
        let z = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
        z as f32
    }
}

/// Generate the background texture pixels.
fn generate_texture(params: &BackgroundParams) -> Vec<u8> {
    let size = params.size;
    let mut rng = Rng::new(params.seed);

    // Generate base white noise
    let w: Vec<f32> = (0..size * size).map(|_| rng.normal()).collect();

    // Apply spatial filter: subtract cardinal neighbors, add diagonal neighbors
    // This creates the fine-grain pattern without clumps
    let mut noise = vec![0.0f32; size * size];

    for y in 0..size {
        for x in 0..size {
            let idx = y * size + x;

            // Wrap-around indices for seamless tiling
            let x_prev = if x == 0 { size - 1 } else { x - 1 };
            let x_next = if x == size - 1 { 0 } else { x + 1 };
            let y_prev = if y == 0 { size - 1 } else { y - 1 };
            let y_next = if y == size - 1 { 0 } else { y + 1 };

            // Cardinal neighbors (subtract)
            let cross = w[y * size + x_next]
                + w[y * size + x_prev]
                + w[y_next * size + x]
                + w[y_prev * size + x];

            // Diagonal neighbors (add)
            let diag = w[y_next * size + x_next]
                + w[y_next * size + x_prev]
                + w[y_prev * size + x_next]
                + w[y_prev * size + x_prev];

            noise[idx] = w[idx] - 0.1 * cross + 0.1 * diag;
        }
    }

    // Normalize to unit standard deviation
    let mean: f32 = noise.iter().sum::<f32>() / noise.len() as f32;
    let variance: f32 = noise.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / noise.len() as f32;
    let std_dev = variance.sqrt().max(0.001);

    for n in &mut noise {
        *n = (*n - mean) / std_dev;
    }

    // Convert to RGB pixels
    let mut pixels = Vec::with_capacity(size * size * 4);
    for n in &noise {
        let v = (params.base + params.grain * n).clamp(0.0, 255.0).round() as u8;
        pixels.push(v); // R
        pixels.push(v); // G
        pixels.push(v); // B
        pixels.push(255); // A
    }

    pixels
}

/// Get or create the background texture.
pub fn get_background_texture(ctx: &egui::Context) -> TextureHandle {
    let data = BACKGROUND_TEXTURE.get_or_init(|| {
        let params = BackgroundParams::default();
        TextureData {
            pixels: generate_texture(&params),
            size: params.size,
        }
    });

    let image = ColorImage::from_rgba_unmultiplied([data.size, data.size], &data.pixels);

    ctx.load_texture("workspace_background", image, TextureOptions::LINEAR)
}

/// Paint the workspace background with the grainy texture.
pub fn paint_workspace_background(
    painter: &egui::Painter,
    rect: egui::Rect,
    texture: &TextureHandle,
    base_color: Color32,
) {
    let texture_size = texture.size_vec2();

    // Calculate UV coordinates for tiling
    let uv_scale = egui::vec2(
        rect.width() / texture_size.x,
        rect.height() / texture_size.y,
    );

    // Create a mesh with the tiled texture
    let mut mesh = egui::Mesh::with_texture(texture.id());

    // Add a quad covering the entire rect
    let uv_rect =
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(uv_scale.x, uv_scale.y));

    mesh.add_rect_with_uv(rect, uv_rect, base_color);

    painter.add(egui::Shape::mesh(mesh));
}

/// Paint workspace with panning texture.
/// The texture scrolls with the pan offset, creating a parallax effect.
pub fn paint_workspace_with_pan(
    painter: &egui::Painter,
    rect: egui::Rect,
    texture: &TextureHandle,
    pan_offset: egui::Vec2,
) {
    let tile_size = 256.0; // Fixed tile size for consistent look

    // Offset the tiling grid by the pan amount (creates the scrolling effect)
    // We negate pan_offset so the texture moves opposite to the view movement
    let offset_x = (-pan_offset.x).rem_euclid(tile_size);
    let offset_y = (-pan_offset.y).rem_euclid(tile_size);

    // Start tiling from before the visible area to cover edges
    let start_x = rect.left() - offset_x - tile_size;
    let start_y = rect.top() - offset_y - tile_size;

    let mut x = start_x;
    while x < rect.right() + tile_size {
        let mut y = start_y;
        while y < rect.bottom() + tile_size {
            let tile_rect =
                egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(tile_size, tile_size));

            // Clip to workspace bounds
            let clipped = tile_rect.intersect(rect);
            if clipped.width() > 0.0 && clipped.height() > 0.0 {
                // Calculate UV for the clipped portion
                let uv_min = egui::pos2(
                    (clipped.left() - tile_rect.left()) / tile_size,
                    (clipped.top() - tile_rect.top()) / tile_size,
                );
                let uv_max = egui::pos2(
                    (clipped.right() - tile_rect.left()) / tile_size,
                    (clipped.bottom() - tile_rect.top()) / tile_size,
                );

                // The texture is light grey; dark mode scales it down to a
                // charcoal workspace, grain and all
                let tint = if crate::theme::colors::is_dark() {
                    Color32::from_gray(46)
                } else {
                    Color32::WHITE
                };
                let mut mesh = egui::Mesh::with_texture(texture.id());
                mesh.add_rect_with_uv(clipped, egui::Rect::from_min_max(uv_min, uv_max), tint);
                painter.add(egui::Shape::mesh(mesh));
            }

            y += tile_size;
        }
        x += tile_size;
    }
}

/// Paint workspace with simple tinted texture (stationary, no pan).
pub fn paint_workspace_simple(painter: &egui::Painter, rect: egui::Rect, texture: &TextureHandle) {
    paint_workspace_with_pan(painter, rect, texture, egui::Vec2::ZERO);
}
