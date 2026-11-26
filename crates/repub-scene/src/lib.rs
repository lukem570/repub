//! # Repub Scene
//!
//! The display-list keystone: a serialization-free, per-frame scene of flat
//! drawing primitives, and the [`compose`] function that builds one from a
//! document page.
//!
//! A [`Scene`] is an immutable list of [`Primitive`]s produced fresh by
//! [`compose`] — there is no retained or diff state. [`compose`] traverses a
//! page's layers and shapes in z-order and resolves each
//! [`Shape`](repub_model::shape::Shape) into primitives:
//!
//! - **Auto shapes** resolve their geometry (an Office preset via
//!   [`repub_escher`], or their own path) into a [`Primitive::FillPath`] and/or
//!   [`Primitive::StrokePath`].
//! - **Text frames** are laid out with [`repub_layout`] and emitted as
//!   [`Primitive::Glyphs`] runs over a background fill and border.
//! - **Tables** emit per-cell fills, grid borders (stroked paths), and cell
//!   text.
//! - **Pictures** emit a [`Primitive::Image`].
//! - **Lines** emit a stroked path plus filled arrowhead shapes.
//! - **Text effects** (WordArt and text-on-path) are resolved into glyph
//!   outline paths — see [`wordart`].
//! - **Groups** recurse, composing the group's transform with each child's.
//!
//! Geometry, warps, text-on-path and 3D extrusion are fully resolved into
//! concrete primitives. Blur-style effects (shadow, glow, reflection, bevel,
//! soft edges) are carried as [`SceneEffect`] descriptors for a backend to
//! realize; they are never pre-baked into geometry.
//!
//! ## WordArt fidelity
//!
//! The WordArt resolver ([`wordart`]) ports the GUI's warp envelopes (all 40+
//! [`WarpPreset`](wordart::WarpPreset) variants) and the glyph-outline warp
//! transform, producing scene paths instead of egui meshes. It is intentionally
//! simpler than the GUI in these respects:
//!
//! - **3D extrusion** is approximated by emitting depth-shaded offset copies of
//!   the front face behind it (the GUI's layered-fill approach), but the GUI's
//!   per-edge quad strips, bevel highlight meshes, contour highlights, and the
//!   lighting/material/rotation parameter model are not ported. Only
//!   [`Extrusion3D`](repub_model::shape::Extrusion3D) depth and color drive the
//!   result.
//! - **Free-form warp** ([`TextTransform::Warp`](repub_model::shape::TextTransform::Warp))
//!   is not implemented and falls back to plain linear mapping.
//! - **Text-on-path** places glyph outlines by arc length along the flattened
//!   path, rotated to the local tangent; path alignment, spacing modes and the
//!   flip option are not yet honored.
//! - Font outlines come from the installed-font registry or the bundled
//!   fallback font; the per-glyph font matching is coarser than the GUI's.

#![warn(missing_docs)]
#![warn(clippy::all)]

mod compose;
mod convert;

pub mod glyph;
pub mod wordart;

mod primitive;

pub use compose::{compose, compose_with_scheme, empty_scene};
pub use convert::{
    fill_to_paint, model_path_fill_rule, model_path_to_geom, paint_with_opacity, stroke_paint,
};
pub use primitive::{
    FillRule, FontRef, ImageRef, LineCap, LineJoin, Paint, PositionedGlyph, Primitive, Scene,
    SceneEffect,
};
