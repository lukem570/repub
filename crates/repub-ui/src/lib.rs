//! # Repub GUI
//!
//! Desktop GUI application for Repub using egui.
//!
//! Features a procedurally generated grainy workspace background.
//!
//! This crate provides:
//! - Main application window with menu bar
//! - Document editing canvas with pan/zoom
//! - Pages panel with thumbnails
//! - Format panel for object properties
//! - Tool palette for drawing and editing
//! - Keyboard shortcuts
//!
//! ## Architecture
//!
//! The GUI is built on egui/eframe for cross-platform support
//! and WebAssembly compatibility. The architecture follows:
//!
//! - **App**: Main application state and event loop
//! - **Canvas**: Document rendering and interaction
//! - **Tools**: Selection, text, shape tools
//! - **Panels**: Pages, format, properties panels
//! - **Dialogs**: New document, export, etc.
//! - **Commands**: Undo/redo command pattern

pub mod app;
pub mod background;
pub mod canvas;
pub mod commands;
pub mod context_menu;
pub mod dialogs;
pub mod edit_points;
pub mod gradient;
pub mod hit_test;
pub mod hyperlinks;
pub mod icons;
pub mod menus;
#[cfg(not(target_arch = "wasm32"))]
pub mod native_menu;
pub mod panels;
pub mod pattern_fill;
pub mod sdf_text;
pub mod settings;
pub mod shape_gallery;
pub mod snapping;
pub mod spelling;
pub mod start_screen;
pub mod state;
pub mod table_view;
pub mod text_chain;
pub mod text_edit;
pub mod text_format;
pub mod theme;
pub mod tools;
pub mod widgets;
pub mod wordart_path;
pub mod wordart_warp;

pub use app::RepubApp;
pub use settings::Settings;
pub use state::AppState;
