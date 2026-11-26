//! # Repub Fonts
//!
//! Font loading, metrics, measurement and the installed-font registry for the
//! Repub desktop publishing system.
//!
//! This is a low-level layer that knows nothing about the document model. It
//! provides:
//!
//! - [`FontProvider`] — a trait abstracting font lookup and measurement, with
//!   the [`SystemFontProvider`] and [`EmbeddedFontProvider`] implementations.
//! - [`FontMetrics`] / [`CharMetrics`] — font and glyph metrics computed from
//!   a [`fontdue::Font`].
//! - [`get_kerning_adjustment`] — a built-in kerning table.
//! - [`registry`] — a scanner for the installed system fonts (ttf-parser).

#![warn(missing_docs)]

pub mod error;
pub mod provider;
pub mod registry;

pub use error::{FontError, FontResult};
pub use provider::{
    get_kerning_adjustment, CharMetrics, EmbeddedFontProvider, FontMetrics, FontProvider,
    FontWeight, LoadedFont, SystemFontProvider, DEFAULT_FALLBACK_FONTS,
};

#[cfg(target_arch = "wasm32")]
pub use provider::WebFontProvider;
