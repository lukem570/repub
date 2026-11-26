//! Pattern fill types.


use crate::color::Color;

/// A bitmap pattern fill.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    /// Pattern type
    pub pattern_type: PatternType,
    /// Foreground color
    pub foreground: Color,
    /// Background color
    pub background: Color,
    /// Scale factor (1.0 = original size)
    pub scale: f64,
    /// Rotation angle in degrees
    pub rotation: f64,
}

impl Default for Pattern {
    fn default() -> Self {
        Self {
            pattern_type: PatternType::Solid,
            foreground: Color::BLACK,
            background: Color::WHITE,
            scale: 1.0,
            rotation: 0.0,
        }
    }
}

impl Pattern {
    /// Create a new pattern.
    pub fn new(pattern_type: PatternType, foreground: Color, background: Color) -> Self {
        Self {
            pattern_type,
            foreground,
            background,
            scale: 1.0,
            rotation: 0.0,
        }
    }

    /// Create a horizontal stripe pattern.
    pub fn horizontal_stripes(foreground: Color, background: Color) -> Self {
        Self::new(PatternType::HorizontalStripe, foreground, background)
    }

    /// Create a vertical stripe pattern.
    pub fn vertical_stripes(foreground: Color, background: Color) -> Self {
        Self::new(PatternType::VerticalStripe, foreground, background)
    }

    /// Create a diagonal stripe pattern.
    pub fn diagonal_stripes(foreground: Color, background: Color) -> Self {
        Self::new(PatternType::DiagonalStripeRight, foreground, background)
    }

    /// Create a crosshatch pattern.
    pub fn crosshatch(foreground: Color, background: Color) -> Self {
        Self::new(PatternType::Cross, foreground, background)
    }

    /// Create a dotted pattern.
    pub fn dots(foreground: Color, background: Color) -> Self {
        Self::new(PatternType::SmallGrid, foreground, background)
    }

    /// Set the scale.
    pub fn with_scale(mut self, scale: f64) -> Self {
        self.scale = scale;
        self
    }

    /// Set the rotation.
    pub fn with_rotation(mut self, degrees: f64) -> Self {
        self.rotation = degrees;
        self
    }
}

/// Built-in pattern types (matching Publisher's patterns).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PatternType {
    /// Solid fill (no pattern)
    #[default]
    Solid,

    // Percentage fills
    /// 5% fill
    Percent5,
    /// 10% fill
    Percent10,
    /// 20% fill
    Percent20,
    /// 25% fill
    Percent25,
    /// 30% fill
    Percent30,
    /// 40% fill
    Percent40,
    /// 50% fill
    Percent50,
    /// 60% fill
    Percent60,
    /// 70% fill
    Percent70,
    /// 75% fill
    Percent75,
    /// 80% fill
    Percent80,
    /// 90% fill
    Percent90,

    // Stripe patterns
    /// Light horizontal stripes
    LightHorizontal,
    /// Light vertical stripes
    LightVertical,
    /// Light diagonal (down-right)
    LightDownwardDiagonal,
    /// Light diagonal (up-right)
    LightUpwardDiagonal,
    /// Dark horizontal stripes
    DarkHorizontal,
    /// Dark vertical stripes
    DarkVertical,
    /// Dark diagonal (down-right)
    DarkDownwardDiagonal,
    /// Dark diagonal (up-right)
    DarkUpwardDiagonal,
    /// Wide diagonal (down-right)
    WideDownwardDiagonal,
    /// Wide diagonal (up-right)
    WideUpwardDiagonal,
    /// Horizontal stripe
    HorizontalStripe,
    /// Vertical stripe
    VerticalStripe,
    /// Diagonal stripe (right)
    DiagonalStripeRight,
    /// Diagonal stripe (left)
    DiagonalStripeLeft,

    // Cross patterns
    /// Light horizontal cross
    LightHorizontalCross,
    /// Light diagonal cross
    LightDiagonalCross,
    /// Dark horizontal cross
    DarkHorizontalCross,
    /// Dark diagonal cross
    DarkDiagonalCross,
    /// Cross
    Cross,
    /// Diagonal cross
    DiagonalCross,

    // Grid patterns
    /// Small grid
    SmallGrid,
    /// Large grid
    LargeGrid,
    /// Dotted grid
    DottedGrid,

    // Checker patterns
    /// Small checker
    SmallChecker,
    /// Large checker
    LargeChecker,

    // Other patterns
    /// Horizontal brick
    HorizontalBrick,
    /// Diagonal brick
    DiagonalBrick,
    /// Weave
    Weave,
    /// Plaid
    Plaid,
    /// Divot
    Divot,
    /// Dotted diamond
    DottedDiamond,
    /// Small confetti
    SmallConfetti,
    /// Large confetti
    LargeConfetti,
    /// Zig zag
    ZigZag,
    /// Wave
    Wave,
    /// Shingle
    Shingle,
    /// Trellis
    Trellis,
    /// Sphere
    Sphere,
    /// Outlined diamond
    OutlinedDiamond,
    /// Solid diamond
    SolidDiamond,
}

impl PatternType {
    /// Get the name of the pattern.
    pub fn name(&self) -> &'static str {
        match self {
            PatternType::Solid => "Solid",
            PatternType::Percent5 => "5%",
            PatternType::Percent10 => "10%",
            PatternType::Percent20 => "20%",
            PatternType::Percent25 => "25%",
            PatternType::Percent30 => "30%",
            PatternType::Percent40 => "40%",
            PatternType::Percent50 => "50%",
            PatternType::Percent60 => "60%",
            PatternType::Percent70 => "70%",
            PatternType::Percent75 => "75%",
            PatternType::Percent80 => "80%",
            PatternType::Percent90 => "90%",
            PatternType::LightHorizontal => "Light Horizontal",
            PatternType::LightVertical => "Light Vertical",
            PatternType::LightDownwardDiagonal => "Light Downward Diagonal",
            PatternType::LightUpwardDiagonal => "Light Upward Diagonal",
            PatternType::DarkHorizontal => "Dark Horizontal",
            PatternType::DarkVertical => "Dark Vertical",
            PatternType::DarkDownwardDiagonal => "Dark Downward Diagonal",
            PatternType::DarkUpwardDiagonal => "Dark Upward Diagonal",
            PatternType::WideDownwardDiagonal => "Wide Downward Diagonal",
            PatternType::WideUpwardDiagonal => "Wide Upward Diagonal",
            PatternType::HorizontalStripe => "Horizontal Stripe",
            PatternType::VerticalStripe => "Vertical Stripe",
            PatternType::DiagonalStripeRight => "Diagonal Stripe Right",
            PatternType::DiagonalStripeLeft => "Diagonal Stripe Left",
            PatternType::LightHorizontalCross => "Light Horizontal Cross",
            PatternType::LightDiagonalCross => "Light Diagonal Cross",
            PatternType::DarkHorizontalCross => "Dark Horizontal Cross",
            PatternType::DarkDiagonalCross => "Dark Diagonal Cross",
            PatternType::Cross => "Cross",
            PatternType::DiagonalCross => "Diagonal Cross",
            PatternType::SmallGrid => "Small Grid",
            PatternType::LargeGrid => "Large Grid",
            PatternType::DottedGrid => "Dotted Grid",
            PatternType::SmallChecker => "Small Checker",
            PatternType::LargeChecker => "Large Checker",
            PatternType::HorizontalBrick => "Horizontal Brick",
            PatternType::DiagonalBrick => "Diagonal Brick",
            PatternType::Weave => "Weave",
            PatternType::Plaid => "Plaid",
            PatternType::Divot => "Divot",
            PatternType::DottedDiamond => "Dotted Diamond",
            PatternType::SmallConfetti => "Small Confetti",
            PatternType::LargeConfetti => "Large Confetti",
            PatternType::ZigZag => "Zig Zag",
            PatternType::Wave => "Wave",
            PatternType::Shingle => "Shingle",
            PatternType::Trellis => "Trellis",
            PatternType::Sphere => "Sphere",
            PatternType::OutlinedDiamond => "Outlined Diamond",
            PatternType::SolidDiamond => "Solid Diamond",
        }
    }

    /// The pattern's 8x8 tile, one byte per row, most significant bit on the
    /// left; set bits are foreground. Close to Publisher's patterns, which tile
    /// at one tile pixel per screen pixel at 100% zoom.
    pub fn bits(&self) -> [u8; 8] {
        use PatternType::*;
        // Ordered (Bayer) dither for the percentage fills
        const BAYER: [[u8; 8]; 8] = [
            [0, 32, 8, 40, 2, 34, 10, 42],
            [48, 16, 56, 24, 50, 18, 58, 26],
            [12, 44, 4, 36, 14, 46, 6, 38],
            [60, 28, 52, 20, 62, 30, 54, 22],
            [3, 35, 11, 43, 1, 33, 9, 41],
            [51, 19, 59, 27, 49, 17, 57, 25],
            [15, 47, 7, 39, 13, 45, 5, 37],
            [63, 31, 55, 23, 61, 29, 53, 21],
        ];
        let from = |on: &dyn Fn(usize, usize) -> bool| {
            let mut rows = [0u8; 8];
            for (y, row) in rows.iter_mut().enumerate() {
                for x in 0..8 {
                    if on(x, y) {
                        *row |= 0x80 >> x;
                    }
                }
            }
            rows
        };
        let percent = |p: usize| from(&|x, y| (BAYER[y][x] as usize) < (p * 64 + 50) / 100);
        let down = |x: usize, y: usize| (x + 8 - y) % 8; // 0 on the \ diagonal through the origin
        let up = |x: usize, y: usize| (x + y) % 8; // constant along / diagonals
        match self {
            Solid => [0xFF; 8],
            Percent5 => percent(5),
            Percent10 => percent(10),
            Percent20 => percent(20),
            Percent25 => percent(25),
            Percent30 => percent(30),
            Percent40 => percent(40),
            Percent50 => percent(50),
            Percent60 => percent(60),
            Percent70 => percent(70),
            Percent75 => percent(75),
            Percent80 => percent(80),
            Percent90 => percent(90),
            LightHorizontal => from(&|_, y| y % 4 == 0),
            LightVertical => from(&|x, _| x % 4 == 0),
            LightDownwardDiagonal => from(&|x, y| down(x, y) % 4 == 0),
            LightUpwardDiagonal => from(&|x, y| up(x, y) % 4 == 3),
            DarkHorizontal => from(&|_, y| y % 4 < 2),
            DarkVertical => from(&|x, _| x % 4 < 2),
            DarkDownwardDiagonal => from(&|x, y| down(x, y) % 4 < 2),
            DarkUpwardDiagonal => from(&|x, y| up(x, y) % 4 >= 2),
            WideDownwardDiagonal => from(&|x, y| down(x, y) < 3),
            WideUpwardDiagonal => from(&|x, y| up(x, y) >= 5),
            HorizontalStripe => from(&|_, y| y < 4),
            VerticalStripe => from(&|x, _| x < 4),
            DiagonalStripeRight => from(&|x, y| up(x, y) < 4),
            DiagonalStripeLeft => from(&|x, y| down(x, y) < 4),
            LightHorizontalCross | SmallGrid => from(&|x, y| x % 4 == 0 || y % 4 == 0),
            LightDiagonalCross => from(&|x, y| down(x, y) % 4 == 0 || up(x, y) % 4 == 3),
            DarkHorizontalCross => from(&|x, y| x % 4 < 2 || y % 4 < 2),
            DarkDiagonalCross => from(&|x, y| down(x, y) % 4 < 2 || up(x, y) % 4 >= 2),
            Cross | LargeGrid => from(&|x, y| x == 0 || y == 0),
            DiagonalCross => from(&|x, y| down(x, y) == 0 || up(x, y) == 7),
            DottedGrid => from(&|x, y| (y == 0 && x % 2 == 0) || (x == 0 && y % 2 == 0)),
            SmallChecker => from(&|x, y| (x / 2 + y / 2) % 2 == 0),
            LargeChecker => from(&|x, y| (x / 4 + y / 4) % 2 == 0),
            HorizontalBrick => from(&|x, y| y % 4 == 0 || (x == 0 && y < 4) || (x == 4 && y > 4)),
            DiagonalBrick => from(&|x, y| up(x, y) == 7 || (down(x, y) == 0 && y >= 4)),
            Weave => [0x88, 0x54, 0x22, 0x45, 0x88, 0x14, 0x22, 0x51],
            Plaid => [0xAA, 0x55, 0xAA, 0x55, 0xF0, 0xF0, 0xF0, 0xF0],
            Divot => [0x00, 0x08, 0x04, 0x08, 0x00, 0x80, 0x40, 0x80],
            DottedDiamond => [0x80, 0x00, 0x22, 0x00, 0x08, 0x00, 0x22, 0x00],
            SmallConfetti => [0x80, 0x10, 0x02, 0x20, 0x01, 0x08, 0x40, 0x04],
            LargeConfetti => [0xB1, 0x30, 0x03, 0x1B, 0xD8, 0xC0, 0x0C, 0x8D],
            ZigZag => [0x81, 0x42, 0x24, 0x18, 0x81, 0x42, 0x24, 0x18],
            Wave => [0x00, 0x18, 0xA4, 0x03, 0x00, 0x18, 0xA4, 0x03],
            Shingle => [0x03, 0x84, 0x48, 0x30, 0x0C, 0x02, 0x01, 0x01],
            Trellis => [0xFF, 0x66, 0xFF, 0x99, 0xFF, 0x66, 0xFF, 0x99],
            Sphere => [0x77, 0x89, 0x8F, 0x8F, 0x77, 0x98, 0xF8, 0xF8],
            OutlinedDiamond => [0x41, 0x22, 0x14, 0x08, 0x14, 0x22, 0x41, 0x80],
            SolidDiamond => [0x08, 0x1C, 0x3E, 0x7F, 0x3E, 0x1C, 0x08, 0x00],
        }
    }

    /// Get all pattern types.
    pub fn all() -> &'static [PatternType] {
        &[
            PatternType::Solid,
            PatternType::Percent5,
            PatternType::Percent10,
            PatternType::Percent20,
            PatternType::Percent25,
            PatternType::Percent30,
            PatternType::Percent40,
            PatternType::Percent50,
            PatternType::Percent60,
            PatternType::Percent70,
            PatternType::Percent75,
            PatternType::Percent80,
            PatternType::Percent90,
            PatternType::LightHorizontal,
            PatternType::LightVertical,
            PatternType::LightDownwardDiagonal,
            PatternType::LightUpwardDiagonal,
            PatternType::DarkHorizontal,
            PatternType::DarkVertical,
            PatternType::DarkDownwardDiagonal,
            PatternType::DarkUpwardDiagonal,
            PatternType::WideDownwardDiagonal,
            PatternType::WideUpwardDiagonal,
            PatternType::HorizontalStripe,
            PatternType::VerticalStripe,
            PatternType::DiagonalStripeRight,
            PatternType::DiagonalStripeLeft,
            PatternType::LightHorizontalCross,
            PatternType::LightDiagonalCross,
            PatternType::DarkHorizontalCross,
            PatternType::DarkDiagonalCross,
            PatternType::Cross,
            PatternType::DiagonalCross,
            PatternType::SmallGrid,
            PatternType::LargeGrid,
            PatternType::DottedGrid,
            PatternType::SmallChecker,
            PatternType::LargeChecker,
            PatternType::HorizontalBrick,
            PatternType::DiagonalBrick,
            PatternType::Weave,
            PatternType::Plaid,
            PatternType::Divot,
            PatternType::DottedDiamond,
            PatternType::SmallConfetti,
            PatternType::LargeConfetti,
            PatternType::ZigZag,
            PatternType::Wave,
            PatternType::Shingle,
            PatternType::Trellis,
            PatternType::Sphere,
            PatternType::OutlinedDiamond,
            PatternType::SolidDiamond,
        ]
    }
}

/// An image-based pattern (texture).
#[derive(Debug, Clone, PartialEq)]
pub struct TexturePattern {
    /// Image source (path or embedded)
    pub source: String,
    /// Tile mode
    pub tile: TileMode,
    /// Scale X
    pub scale_x: f64,
    /// Scale Y
    pub scale_y: f64,
    /// Offset X (0-1)
    pub offset_x: f64,
    /// Offset Y (0-1)
    pub offset_y: f64,
    /// Rotation in degrees
    pub rotation: f64,
    /// Transparency (0 = opaque, 1 = fully transparent)
    pub transparency: f64,
}

impl Default for TexturePattern {
    fn default() -> Self {
        Self {
            source: String::new(),
            tile: TileMode::Tile,
            scale_x: 1.0,
            scale_y: 1.0,
            offset_x: 0.0,
            offset_y: 0.0,
            rotation: 0.0,
            transparency: 0.0,
        }
    }
}

impl TexturePattern {
    /// Create a texture pattern from an image.
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            ..Default::default()
        }
    }

    /// Set uniform scale.
    pub fn with_scale(mut self, scale: f64) -> Self {
        self.scale_x = scale;
        self.scale_y = scale;
        self
    }

    /// Set non-uniform scale.
    pub fn with_scale_xy(mut self, scale_x: f64, scale_y: f64) -> Self {
        self.scale_x = scale_x;
        self.scale_y = scale_y;
        self
    }

    /// Set the tile mode.
    pub fn with_tile_mode(mut self, tile: TileMode) -> Self {
        self.tile = tile;
        self
    }
}

/// Tile mode for texture patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TileMode {
    /// Tile the image
    #[default]
    Tile,
    /// Tile horizontally, stretch vertically
    TileHorizontal,
    /// Tile vertically, stretch horizontally
    TileVertical,
    /// Stretch to fill
    Stretch,
    /// Center without tiling
    Center,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_creation() {
        let pattern = Pattern::horizontal_stripes(Color::BLACK, Color::WHITE);
        assert!(matches!(
            pattern.pattern_type,
            PatternType::HorizontalStripe
        ));
        assert_eq!(pattern.foreground, Color::BLACK);
        assert_eq!(pattern.background, Color::WHITE);
    }

    #[test]
    fn test_pattern_with_scale() {
        let pattern = Pattern::dots(Color::RED, Color::WHITE).with_scale(2.0);
        assert!((pattern.scale - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_all_patterns() {
        let all = PatternType::all();
        assert!(all.len() > 40);
        assert!(all.contains(&PatternType::Cross));
        assert!(all.contains(&PatternType::Weave));
    }

    #[test]
    fn test_texture_pattern() {
        let texture = TexturePattern::new("texture.png")
            .with_scale(0.5)
            .with_tile_mode(TileMode::TileHorizontal);

        assert_eq!(texture.source, "texture.png");
        assert!((texture.scale_x - 0.5).abs() < 0.01);
        assert!(matches!(texture.tile, TileMode::TileHorizontal));
    }
}

#[cfg(test)]
mod bits_tests {
    use super::PatternType;

    fn coverage(p: PatternType) -> u32 {
        p.bits().iter().map(|r| r.count_ones()).sum()
    }

    #[test]
    fn percent_patterns_cover_their_share() {
        assert_eq!(coverage(PatternType::Percent50), 32);
        assert_eq!(coverage(PatternType::Percent25), 16);
        assert!(coverage(PatternType::Percent5) >= 3 && coverage(PatternType::Percent5) <= 4);
        assert_eq!(coverage(PatternType::Solid), 64);
    }

    #[test]
    fn stripes_run_the_right_way() {
        assert_eq!(PatternType::LightHorizontal.bits(), [0xFF, 0, 0, 0, 0xFF, 0, 0, 0]);
        assert_eq!(PatternType::LightVertical.bits(), [0x88; 8]);
        assert_eq!(PatternType::LightDownwardDiagonal.bits()[0..2], [0x88, 0x44]);
        assert_eq!(PatternType::LightUpwardDiagonal.bits()[0..2], [0x11, 0x22]);
        for p in PatternType::all() {
            assert!(coverage(*p) > 0, "{p:?} draws something");
        }
    }
}
