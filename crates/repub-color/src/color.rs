//! RGBA and CMYK color types and color spaces.

/// An RGBA color with components in the range 0.0–1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    /// Red component (0.0–1.0).
    pub r: f64,
    /// Green component (0.0–1.0).
    pub g: f64,
    /// Blue component (0.0–1.0).
    pub b: f64,
    /// Alpha component (0.0–1.0, where 1.0 is fully opaque).
    pub a: f64,
}

impl Default for Color {
    fn default() -> Self {
        Self::BLACK
    }
}

impl Color {
    /// Create a new color from RGBA components (0.0–1.0).
    pub const fn new(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self { r, g, b, a }
    }

    /// Create an opaque color from RGB components (0.0–1.0).
    pub const fn rgb(r: f64, g: f64, b: f64) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create a color from 8-bit RGBA components (0–255).
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f64 / 255.0,
            g: g as f64 / 255.0,
            b: b as f64 / 255.0,
            a: a as f64 / 255.0,
        }
    }

    /// Create an opaque color from 8-bit RGB components (0–255).
    pub fn from_rgb8(r: u8, g: u8, b: u8) -> Self {
        Self::from_rgba8(r, g, b, 255)
    }

    /// Parse a color from a hex string (e.g. `"#FF0000"`, `"FF0000"`, or
    /// `"#FF0000FF"`). Returns `None` on malformed input.
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        match hex.len() {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                Some(Self::from_rgb8(r, g, b))
            }
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                Some(Self::from_rgba8(r, g, b, a))
            }
            _ => None,
        }
    }

    /// Convert to 8-bit RGBA components.
    pub fn to_rgba8(&self) -> (u8, u8, u8, u8) {
        (
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.a.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }

    /// Convert to a hex string (e.g. `"#FF0000"`).
    pub fn to_hex(&self) -> String {
        let (r, g, b, _) = self.to_rgba8();
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    }

    /// Convert to a hex string with alpha (e.g. `"#FF0000FF"`).
    pub fn to_hex_with_alpha(&self) -> String {
        let (r, g, b, a) = self.to_rgba8();
        format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
    }

    /// Return a copy with a modified alpha.
    pub fn with_alpha(self, alpha: f64) -> Self {
        Self { a: alpha, ..self }
    }

    /// Return a copy scaled toward black by `factor` (0.0 = black, 1.0 = unchanged).
    pub fn darkened(self, factor: f64) -> Self {
        let f = factor.clamp(0.0, 1.0);
        Self {
            r: self.r * f,
            g: self.g * f,
            b: self.b * f,
            a: self.a,
        }
    }

    /// Return a copy blended toward white by `t` (0.0 = unchanged, 1.0 = white).
    pub fn lightened(self, t: f64) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: self.r + (1.0 - self.r) * t,
            g: self.g + (1.0 - self.g) * t,
            b: self.b + (1.0 - self.b) * t,
            a: self.a,
        }
    }

    /// Linearly interpolate between two colors (`t` clamped to [0, 1]).
    pub fn lerp(self, other: Self, t: f64) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    // Common colors.
    /// Transparent (alpha = 0).
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    /// Black.
    pub const BLACK: Self = Self::rgb(0.0, 0.0, 0.0);
    /// White.
    pub const WHITE: Self = Self::rgb(1.0, 1.0, 1.0);
    /// Red.
    pub const RED: Self = Self::rgb(1.0, 0.0, 0.0);
    /// Green.
    pub const GREEN: Self = Self::rgb(0.0, 1.0, 0.0);
    /// Blue.
    pub const BLUE: Self = Self::rgb(0.0, 0.0, 1.0);
    /// Yellow.
    pub const YELLOW: Self = Self::rgb(1.0, 1.0, 0.0);
    /// Cyan.
    pub const CYAN: Self = Self::rgb(0.0, 1.0, 1.0);
    /// Magenta.
    pub const MAGENTA: Self = Self::rgb(1.0, 0.0, 1.0);
    /// Gray (50%).
    pub const GRAY: Self = Self::rgb(0.5, 0.5, 0.5);
    /// Light gray (75%).
    pub const LIGHT_GRAY: Self = Self::rgb(0.75, 0.75, 0.75);
    /// Dark gray (25%).
    pub const DARK_GRAY: Self = Self::rgb(0.25, 0.25, 0.25);
    /// Light blue.
    pub const LIGHT_BLUE: Self = Self::rgb(0.68, 0.85, 0.9);
    /// Dark blue.
    pub const DARK_BLUE: Self = Self::rgb(0.0, 0.0, 0.55);
    /// Orange.
    pub const ORANGE: Self = Self::rgb(1.0, 0.65, 0.0);
    /// Purple.
    pub const PURPLE: Self = Self::rgb(0.5, 0.0, 0.5);
    /// Pink.
    pub const PINK: Self = Self::rgb(1.0, 0.75, 0.8);
    /// Brown.
    pub const BROWN: Self = Self::rgb(0.65, 0.16, 0.16);
}

/// A CMYK color for print workflows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CmykColor {
    /// Cyan component (0.0–1.0).
    pub c: f64,
    /// Magenta component (0.0–1.0).
    pub m: f64,
    /// Yellow component (0.0–1.0).
    pub y: f64,
    /// Key/black component (0.0–1.0).
    pub k: f64,
}

impl CmykColor {
    /// Create a new CMYK color.
    pub const fn new(c: f64, m: f64, y: f64, k: f64) -> Self {
        Self { c, m, y, k }
    }

    /// Convert to an RGB color (approximate, naive conversion).
    pub fn to_rgb(&self) -> Color {
        Color::rgb(
            (1.0 - self.c) * (1.0 - self.k),
            (1.0 - self.m) * (1.0 - self.k),
            (1.0 - self.y) * (1.0 - self.k),
        )
    }

    /// Convert from an RGB color (approximate, naive conversion).
    pub fn from_rgb(color: Color) -> Self {
        let k = 1.0 - color.r.max(color.g).max(color.b);
        if k >= 1.0 {
            return Self::new(0.0, 0.0, 0.0, 1.0);
        }
        Self::new(
            (1.0 - color.r - k) / (1.0 - k),
            (1.0 - color.g - k) / (1.0 - k),
            (1.0 - color.b - k) / (1.0 - k),
            k,
        )
    }
}

/// A color space specification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ColorSpace {
    /// Standard RGB (sRGB).
    #[default]
    Srgb,
    /// CMYK for print.
    Cmyk,
    /// Grayscale.
    Grayscale,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_from_hex() {
        let red = Color::from_hex("#FF0000").unwrap();
        assert!((red.r - 1.0).abs() < 0.01);
        assert!(red.g.abs() < 0.01);
        assert!(red.b.abs() < 0.01);
        assert!(Color::from_hex("nope").is_none());
    }

    #[test]
    fn color_to_hex() {
        assert_eq!(Color::rgb(1.0, 0.0, 0.5).to_hex(), "#FF0080");
    }

    #[test]
    fn color_lerp() {
        let gray = Color::BLACK.lerp(Color::WHITE, 0.5);
        assert!((gray.r - 0.5).abs() < 0.01);
    }

    #[test]
    fn darken_and_lighten() {
        let c = Color::rgb(0.5, 0.5, 0.5);
        assert!((c.darkened(0.6).r - 0.3).abs() < 1e-9);
        assert!((c.lightened(0.5).r - 0.75).abs() < 1e-9);
    }

    #[test]
    fn cmyk_roundtrip() {
        let original = Color::rgb(0.8, 0.2, 0.5);
        let back = CmykColor::from_rgb(original).to_rgb();
        assert!((original.r - back.r).abs() < 0.01);
        assert!((original.g - back.g).abs() < 0.01);
        assert!((original.b - back.b).abs() < 0.01);
    }
}
