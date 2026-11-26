//! Document color schemes and scheme-color references.
//!
//! A publication defines a [`ColorScheme`]: a small palette of named roles
//! (background, text, accents, hyperlinks). Objects can reference a role via a
//! [`SchemeRef`] rather than storing an absolute color, optionally applying a
//! tint (toward white) or shade (toward black). Resolving a reference against
//! a scheme yields a concrete [`Color`].

use crate::color::Color;

/// A named role within a [`ColorScheme`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemeColor {
    /// Main page/background color.
    Background,
    /// Main text color.
    Text,
    /// Accent color 1.
    Accent1,
    /// Accent color 2.
    Accent2,
    /// Accent color 3.
    Accent3,
    /// Accent color 4.
    Accent4,
    /// Accent color 5.
    Accent5,
    /// Unfollowed hyperlink color.
    Hyperlink,
    /// Followed hyperlink color.
    FollowedHyperlink,
}

impl SchemeColor {
    /// All roles in palette order.
    pub const ALL: [SchemeColor; 9] = [
        SchemeColor::Background,
        SchemeColor::Text,
        SchemeColor::Accent1,
        SchemeColor::Accent2,
        SchemeColor::Accent3,
        SchemeColor::Accent4,
        SchemeColor::Accent5,
        SchemeColor::Hyperlink,
        SchemeColor::FollowedHyperlink,
    ];

    /// The palette index for this role (0-based, matching [`SchemeColor::ALL`]).
    pub fn index(self) -> usize {
        SchemeColor::ALL.iter().position(|&r| r == self).unwrap()
    }

    /// The role for a 0-based palette index, or `None` if out of range.
    pub fn from_index(index: usize) -> Option<SchemeColor> {
        SchemeColor::ALL.get(index).copied()
    }
}

/// A reference to a scheme role with an optional tint or shade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SchemeRef {
    /// Which role to use.
    pub role: SchemeColor,
    /// Lightening toward white, 0.0 (none) to 1.0 (white).
    pub tint: f64,
    /// Darkening toward black, 0.0 (none) to 1.0 (black).
    pub shade: f64,
}

impl SchemeRef {
    /// A plain reference to a role with no tint or shade.
    pub fn new(role: SchemeColor) -> Self {
        Self {
            role,
            tint: 0.0,
            shade: 0.0,
        }
    }

    /// Set the tint (toward white).
    pub fn with_tint(mut self, tint: f64) -> Self {
        self.tint = tint.clamp(0.0, 1.0);
        self
    }

    /// Set the shade (toward black).
    pub fn with_shade(mut self, shade: f64) -> Self {
        self.shade = shade.clamp(0.0, 1.0);
        self
    }

    /// Resolve against `scheme` into a concrete color, applying tint then shade.
    pub fn resolve(&self, scheme: &ColorScheme) -> Color {
        let base = scheme.color(self.role);
        let tinted = if self.tint > 0.0 {
            base.lightened(self.tint)
        } else {
            base
        };
        if self.shade > 0.0 {
            tinted.darkened(1.0 - self.shade)
        } else {
            tinted
        }
    }
}

/// A publication's palette of scheme colors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColorScheme {
    /// Colors indexed by [`SchemeColor::index`].
    pub colors: [Color; 9],
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self::OFFICE
    }
}

impl ColorScheme {
    /// Resolve a role to its base color (before any tint/shade).
    pub fn color(&self, role: SchemeColor) -> Color {
        self.colors[role.index()]
    }

    /// Replace the color for a role.
    pub fn set(&mut self, role: SchemeColor, color: Color) {
        self.colors[role.index()] = color;
    }

    /// The default Office-style scheme.
    pub const OFFICE: ColorScheme = ColorScheme {
        colors: [
            Color::WHITE,                 // Background
            Color::BLACK,                 // Text
            Color::rgb(0.27, 0.45, 0.77), // Accent1 (blue)
            Color::rgb(0.91, 0.43, 0.32), // Accent2 (orange-red)
            Color::rgb(0.64, 0.64, 0.64), // Accent3 (gray)
            Color::rgb(1.0, 0.75, 0.0),   // Accent4 (gold)
            Color::rgb(0.44, 0.68, 0.28), // Accent5 (green)
            Color::rgb(0.0, 0.0, 1.0),    // Hyperlink
            Color::rgb(0.5, 0.0, 0.5),    // FollowedHyperlink
        ],
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn index_roundtrip() {
        for role in SchemeColor::ALL {
            assert_eq!(SchemeColor::from_index(role.index()), Some(role));
        }
        assert_eq!(SchemeColor::from_index(99), None);
    }

    #[test]
    fn resolve_plain_reference() {
        let scheme = ColorScheme::OFFICE;
        let r = SchemeRef::new(SchemeColor::Text);
        assert_eq!(r.resolve(&scheme), Color::BLACK);
    }

    #[test]
    fn resolve_with_tint_lightens() {
        let scheme = ColorScheme::OFFICE;
        let r = SchemeRef::new(SchemeColor::Text).with_tint(0.5);
        let c = r.resolve(&scheme);
        assert!(c.r > 0.0); // black tinted toward white
    }

    #[test]
    fn resolve_with_shade_darkens() {
        let scheme = ColorScheme::OFFICE;
        let r = SchemeRef::new(SchemeColor::Background).with_shade(0.5);
        let c = r.resolve(&scheme);
        assert!(c.r < 1.0); // white shaded toward black
    }
}
