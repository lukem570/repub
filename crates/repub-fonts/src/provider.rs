//! Font loading, metrics, and the [`FontProvider`] trait.

use std::collections::HashMap;
use std::sync::Arc;

use fontdue::{Font, FontSettings};

use crate::error::{FontError, FontResult};

/// Common kerning pairs and their adjustments (in em units).
/// These are scaled by font size when applied.
static COMMON_KERNING_PAIRS: &[((char, char), f64)] = &[
    // Capital letters with lowercase vowels
    (('A', 'v'), -0.05),
    (('A', 'w'), -0.03),
    (('A', 'y'), -0.05),
    (('A', 'V'), -0.08),
    (('A', 'W'), -0.05),
    (('A', 'Y'), -0.08),
    (('A', 'T'), -0.08),
    // T combinations
    (('T', 'a'), -0.08),
    (('T', 'e'), -0.08),
    (('T', 'i'), -0.03),
    (('T', 'o'), -0.08),
    (('T', 'u'), -0.05),
    (('T', 'y'), -0.03),
    (('T', 'A'), -0.08),
    // V combinations
    (('V', 'a'), -0.08),
    (('V', 'e'), -0.08),
    (('V', 'i'), -0.03),
    (('V', 'o'), -0.08),
    (('V', 'u'), -0.05),
    (('V', 'A'), -0.08),
    // W combinations
    (('W', 'a'), -0.05),
    (('W', 'e'), -0.03),
    (('W', 'i'), -0.02),
    (('W', 'o'), -0.03),
    (('W', 'A'), -0.05),
    // Y combinations
    (('Y', 'a'), -0.08),
    (('Y', 'e'), -0.08),
    (('Y', 'i'), -0.03),
    (('Y', 'o'), -0.08),
    (('Y', 'u'), -0.05),
    (('Y', 'A'), -0.08),
    // F combinations
    (('F', 'a'), -0.05),
    (('F', 'e'), -0.03),
    (('F', 'o'), -0.05),
    // P combinations
    (('P', 'a'), -0.05),
    (('P', 'e'), -0.03),
    (('P', 'o'), -0.05),
    (('P', 'A'), -0.08),
    // L combinations
    (('L', 'T'), -0.08),
    (('L', 'V'), -0.08),
    (('L', 'W'), -0.05),
    (('L', 'Y'), -0.08),
    // r combinations
    (('r', 'a'), -0.02),
    (('r', 'e'), -0.02),
    (('r', 'o'), -0.02),
    (('r', '.'), -0.05),
    (('r', ','), -0.05),
    // f combinations
    (('f', 'f'), -0.02),
    (('f', 'i'), -0.02),
    (('f', 'l'), -0.02),
    (('f', '.'), -0.05),
    (('f', ','), -0.05),
    // Punctuation
    (('.', '"'), -0.08),
    (('.', '\''), -0.08),
    ((',', '"'), -0.08),
    ((',', '\''), -0.08),
    (('"', 'A'), -0.05),
    (('\'', 'A'), -0.05),
];

/// Font metrics for a specific font and size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontMetrics {
    /// Distance from baseline to top of tallest character
    pub ascent: f64,
    /// Distance from baseline to bottom of lowest descender (positive value)
    pub descent: f64,
    /// Recommended line height
    pub line_height: f64,
    /// Height of lowercase 'x'
    pub x_height: f64,
    /// Height of capital letters
    pub cap_height: f64,
    /// Width of space character
    pub space_width: f64,
    /// Underline position (from baseline, positive = below)
    pub underline_position: f64,
    /// Underline thickness
    pub underline_thickness: f64,
}

impl FontMetrics {
    /// Calculate metrics for a font at a given size.
    pub fn from_font(font: &Font, size: f64) -> Self {
        let metrics = font
            .horizontal_line_metrics(size as f32)
            .unwrap_or(fontdue::LineMetrics {
                ascent: size as f32 * 0.8,
                descent: size as f32 * -0.2,
                line_gap: 0.0,
                new_line_size: size as f32,
            });

        // Get space width
        let space_metrics = font.metrics(' ', size as f32);
        let space_width = space_metrics.advance_width as f64;

        // Approximate x-height and cap-height
        let x_metrics = font.metrics('x', size as f32);
        let cap_metrics = font.metrics('H', size as f32);

        Self {
            ascent: metrics.ascent as f64,
            descent: -metrics.descent as f64, // fontdue returns negative
            line_height: metrics.new_line_size as f64,
            x_height: x_metrics.height as f64,
            cap_height: cap_metrics.height as f64,
            space_width,
            underline_position: metrics.descent as f64 * 0.5,
            underline_thickness: (size / 12.0).max(1.0),
        }
    }

    /// Total height from ascent to descent.
    pub fn total_height(&self) -> f64 {
        self.ascent + self.descent
    }
}

/// Character metrics for layout.
#[derive(Debug, Clone, Copy)]
pub struct CharMetrics {
    /// Advance width (how far to move for next character)
    pub advance: f64,
    /// Left side bearing
    pub lsb: f64,
    /// Glyph width
    pub width: f64,
    /// Glyph height
    pub height: f64,
}

/// Font provider trait for abstracting font access.
pub trait FontProvider: Send + Sync {
    /// Get a font by family name and style.
    fn get_font(&self, family: &str, weight: u16, italic: bool) -> Option<Arc<Font>>;

    /// Get the default font.
    fn default_font(&self) -> Arc<Font>;

    /// Get the fallback font chain.
    fn fallback_fonts(&self) -> &[String] {
        &[]
    }

    /// Get a font with fallback support.
    /// Tries the requested font first, then each fallback in order,
    /// finally returning the default font.
    fn get_font_with_fallback(&self, family: &str, weight: u16, italic: bool) -> Arc<Font> {
        // Try exact match first
        if let Some(font) = self.get_font(family, weight, italic) {
            return font;
        }

        // Try each fallback font
        for fallback in self.fallback_fonts() {
            if let Some(font) = self.get_font(fallback, weight, italic) {
                return font;
            }
        }

        // Last resort: default font
        self.default_font()
    }

    /// Measure text with a given font and size, including kerning.
    fn measure_text(&self, text: &str, font: &Font, size: f64) -> f64 {
        let chars: Vec<char> = text.chars().collect();
        if chars.is_empty() {
            return 0.0;
        }

        let mut width = 0.0;
        for i in 0..chars.len() {
            width += font.metrics(chars[i], size as f32).advance_width as f64;

            // Add kerning if there's a next character
            if i + 1 < chars.len() {
                width += self.get_kerning(font, chars[i], chars[i + 1], size);
            }
        }
        width
    }

    /// Measure text without kerning (faster for single characters or when kerning isn't needed).
    fn measure_text_no_kerning(&self, text: &str, font: &Font, size: f64) -> f64 {
        text.chars()
            .map(|c| font.metrics(c, size as f32).advance_width as f64)
            .sum()
    }

    /// Measure a single character.
    fn measure_char(&self, c: char, font: &Font, size: f64) -> CharMetrics {
        let m = font.metrics(c, size as f32);
        CharMetrics {
            advance: m.advance_width as f64,
            lsb: m.bounds.xmin as f64,
            width: m.bounds.width as f64,
            height: m.bounds.height as f64,
        }
    }

    /// Get font metrics for a font at a given size.
    fn get_metrics(&self, font: &Font, size: f64) -> FontMetrics {
        FontMetrics::from_font(font, size)
    }

    /// Get kerning between two characters.
    /// Uses a built-in table of common kerning pairs.
    fn get_kerning(&self, _font: &Font, left: char, right: char, size: f64) -> f64 {
        get_kerning_adjustment(left, right, size)
    }
}

/// Get kerning adjustment from the built-in kerning table.
/// Returns the kerning adjustment in points for the given font size.
pub fn get_kerning_adjustment(left: char, right: char, size: f64) -> f64 {
    for &((l, r), adjustment) in COMMON_KERNING_PAIRS {
        if l == left && r == right {
            return adjustment * size;
        }
    }
    0.0
}

/// A font with its identifying information.
#[derive(Clone)]
pub struct LoadedFont {
    /// The font family name
    pub family: String,
    /// Font weight (100-900)
    pub weight: u16,
    /// Whether the font is italic
    pub italic: bool,
    /// The loaded font
    pub font: Arc<Font>,
}

/// Standard font weights for matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FontWeight {
    /// Thin (100)
    Thin = 100,
    /// Extra Light (200)
    ExtraLight = 200,
    /// Light (300)
    Light = 300,
    /// Normal/Regular (400)
    Normal = 400,
    /// Medium (500)
    Medium = 500,
    /// Semi Bold (600)
    SemiBold = 600,
    /// Bold (700)
    Bold = 700,
    /// Extra Bold (800)
    ExtraBold = 800,
    /// Black (900)
    Black = 900,
}

impl FontWeight {
    /// Convert a numeric weight to the nearest standard weight.
    pub fn from_numeric(weight: u16) -> Self {
        match weight {
            0..=149 => FontWeight::Thin,
            150..=249 => FontWeight::ExtraLight,
            250..=349 => FontWeight::Light,
            350..=449 => FontWeight::Normal,
            450..=549 => FontWeight::Medium,
            550..=649 => FontWeight::SemiBold,
            650..=749 => FontWeight::Bold,
            750..=849 => FontWeight::ExtraBold,
            _ => FontWeight::Black,
        }
    }

    /// Get the numeric value of this weight.
    pub fn as_numeric(self) -> u16 {
        self as u16
    }
}

/// Default fallback font chain for common font families.
pub const DEFAULT_FALLBACK_FONTS: &[&str] = &[
    "Arial",
    "Helvetica",
    "Helvetica Neue",
    "Liberation Sans",
    "DejaVu Sans",
    "sans-serif",
];

/// System font provider that loads fonts from the system.
pub struct SystemFontProvider {
    /// Loaded fonts by cache key
    fonts: HashMap<String, Arc<Font>>,
    /// Font family to weights/styles mapping for smart matching
    font_families: HashMap<String, Vec<(u16, bool)>>,
    /// Default font
    default: Arc<Font>,
    /// Fallback font chain
    fallback_fonts: Vec<String>,
}

impl SystemFontProvider {
    /// Create a new system font provider.
    pub fn new() -> FontResult<Self> {
        // Create a default font (built-in)
        let default = Self::create_default_font()?;

        Ok(Self {
            fonts: HashMap::new(),
            font_families: HashMap::new(),
            default,
            fallback_fonts: DEFAULT_FALLBACK_FONTS
                .iter()
                .map(|s| s.to_string())
                .collect(),
        })
    }

    /// Create a new system font provider with custom fallbacks.
    pub fn with_fallbacks(fallbacks: Vec<String>) -> FontResult<Self> {
        let default = Self::create_default_font()?;

        Ok(Self {
            fonts: HashMap::new(),
            font_families: HashMap::new(),
            default,
            fallback_fonts: fallbacks,
        })
    }

    /// Set the fallback font chain.
    pub fn set_fallback_fonts(&mut self, fallbacks: Vec<String>) {
        self.fallback_fonts = fallbacks;
    }

    /// Create the default font.
    fn create_default_font() -> FontResult<Arc<Font>> {
        // Use fontdue's built-in font for now
        // In a real implementation, we'd load a system font
        let _settings = FontSettings::default();

        // Try to load a basic sans-serif font
        // For now, we embed a minimal font or use system fonts
        #[cfg(target_os = "macos")]
        {
            // Try to load system font on macOS
            if let Some(font) = Self::try_load_system_font() {
                return Ok(font);
            }
        }

        // Fallback to a minimal embedded font
        Self::create_fallback_font()
    }

    #[cfg(target_os = "macos")]
    fn try_load_system_font() -> Option<Arc<Font>> {
        use std::fs;

        // Try common macOS font paths
        let paths = [
            "/System/Library/Fonts/Helvetica.ttc",
            "/System/Library/Fonts/SFNSText.ttf",
            "/Library/Fonts/Arial.ttf",
        ];

        for path in &paths {
            if let Ok(data) = fs::read(path) {
                if let Ok(font) = Font::from_bytes(data, FontSettings::default()) {
                    return Some(Arc::new(font));
                }
            }
        }

        None
    }

    /// The bundled font, used when the system has none to offer (a bare Linux install, the
    /// browser).
    fn create_fallback_font() -> FontResult<Arc<Font>> {
        const FALLBACK_FONT: &[u8] = include_bytes!("../../../assets/fonts/fallback.ttf");
        Font::from_bytes(FALLBACK_FONT, FontSettings::default())
            .map(Arc::new)
            .map_err(|e| FontError::InvalidFont(e.to_string()))
    }

    /// Load a font from bytes.
    pub fn load_font(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
        data: Vec<u8>,
    ) -> FontResult<()> {
        let font = Font::from_bytes(data, FontSettings::default())
            .map_err(|e| FontError::InvalidFont(e.to_string()))?;

        let key = Self::font_key(family, weight, italic);
        self.fonts.insert(key, Arc::new(font));

        // Track font family variants for smart matching
        let family_lower = family.to_lowercase();
        self.font_families
            .entry(family_lower)
            .or_default()
            .push((weight, italic));

        Ok(())
    }

    /// Generate a cache key for a font.
    fn font_key(family: &str, weight: u16, italic: bool) -> String {
        format!("{}:{}:{}", family.to_lowercase(), weight, italic)
    }

    /// Find the closest matching font variant for a family.
    /// Returns the best match based on weight and italic preferences.
    fn find_closest_match(&self, family: &str, weight: u16, italic: bool) -> Option<Arc<Font>> {
        let family_lower = family.to_lowercase();
        let variants = self.font_families.get(&family_lower)?;

        if variants.is_empty() {
            return None;
        }

        // First, try to find variants with matching italic
        let matching_italic: Vec<_> = variants.iter().filter(|(_, i)| *i == italic).collect();

        let candidates = if matching_italic.is_empty() {
            // No italic match, use all variants (will fall back to non-italic)
            variants.iter().collect::<Vec<_>>()
        } else {
            matching_italic
        };

        // Find the closest weight
        let closest = candidates
            .iter()
            .min_by_key(|(w, _)| ((*w as i32) - (weight as i32)).abs())?;

        let key = Self::font_key(family, closest.0, closest.1);
        self.fonts.get(&key).cloned()
    }
}

impl Default for SystemFontProvider {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // Create with no default font - will panic on use
            Self {
                fonts: HashMap::new(),
                font_families: HashMap::new(),
                default: Arc::new(
                    Font::from_bytes(
                        include_bytes!("../../../assets/fonts/fallback.ttf").as_slice(),
                        FontSettings::default(),
                    )
                    .expect("Failed to load fallback font"),
                ),
                fallback_fonts: DEFAULT_FALLBACK_FONTS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            }
        })
    }
}

impl FontProvider for SystemFontProvider {
    fn get_font(&self, family: &str, weight: u16, italic: bool) -> Option<Arc<Font>> {
        // First try exact match
        let key = Self::font_key(family, weight, italic);
        if let Some(font) = self.fonts.get(&key) {
            return Some(font.clone());
        }

        // Try to find closest match within the same family
        self.find_closest_match(family, weight, italic)
    }

    fn default_font(&self) -> Arc<Font> {
        self.default.clone()
    }

    fn fallback_fonts(&self) -> &[String] {
        &self.fallback_fonts
    }
}

/// Embedded font provider for fonts bundled with documents.
pub struct EmbeddedFontProvider {
    fonts: HashMap<String, Arc<Font>>,
    fallback: Arc<dyn FontProvider>,
}

impl EmbeddedFontProvider {
    /// Create a new embedded font provider with a fallback.
    pub fn new(fallback: Arc<dyn FontProvider>) -> Self {
        Self {
            fonts: HashMap::new(),
            fallback,
        }
    }

    /// Add an embedded font.
    pub fn add_font(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
        data: Vec<u8>,
    ) -> FontResult<()> {
        let font = Font::from_bytes(data, FontSettings::default())
            .map_err(|e| FontError::InvalidFont(e.to_string()))?;

        let key = format!("{}:{}:{}", family.to_lowercase(), weight, italic);
        self.fonts.insert(key, Arc::new(font));
        Ok(())
    }
}

impl FontProvider for EmbeddedFontProvider {
    fn get_font(&self, family: &str, weight: u16, italic: bool) -> Option<Arc<Font>> {
        let key = format!("{}:{}:{}", family.to_lowercase(), weight, italic);
        self.fonts
            .get(&key)
            .cloned()
            .or_else(|| self.fallback.get_font(family, weight, italic))
    }

    fn default_font(&self) -> Arc<Font> {
        self.fallback.default_font()
    }

    fn fallback_fonts(&self) -> &[String] {
        self.fallback.fallback_fonts()
    }
}

/// Web font provider for WASM targets.
/// Uses browser APIs to access system fonts and load web fonts.
#[cfg(target_arch = "wasm32")]
pub struct WebFontProvider {
    /// Cached fonts loaded from the browser
    fonts: HashMap<String, Arc<Font>>,
    /// Fallback font chain
    fallback_fonts: Vec<String>,
    /// Default font (must be loaded)
    default: Option<Arc<Font>>,
}

#[cfg(target_arch = "wasm32")]
impl WebFontProvider {
    /// Create a new web font provider.
    ///
    /// Note: The default font must be set via `set_default_font` before use,
    /// typically by loading a font from a URL or embedded data.
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            fallback_fonts: vec![
                "Arial".to_string(),
                "Helvetica".to_string(),
                "sans-serif".to_string(),
            ],
            default: None,
        }
    }

    /// Set the default font from bytes.
    pub fn set_default_font(&mut self, data: Vec<u8>) -> FontResult<()> {
        let font = Font::from_bytes(data, FontSettings::default())
            .map_err(|e| FontError::InvalidFont(e.to_string()))?;
        self.default = Some(Arc::new(font));
        Ok(())
    }

    /// Add a font from bytes.
    pub fn add_font(
        &mut self,
        family: &str,
        weight: u16,
        italic: bool,
        data: Vec<u8>,
    ) -> FontResult<()> {
        let font = Font::from_bytes(data, FontSettings::default())
            .map_err(|e| FontError::InvalidFont(e.to_string()))?;

        let key = Self::font_key(family, weight, italic);
        self.fonts.insert(key, Arc::new(font));
        Ok(())
    }

    /// Generate a cache key for a font.
    fn font_key(family: &str, weight: u16, italic: bool) -> String {
        format!("{}:{}:{}", family.to_lowercase(), weight, italic)
    }

    /// Load a font from a URL.
    ///
    /// TODO: Implement using web_sys and js_sys to fetch font data.
    /// This would use the Fetch API to download the font file,
    /// then parse it with fontdue.
    pub async fn load_font_from_url(
        &mut self,
        _family: &str,
        _weight: u16,
        _italic: bool,
        _url: &str,
    ) -> FontResult<()> {
        Err(FontError::Internal(
            "Web font loading not yet implemented".to_string(),
        ))
    }

    /// Query available system fonts via the browser.
    ///
    /// TODO: Implement using the Local Font Access API (where available)
    /// or fall back to testing common font families via canvas measurement.
    pub async fn query_system_fonts(&self) -> Vec<String> {
        vec![]
    }
}

#[cfg(target_arch = "wasm32")]
impl Default for WebFontProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_arch = "wasm32")]
impl FontProvider for WebFontProvider {
    fn get_font(&self, family: &str, weight: u16, italic: bool) -> Option<Arc<Font>> {
        let key = Self::font_key(family, weight, italic);
        self.fonts.get(&key).cloned()
    }

    fn default_font(&self) -> Arc<Font> {
        self.default
            .clone()
            .expect("WebFontProvider: default font not set - call set_default_font first")
    }

    fn fallback_fonts(&self) -> &[String] {
        &self.fallback_fonts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_default_font_exists_without_any_system_fonts() {
        // The bundled font keeps exports working on a machine (or browser) with no fonts
        let font = SystemFontProvider::create_fallback_font().expect("bundled font loads");
        assert!(font.horizontal_line_metrics(12.0).is_some());
    }

    #[test]
    fn test_font_metrics() {
        let metrics = FontMetrics {
            ascent: 10.0,
            descent: 2.0,
            line_height: 14.0,
            x_height: 5.0,
            cap_height: 8.0,
            space_width: 3.0,
            underline_position: 1.0,
            underline_thickness: 0.5,
        };

        assert!((metrics.total_height() - 12.0).abs() < 0.01);
    }

    #[test]
    fn test_font_weight_from_numeric() {
        assert_eq!(FontWeight::from_numeric(100), FontWeight::Thin);
        assert_eq!(FontWeight::from_numeric(200), FontWeight::ExtraLight);
        assert_eq!(FontWeight::from_numeric(300), FontWeight::Light);
        assert_eq!(FontWeight::from_numeric(400), FontWeight::Normal);
        assert_eq!(FontWeight::from_numeric(500), FontWeight::Medium);
        assert_eq!(FontWeight::from_numeric(600), FontWeight::SemiBold);
        assert_eq!(FontWeight::from_numeric(700), FontWeight::Bold);
        assert_eq!(FontWeight::from_numeric(800), FontWeight::ExtraBold);
        assert_eq!(FontWeight::from_numeric(900), FontWeight::Black);
    }

    #[test]
    fn test_font_weight_boundaries() {
        assert_eq!(FontWeight::from_numeric(149), FontWeight::Thin);
        assert_eq!(FontWeight::from_numeric(150), FontWeight::ExtraLight);
        assert_eq!(FontWeight::from_numeric(449), FontWeight::Normal);
        assert_eq!(FontWeight::from_numeric(450), FontWeight::Medium);
    }

    #[test]
    fn test_font_weight_as_numeric() {
        assert_eq!(FontWeight::Thin.as_numeric(), 100);
        assert_eq!(FontWeight::Normal.as_numeric(), 400);
        assert_eq!(FontWeight::Bold.as_numeric(), 700);
        assert_eq!(FontWeight::Black.as_numeric(), 900);
    }

    #[test]
    fn test_kerning_common_pairs() {
        let size = 12.0;

        let kern_av = get_kerning_adjustment('A', 'V', size);
        assert!(kern_av < 0.0, "AV should have negative kerning");

        let kern_ta = get_kerning_adjustment('T', 'a', size);
        assert!(kern_ta < 0.0, "Ta should have negative kerning");

        let kern_yo = get_kerning_adjustment('Y', 'o', size);
        assert!(kern_yo < 0.0, "Yo should have negative kerning");
    }

    #[test]
    fn test_kerning_scales_with_size() {
        let kern_12 = get_kerning_adjustment('A', 'V', 12.0);
        let kern_24 = get_kerning_adjustment('A', 'V', 24.0);

        assert!((kern_24 - kern_12 * 2.0).abs() < 0.001);
    }

    #[test]
    fn test_kerning_unknown_pairs() {
        let kern = get_kerning_adjustment('z', 'q', 12.0);
        assert!((kern - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_kerning_punctuation() {
        let size = 12.0;

        let kern_period_quote = get_kerning_adjustment('.', '"', size);
        assert!(
            kern_period_quote < 0.0,
            "Period-quote should have negative kerning"
        );
    }

    #[test]
    fn test_default_fallback_fonts() {
        assert!(!DEFAULT_FALLBACK_FONTS.is_empty());
        assert!(DEFAULT_FALLBACK_FONTS.contains(&"Arial"));
        assert!(DEFAULT_FALLBACK_FONTS.contains(&"Helvetica"));
        assert!(DEFAULT_FALLBACK_FONTS.contains(&"sans-serif"));
    }

    #[test]
    fn test_system_provider_fallback_fonts() {
        let provider = SystemFontProvider::default();
        let fallbacks = provider.fallback_fonts();

        assert!(!fallbacks.is_empty());
        assert!(fallbacks.iter().any(|f| f == "Arial"));
    }

    #[test]
    fn test_custom_fallback_fonts() {
        let custom_fallbacks = vec!["CustomFont".to_string(), "FallbackFont".to_string()];

        let provider = SystemFontProvider::with_fallbacks(custom_fallbacks.clone())
            .expect("Failed to create provider");

        assert_eq!(provider.fallback_fonts(), custom_fallbacks.as_slice());
    }

    #[test]
    fn test_set_fallback_fonts() {
        let mut provider = SystemFontProvider::default();

        let new_fallbacks = vec!["NewFont".to_string()];
        provider.set_fallback_fonts(new_fallbacks.clone());

        assert_eq!(provider.fallback_fonts(), new_fallbacks.as_slice());
    }

    #[test]
    fn test_get_font_with_fallback_returns_default() {
        let provider = SystemFontProvider::default();

        let font = provider.get_font_with_fallback("NonExistentFont", 400, false);

        let default = provider.default_font();
        assert!(Arc::ptr_eq(&font, &default));
    }

    #[test]
    fn test_closest_weight_matching() {
        let mut provider = SystemFontProvider::default();

        let font_data = include_bytes!("../../../assets/fonts/fallback.ttf").to_vec();

        provider
            .load_font("TestFont", 400, false, font_data.clone())
            .expect("Failed to load font");
        provider
            .load_font("TestFont", 700, false, font_data.clone())
            .expect("Failed to load font");

        let font = provider.get_font("TestFont", 500, false);
        assert!(font.is_some(), "Should find closest weight match");

        let font = provider.get_font("TestFont", 600, false);
        assert!(font.is_some(), "Should find closest weight match");
    }

    #[test]
    fn test_italic_fallback() {
        let mut provider = SystemFontProvider::default();

        let font_data = include_bytes!("../../../assets/fonts/fallback.ttf").to_vec();

        provider
            .load_font("TestFont", 400, false, font_data)
            .expect("Failed to load font");

        let font = provider.get_font("TestFont", 400, true);
        assert!(
            font.is_some(),
            "Should fall back to non-italic when italic not available"
        );
    }

    #[test]
    fn test_measure_text_with_kerning() {
        let provider = SystemFontProvider::default();
        let font = provider.default_font();
        let size = 12.0;

        let with_kerning = provider.measure_text("AV", &font, size);
        let without_kerning = provider.measure_text_no_kerning("AV", &font, size);

        assert!(
            with_kerning <= without_kerning,
            "Text with kerning should be same or narrower"
        );
    }

    #[test]
    fn test_measure_empty_text() {
        let provider = SystemFontProvider::default();
        let font = provider.default_font();

        let width = provider.measure_text("", &font, 12.0);
        assert!((width - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_measure_single_char() {
        let provider = SystemFontProvider::default();
        let font = provider.default_font();

        let with_kerning = provider.measure_text("A", &font, 12.0);
        let without_kerning = provider.measure_text_no_kerning("A", &font, 12.0);

        assert!((with_kerning - without_kerning).abs() < 0.001);
    }

    #[test]
    fn test_embedded_provider_uses_fallback() {
        let system_provider = Arc::new(SystemFontProvider::default());
        let embedded = EmbeddedFontProvider::new(system_provider.clone());

        let font = embedded.get_font("Arial", 400, false);
        let _ = font;
    }

    #[test]
    fn test_embedded_provider_fallback_fonts() {
        let system_provider = Arc::new(SystemFontProvider::default());
        let embedded = EmbeddedFontProvider::new(system_provider.clone());

        assert_eq!(embedded.fallback_fonts(), system_provider.fallback_fonts());
    }

    #[test]
    fn test_char_metrics() {
        let provider = SystemFontProvider::default();
        let font = provider.default_font();

        let metrics = provider.measure_char('M', &font, 12.0);

        assert!(metrics.advance > 0.0);
        assert!(metrics.width >= 0.0);
        assert!(metrics.height >= 0.0);
    }
}
