//! # Repub Model
//!
//! The pure document model for the Repub desktop publishing system: pages,
//! layers, objects, text, and styles. This is a serialization-free layer built
//! on the foundation [`repub_geometry`] and [`repub_color`] crates.
//!
//! The model provides the fundamental data structures for representing
//! Publisher-compatible documents including:
//!
//! - [`Document`] - The root container for a publication
//! - [`Page`] - Individual pages with layers and shapes
//! - [`shape::Shape`] - The single shape type: text frames, pictures, auto
//!   shapes, tables, text effects, lines and groups
//! - Styles for text, paragraphs, and shapes
//!
//! ## Example
//!
//! ```rust
//! use repub_model::{Document, PageSize, PageOrientation};
//! use repub_model::shape::Shape;
//! use repub_model::Rect;
//!
//! // Create a new document
//! let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
//!
//! // Add a shape to the first page
//! let shape = Shape::rectangle(Rect::new(100.0, 100.0, 200.0, 150.0));
//! doc.page_mut(0).unwrap().add_shape(shape);
//!
//! // Add a text frame
//! let text = Shape::text_frame_with_text(
//!     Rect::new(100.0, 300.0, 400.0, 100.0),
//!     "Hello, Repub!"
//! );
//! doc.page_mut(0).unwrap().add_shape(text);
//!
//! assert_eq!(doc.page(0).unwrap().shape_count(), 2);
//! ```

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod document;
pub mod geometry;
pub mod id;
pub mod layer;
pub mod path_edit;
pub mod shape;
pub mod styles;
pub mod text;
pub mod text_fit;

// Foundation-layer geometry, color, and unit types, re-exported for convenience
// so model consumers have a single import surface.
pub use repub_color::{
    CmykColor, Color, ColorScheme, ColorSpace, GradientStop, LinearGradient, Pattern, PatternType,
    RadialGradient, SchemeColor, SchemeRef,
};
pub use repub_geometry::{
    Length, Path as GeometryPath, PathSegment, Point, Rect, Size, Transform2D, Unit, Vector,
    POINTS_PER_CM, POINTS_PER_INCH, POINTS_PER_MM, POINTS_PER_PIXEL_96DPI,
};

// Re-export commonly used model types at the crate root
pub use document::{
    Document, DocumentMetadata, DocumentSettings, Guide, Margins, MasterPage, Page,
};
pub use id::{reset_id_counter, LayerId, MasterPageId, ObjectId, PageId, StyleId};
pub use layer::Layer;

/// Page size presets.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PageSize {
    /// US Letter (8.5 x 11 inches)
    #[default]
    Letter,
    /// US Legal (8.5 x 14 inches)
    Legal,
    /// US Tabloid (11 x 17 inches)
    Tabloid,
    /// A3 (297 x 420 mm)
    A3,
    /// A4 (210 x 297 mm)
    A4,
    /// A5 (148 x 210 mm)
    A5,
    /// A6 (105 x 148 mm)
    A6,
    /// B5 (176 x 250 mm)
    B5,
    /// Executive (7.25 x 10.5 inches)
    Executive,
    /// Business card (3.5 x 2 inches)
    BusinessCard,
    /// Index card 4x6 (4 x 6 inches)
    IndexCard4x6,
    /// Index card 5x7 (5 x 7 inches)
    IndexCard5x7,
    /// Custom size in points
    Custom {
        /// Width in points
        width: f64,
        /// Height in points
        height: f64,
    },
}

impl PageSize {
    /// Convert to size in points.
    pub fn to_points(&self) -> Size {
        match self {
            PageSize::Letter => Size::new(612.0, 792.0),
            PageSize::Legal => Size::new(612.0, 1008.0),
            PageSize::Tabloid => Size::new(792.0, 1224.0),
            PageSize::A3 => Size::new(841.89, 1190.55),
            PageSize::A4 => Size::new(595.28, 841.89),
            PageSize::A5 => Size::new(419.53, 595.28),
            PageSize::A6 => Size::new(297.64, 419.53),
            PageSize::B5 => Size::new(498.90, 708.66),
            PageSize::Executive => Size::new(522.0, 756.0),
            PageSize::BusinessCard => Size::new(252.0, 144.0), // 3.5 x 2 inches
            PageSize::IndexCard4x6 => Size::new(288.0, 432.0),
            PageSize::IndexCard5x7 => Size::new(360.0, 504.0),
            PageSize::Custom { width, height } => Size::new(*width, *height),
        }
    }

    /// Create a custom page size from inches.
    pub fn from_inches(width: f64, height: f64) -> Self {
        PageSize::Custom {
            width: width * 72.0,
            height: height * 72.0,
        }
    }

    /// Create a custom page size from millimeters.
    pub fn from_mm(width: f64, height: f64) -> Self {
        PageSize::Custom {
            width: width * 72.0 / 25.4,
            height: height * 72.0 / 25.4,
        }
    }

    /// Get the name of the page size.
    pub fn name(&self) -> &'static str {
        match self {
            PageSize::Letter => "Letter",
            PageSize::Legal => "Legal",
            PageSize::Tabloid => "Tabloid",
            PageSize::A3 => "A3",
            PageSize::A4 => "A4",
            PageSize::A5 => "A5",
            PageSize::A6 => "A6",
            PageSize::B5 => "B5",
            PageSize::Executive => "Executive",
            PageSize::BusinessCard => "Business Card",
            PageSize::IndexCard4x6 => "Index Card 4x6",
            PageSize::IndexCard5x7 => "Index Card 5x7",
            PageSize::Custom { .. } => "Custom",
        }
    }
}

/// Page orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageOrientation {
    /// Portrait (height > width)
    #[default]
    Portrait,
    /// Landscape (width > height)
    Landscape,
}

impl PageOrientation {
    /// Toggle the orientation.
    pub fn toggle(&self) -> Self {
        match self {
            PageOrientation::Portrait => PageOrientation::Landscape,
            PageOrientation::Landscape => PageOrientation::Portrait,
        }
    }
}

/// Repub model library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_size_to_points() {
        let letter = PageSize::Letter.to_points();
        assert_eq!(letter.width, 612.0);
        assert_eq!(letter.height, 792.0);
    }

    #[test]
    fn test_page_size_from_inches() {
        let size = PageSize::from_inches(8.5, 11.0);
        if let PageSize::Custom { width, height } = size {
            assert!((width - 612.0).abs() < 0.1);
            assert!((height - 792.0).abs() < 0.1);
        } else {
            panic!("Expected Custom size");
        }
    }

    #[test]
    fn test_orientation_toggle() {
        assert_eq!(
            PageOrientation::Portrait.toggle(),
            PageOrientation::Landscape
        );
        assert_eq!(
            PageOrientation::Landscape.toggle(),
            PageOrientation::Portrait
        );
    }
}
