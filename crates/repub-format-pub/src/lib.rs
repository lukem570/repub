//! # Repub Format: Publisher (`.pub`)
//!
//! Microsoft Publisher `.pub` import and export for Repub documents, built on
//! the [`repub_model`] document model.
//!
//! Publisher files are OLE Compound Documents with several streams: `Contents`
//! (page/shape structure), `Escher`/`EscherStm` (drawing data), and
//! `Quill/QuillSub/CONTENTS` (text). This crate parses those streams into a
//! [`repub_model::Document`] and writes a document back out.
//!
//! This crate performs no rendering and decodes no images: embedded image
//! bytes are preserved as the picture shape's embedded source, and decoding is
//! left to export/scene layers.
//!
//! ## Format reference
//!
//! The on-disk `.pub` format is documented as an ordered implementer's guide
//! under this crate's `docs/` directory, starting at `docs/00-overview.md`.
//! Byte layouts are given as C structs. Every block the parser reads is
//! explained there; any bytes read but not yet interpreted are tracked
//! explicitly in `docs/04-unresolved.md` (the "no unknown blocks" ledger),
//! which is empty when the format is fully specified.
//!
//! ## Import
//!
//! Import is **strict** by default: if the parser cannot fully read every item
//! in the file, [`import`] returns [`FormatError::UnreadItems`] listing what it
//! could not read, rather than silently dropping it. Use [`import_lenient`] for
//! best-effort recovery that records unreadable items as warnings instead.
//!
//! ```ignore
//! let document = repub_format_pub::import("document.pub")?; // errors on any unread item
//! let document = repub_format_pub::import_lenient("document.pub")?; // best effort
//! ```
//!
//! ## Export
//!
//! The export produces a valid OLE structure, but its content encoding does
//! not yet fully match Publisher (see `docs/TODO.md`).
//!
//! ```ignore
//! repub_format_pub::export(&document, "out.pub")?;
//! ```

#![warn(missing_docs)]

pub mod compat;
pub mod error;
pub mod model;

// The ported parser/serializer modules carry many internal public items that
// were undocumented in the original implementation. Documenting each of the
// thousands of internal items is impractical for a faithful port, so these
// implementation modules opt out of the crate-wide `missing_docs` lint; the
// crate's public entry points (below, and in `error`/`model`) stay documented.
#[allow(missing_docs)]
pub mod contents;
#[allow(missing_docs)]
pub mod escher;
#[allow(missing_docs)]
pub mod export;
#[allow(missing_docs)]
pub mod import;
#[allow(missing_docs)]
pub mod quill;

// The ported import/export/compat code refers to these modules under their
// original names; keep those paths working.
pub use compat as pub_compat;
pub use export as pub_export;
pub use import as pub_import;

pub use error::{FormatError, FormatResult, PubImportError};

use std::path::Path;

use repub_model::Document;

/// Import a Publisher `.pub` file from a path into a [`Document`].
pub fn import<P: AsRef<Path>>(path: P) -> FormatResult<Document> {
    import::import(path)
}

/// Import a Publisher `.pub` file from in-memory bytes into a [`Document`].
pub fn import_bytes(bytes: &[u8]) -> FormatResult<Document> {
    import::import_bytes(bytes)
}

/// Best-effort import from a path: recover what can be read instead of failing
/// on unreadable items. See the module docs on strictness.
pub fn import_lenient<P: AsRef<Path>>(path: P) -> FormatResult<Document> {
    import::import_lenient(path)
}

/// Best-effort import from in-memory bytes. See [`import_lenient`].
pub fn import_bytes_lenient(bytes: &[u8]) -> FormatResult<Document> {
    import::import_bytes_lenient(bytes)
}

/// Export a [`Document`] to a Publisher `.pub` file at a path.
pub fn export<P: AsRef<Path>>(doc: &Document, path: P) -> FormatResult<()> {
    export::export(doc, path, &export::PubExportOptions::default())
}

/// Export a [`Document`] to a Publisher `.pub` file as in-memory bytes.
pub fn export_bytes(doc: &Document) -> FormatResult<Vec<u8>> {
    export::export_bytes(doc, &export::PubExportOptions::default())
}
