//! Measurement units and length conversions.
//!
//! The internal unit throughout Repub is the point (1/72 inch).

/// Points per inch (by definition).
pub const POINTS_PER_INCH: f64 = 72.0;
/// Points per millimeter.
pub const POINTS_PER_MM: f64 = POINTS_PER_INCH / 25.4;
/// Points per centimeter.
pub const POINTS_PER_CM: f64 = POINTS_PER_INCH / 2.54;
/// Points per pixel at 96 DPI (common screen resolution).
pub const POINTS_PER_PIXEL_96DPI: f64 = POINTS_PER_INCH / 96.0;

/// A measurement unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Unit {
    /// Points (1/72 inch) — the internal unit.
    #[default]
    Points,
    /// Inches.
    Inches,
    /// Millimeters.
    Millimeters,
    /// Centimeters.
    Centimeters,
    /// Pixels (at 96 DPI).
    Pixels,
    /// Picas (1/6 inch = 12 points).
    Picas,
}

impl Unit {
    /// The abbreviation for this unit.
    pub fn abbreviation(&self) -> &'static str {
        match self {
            Unit::Points => "pt",
            Unit::Inches => "in",
            Unit::Millimeters => "mm",
            Unit::Centimeters => "cm",
            Unit::Pixels => "px",
            Unit::Picas => "pc",
        }
    }

    /// The full name for this unit.
    pub fn name(&self) -> &'static str {
        match self {
            Unit::Points => "points",
            Unit::Inches => "inches",
            Unit::Millimeters => "millimeters",
            Unit::Centimeters => "centimeters",
            Unit::Pixels => "pixels",
            Unit::Picas => "picas",
        }
    }

    /// Convert a value from this unit to points.
    pub fn to_points(&self, value: f64) -> f64 {
        match self {
            Unit::Points => value,
            Unit::Inches => value * POINTS_PER_INCH,
            Unit::Millimeters => value * POINTS_PER_MM,
            Unit::Centimeters => value * POINTS_PER_CM,
            Unit::Pixels => value * POINTS_PER_PIXEL_96DPI,
            Unit::Picas => value * 12.0,
        }
    }

    /// Convert a value from points to this unit.
    pub fn from_points(&self, points: f64) -> f64 {
        match self {
            Unit::Points => points,
            Unit::Inches => points / POINTS_PER_INCH,
            Unit::Millimeters => points / POINTS_PER_MM,
            Unit::Centimeters => points / POINTS_PER_CM,
            Unit::Pixels => points / POINTS_PER_PIXEL_96DPI,
            Unit::Picas => points / 12.0,
        }
    }
}

/// A length value with an associated unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Length {
    /// The numeric value.
    pub value: f64,
    /// The unit of measurement.
    pub unit: Unit,
}

impl Length {
    /// Zero length.
    pub const ZERO: Self = Self::new(0.0, Unit::Points);

    /// Create a new length.
    pub const fn new(value: f64, unit: Unit) -> Self {
        Self { value, unit }
    }

    /// Create a length in points.
    pub const fn points(value: f64) -> Self {
        Self::new(value, Unit::Points)
    }

    /// Create a length in inches.
    pub const fn inches(value: f64) -> Self {
        Self::new(value, Unit::Inches)
    }

    /// Create a length in millimeters.
    pub const fn mm(value: f64) -> Self {
        Self::new(value, Unit::Millimeters)
    }

    /// Create a length in centimeters.
    pub const fn cm(value: f64) -> Self {
        Self::new(value, Unit::Centimeters)
    }

    /// Create a length in pixels.
    pub const fn pixels(value: f64) -> Self {
        Self::new(value, Unit::Pixels)
    }

    /// Convert to points (the internal unit).
    pub fn to_points(&self) -> f64 {
        self.unit.to_points(self.value)
    }

    /// Convert to a different unit.
    pub fn to_unit(&self, target_unit: Unit) -> Self {
        let points = self.to_points();
        Self::new(target_unit.from_points(points), target_unit)
    }
}

impl Default for Length {
    fn default() -> Self {
        Self::ZERO
    }
}

impl core::fmt::Display for Length {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:.2}{}", self.value, self.unit.abbreviation())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inch_to_points() {
        assert!((Length::inches(1.0).to_points() - 72.0).abs() < 0.001);
    }

    #[test]
    fn mm_to_points() {
        assert!((Length::mm(25.4).to_points() - 72.0).abs() < 0.001);
    }

    #[test]
    fn unit_conversion() {
        let mm = Length::inches(1.0).to_unit(Unit::Millimeters);
        assert!((mm.value - 25.4).abs() < 0.001);
    }

    #[test]
    fn picas() {
        assert!((Length::new(6.0, Unit::Picas).to_points() - 72.0).abs() < 0.001);
    }

    #[test]
    fn display() {
        assert_eq!(format!("{}", Length::inches(2.5)), "2.50in");
    }
}
