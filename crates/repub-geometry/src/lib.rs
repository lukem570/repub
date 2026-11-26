//! # Repub Geometry
//!
//! Foundation-layer 2D geometry primitives for Repub: points, sizes,
//! rectangles, affine transforms, vector paths with Bézier segments, and
//! measurement-unit conversions.
//!
//! This crate has no internal dependencies and no serialization layer.
//! All measurements are expressed in points (1/72 inch) unless a [`Unit`]
//! says otherwise.

#![warn(missing_docs)]
#![warn(clippy::all)]

mod path;
mod primitives;
mod transform;
mod units;

pub use path::{Path, PathSegment};
pub use primitives::{Point, Rect, Size, Vector};
pub use transform::Transform2D;
pub use units::{
    Length, Unit, POINTS_PER_CM, POINTS_PER_INCH, POINTS_PER_MM, POINTS_PER_PIXEL_96DPI,
};
