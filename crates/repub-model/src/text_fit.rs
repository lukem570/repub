//! Text fit calculations for auto-sizing text to fit frames.

use repub_geometry::Rect;

use crate::shape::{TextFitMode, TextFrameData};

/// Minimum font scale factor (20% of original size).
pub const MIN_SCALE: f64 = 0.2;

/// Maximum font scale factor (500% of original size).
pub const MAX_SCALE: f64 = 5.0;

/// Configuration for text fitting calculations.
#[derive(Debug, Clone, Copy)]
pub struct TextFitConfig {
    /// Smallest allowed font scale factor.
    pub min_scale: f64,
    /// Largest allowed font scale factor.
    pub max_scale: f64,
    /// Relative tolerance for the binary search.
    pub tolerance: f64,
    /// Maximum binary-search iterations.
    pub max_iterations: u32,
}

impl Default for TextFitConfig {
    fn default() -> Self {
        Self {
            min_scale: MIN_SCALE,
            max_scale: MAX_SCALE,
            tolerance: 0.01,
            max_iterations: 20,
        }
    }
}

/// Result of a text fit calculation.
#[derive(Debug, Clone, Copy)]
pub struct TextFitResult {
    /// Font scale factor to apply.
    pub scale: f64,
    /// Whether the text still overflows at this scale.
    pub overflows: bool,
    /// Measured text height at this scale, in points.
    pub text_height: f64,
    /// Available frame height, in points.
    pub frame_height: f64,
}

impl TextFitResult {
    /// A result that leaves the scale unchanged.
    pub fn no_change(text_height: f64, frame_height: f64) -> Self {
        Self {
            scale: 1.0,
            overflows: text_height > frame_height,
            text_height,
            frame_height,
        }
    }
}

/// Calculate the font scale factor for a text frame based on its fit mode.
///
/// `bounds` is the frame's on-page rectangle (from the enclosing shape).
pub fn calculate_fit_scale<F>(
    tf: &TextFrameData,
    bounds: Rect,
    config: &TextFitConfig,
    measure_height: F,
) -> TextFitResult
where
    F: Fn(f64) -> f64,
{
    let content_rect = tf.content_rect(bounds);
    let frame_height = content_rect.height();

    if frame_height <= 0.0 {
        return TextFitResult::no_change(0.0, 0.0);
    }

    let base_height = measure_height(1.0);

    match tf.text_fit {
        TextFitMode::None => TextFitResult::no_change(base_height, frame_height),

        TextFitMode::ShrinkOnOverflow => {
            if base_height <= frame_height {
                TextFitResult::no_change(base_height, frame_height)
            } else {
                let scale =
                    find_fit_scale(frame_height, config.min_scale, 1.0, config, &measure_height);
                TextFitResult {
                    scale,
                    overflows: false,
                    text_height: measure_height(scale),
                    frame_height,
                }
            }
        }

        TextFitMode::GrowToFit => {
            if base_height >= frame_height {
                TextFitResult::no_change(base_height, frame_height)
            } else {
                let scale =
                    find_fit_scale(frame_height, 1.0, config.max_scale, config, &measure_height);
                let final_height = measure_height(scale);
                TextFitResult {
                    scale,
                    overflows: final_height > frame_height,
                    text_height: final_height,
                    frame_height,
                }
            }
        }

        TextFitMode::BestFit => {
            let scale = find_fit_scale(
                frame_height,
                config.min_scale,
                config.max_scale,
                config,
                &measure_height,
            );
            let final_height = measure_height(scale);
            TextFitResult {
                scale,
                overflows: final_height > frame_height,
                text_height: final_height,
                frame_height,
            }
        }
    }
}

fn find_fit_scale<F>(
    target_height: f64,
    min_scale: f64,
    max_scale: f64,
    config: &TextFitConfig,
    measure_height: &F,
) -> f64
where
    F: Fn(f64) -> f64,
{
    let mut low = min_scale;
    let mut high = max_scale;
    let tolerance = config.tolerance * target_height;

    for _ in 0..config.max_iterations {
        let mid = (low + high) / 2.0;
        let height = measure_height(mid);

        if (height - target_height).abs() < tolerance {
            return mid;
        }

        if height > target_height {
            high = mid;
        } else {
            low = mid;
        }
    }

    low
}

/// Apply a scale factor to all font sizes in a text frame.
pub fn apply_scale(tf: &mut TextFrameData, scale: f64) {
    if (scale - 1.0).abs() < 0.001 {
        return;
    }

    for para in &mut tf.content.paragraphs {
        para.style.default_char_style.font_size =
            (para.style.default_char_style.font_size * scale).max(1.0);

        for element in &mut para.elements {
            match element {
                crate::text::InlineElement::Text(run) => {
                    run.style.font_size = (run.style.font_size * scale).max(1.0);
                }
                crate::text::InlineElement::Link(link) => {
                    link.style.font_size = (link.style.font_size * scale).max(1.0);
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BOUNDS: Rect = Rect::new(0.0, 0.0, 200.0, 100.0);

    #[test]
    fn test_no_fit_mode() {
        let tf = TextFrameData::new();
        let config = TextFitConfig::default();
        let result = calculate_fit_scale(&tf, BOUNDS, &config, |_| 150.0);
        assert!((result.scale - 1.0).abs() < 0.001);
        assert!(result.overflows);
    }

    #[test]
    fn test_shrink_on_overflow() {
        let mut tf = TextFrameData::new();
        tf.text_fit = TextFitMode::ShrinkOnOverflow;
        tf.inset = 0.0;
        let config = TextFitConfig::default();
        let result = calculate_fit_scale(&tf, BOUNDS, &config, |scale| 200.0 * scale);
        assert!(result.scale < 1.0);
        assert!(result.scale > 0.4);
    }

    #[test]
    fn test_grow_to_fit() {
        let mut tf = TextFrameData::new();
        tf.text_fit = TextFitMode::GrowToFit;
        tf.inset = 0.0;
        let config = TextFitConfig::default();
        let result = calculate_fit_scale(&tf, BOUNDS, &config, |scale| 50.0 * scale);
        assert!(result.scale > 1.5);
        assert!(result.scale < 2.5);
    }

    #[test]
    fn test_best_fit() {
        let mut tf = TextFrameData::new();
        tf.text_fit = TextFitMode::BestFit;
        tf.inset = 0.0;
        let config = TextFitConfig::default();
        let result = calculate_fit_scale(&tf, BOUNDS, &config, |scale| 80.0 * scale);
        assert!(result.scale > 1.0);
        assert!(result.scale < 1.5);
    }
}
