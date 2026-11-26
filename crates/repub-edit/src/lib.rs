//! # Repub Edit
//!
//! The framework-free editing engine for the Repub desktop publishing system.
//!
//! This crate holds the editing logic that sits above the document model
//! ([`repub_model`]) and below any user interface: the command pattern and
//! undo/redo history, the editing application state (selection, clipboard,
//! page view, master-page editing), in-place text editing, character and
//! paragraph formatting, linked text-frame flow, grid/guide/object snapping,
//! spell checking, hyperlink destinations, document-tree selection
//! hit-testing, path-point editing, and the drawing/selection tool state
//! machine.
//!
//! It depends only on the foundation and model crates ([`repub_geometry`],
//! [`repub_color`], [`repub_model`]) and carries no GUI framework dependency,
//! so it can be driven by the GUI, the CLI, an MCP server, or tests alike.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod commands;
pub mod edit_points;
pub mod error;
pub mod hit_test;
pub mod hyperlinks;
pub mod snapping;
pub mod spelling;
pub mod state;
pub mod text_chain;
pub mod text_edit;
pub mod text_format;
pub mod tools;

pub use commands::{
    AddPageCommand, AddShapeCommand, Command, CommandHistory, DeletePageCommand,
    DeleteShapesCommand, MoveShapesCommand, ResizeShapeCommand, SnapshotCommand,
};
pub use edit_points::{anchor_at, anchor_points, move_anchor, EditPointsState};
pub use error::{EditError, EditResult};
pub use hit_test::{member_at, shape_at, Hit, HIT_TOLERANCE_PX};
pub use hyperlinks::{destination, open_address, Destination};
pub use snapping::{SnapAxis, SnapConfig, SnapEngine, SnapResult, SnapTarget, SNAP_DISTANCE};
pub use spelling::{Misspelled, SpellSession};
pub use state::{
    AppState, CopiedFormat, DisplayUnit, Guide, GuideOrientation, MasterEdit, PageDragState,
};
pub use text_chain::{break_link, chain, head, layout_chains, link, previous, ChainFrame};
pub use text_edit::{TextEditState, TextPos};
pub use text_format::{CharChange, ListKind, Toggle};
pub use tools::{CursorKind, DrawState, HandleType, ResizeHandle, SelectState, Tool};

/// Repub edit library version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
