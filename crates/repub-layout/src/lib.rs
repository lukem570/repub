//! # Repub Layout
//!
//! Document layout engine for positioning and flowing content, built on the
//! clean lower layers ([`repub_geometry`], [`repub_model`], [`repub_fonts`] and
//! [`repub_text`]).
//!
//! This crate provides:
//! - Text frame layout (single and multi-column), vertical alignment, tab
//!   stops, truncation and hanging punctuation.
//! - Text overflow and linking between frames.
//! - Shape positioning, alignment, distribution and grid/guide snapping.
//! - Table layout.
//! - Layout caching for incremental updates.
//! - Page layout.
//!
//! Every object on a page is a [`repub_model::shape::Shape`]; a text frame is a
//! `Shape` whose [`repub_model::shape::ShapeKind::TextFrame`] carries a
//! [`repub_model::shape::TextFrameData`], and a table a
//! [`repub_model::shape::ShapeKind::Table`] with a
//! [`repub_model::shape::TableData`]. The layouters read placement from the
//! shape's common bounds and content from the kind-specific payload.
//!
//! ## Example
//!
//! ```rust,ignore
//! use repub_layout::{LayoutEngine};
//! use repub_fonts::SystemFontProvider;
//! use repub_model::shape::Shape;
//! use repub_geometry::Rect;
//! use std::sync::Arc;
//!
//! let mut engine = LayoutEngine::new(Arc::new(SystemFontProvider::default()));
//! let frame = Shape::text_frame_with_text(Rect::new(0.0, 0.0, 200.0, 100.0), "Hello");
//! if let repub_model::shape::ShapeKind::TextFrame(data) = &frame.kind {
//!     let layout = engine.layout_text_frame(&frame, data);
//! }
//! ```

#![warn(missing_docs)]

pub mod error;
pub mod object;
pub mod page;
pub mod table;
pub mod text;

mod cache;
mod engine;

pub use cache::LayoutCache;
pub use engine::{
    layout_linked_frames, layout_linked_frames_simple, LayoutEngine, LinkedFrameResult,
};
pub use error::{LayoutError, LayoutResult};
pub use object::{Alignment, Distribution, ObjectLayout, Snap};
pub use page::PageLayout;
pub use table::{
    CellConstraints, CellContentMeasurement, CellLayout, TableLayout, TableLayouter,
    VerticalAlignment as TableVerticalAlignment,
};
pub use text::{
    ColumnBalanceOptions, HangingPunctuation, LayoutColumn, LayoutLine, LayoutParagraph, LayoutRun,
    TabAlignment, TabStop, TextFrameLayout, TextLayouter, TruncationOptions, TruncationPosition,
    VerticalAlignment, LEFT_HANG_PUNCTUATION, RIGHT_HANG_PUNCTUATION,
};
