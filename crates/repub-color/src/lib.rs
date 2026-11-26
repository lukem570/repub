//! # Repub Color
//!
//! Foundation-layer color vocabulary for Repub: RGBA and CMYK colors, color
//! spaces, document scheme-color references and their resolution, gradients,
//! and bitmap fill patterns expressed as data.
//!
//! This crate depends only on [`repub_geometry`] and has no serialization
//! layer.

#![warn(missing_docs)]
#![warn(clippy::all)]

mod color;
mod gradient;
mod pattern;
mod scheme;

pub use color::{CmykColor, Color, ColorSpace};
pub use gradient::{GradientStop, LinearGradient, RadialGradient};
pub use pattern::{Pattern, PatternType};
pub use scheme::{ColorScheme, SchemeColor, SchemeRef};
