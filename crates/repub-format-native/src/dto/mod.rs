//! Serde DTO mirror types for the serialization-free `repub-model`.
//!
//! Each DTO carries the same data as its model counterpart and converts in
//! both directions with `From` impls. The top-level [`document::DocumentDto`]
//! is the serialized form of a [`repub_model::Document`].

pub mod document;
pub mod geometry;
pub mod shape;
pub mod shape_geometry;
pub mod styles;
pub mod text;
pub mod text_wrap;

pub use document::DocumentDto;
