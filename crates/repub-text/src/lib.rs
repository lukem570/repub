//! # Repub Text
//!
//! Shaping-agnostic text preparation for the Repub layout engine: Unicode line
//! breaking and segmentation, and flattening styled text into shapeable runs.
//!
//! This crate deliberately stops short of frame and column geometry. It knows
//! the document model's text and styles, and it finds *where* text may break
//! and *what* runs it is made of, but it does not place runs on a page or
//! measure them against a frame — that is the job of `repub-layout`, which
//! consumes these primitives.
//!
//! - [`LineBreaker`] — Unicode Line Breaking (UAX #14) opportunities, word
//!   segments, and a width-fitting break search.
//! - [`BreakOpportunity`] / [`TextSegment`] / [`COLUMN_BREAK_CHAR`] — the break
//!   vocabulary shared with the layouter.
//! - [`ShapeRun`] / [`paragraph_runs`] — flattening [`Paragraph`] inline
//!   elements into styled text spans plus tab and line-break markers.
//!
//! [`Paragraph`]: repub_model::text::Paragraph

#![warn(missing_docs)]

pub mod line_break;
pub mod runs;

pub use line_break::{BreakOpportunity, LineBreaker, TextSegment, COLUMN_BREAK_CHAR};
pub use runs::{paragraph_runs, styled_text_runs, ShapeRun};
