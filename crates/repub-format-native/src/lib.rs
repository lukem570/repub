//! # Repub Native Format
//!
//! Serialization for the serialization-free [`repub_model`] document model.
//!
//! The model carries no `serde` derives, so this crate provides a parallel set
//! of DTO (data-transfer-object) mirror structs (in [`dto`]) that *do* derive
//! `serde`, together with `From` conversions in both directions. A document is
//! saved by converting it to a [`dto::DocumentDto`] and serializing that, and
//! loaded by deserializing a `DocumentDto` and rebuilding the model from it.
//!
//! Two on-disk forms are produced, matching the legacy `repub-formats`
//! native format:
//!
//! - `.repub` — a JSON file wrapping the document with a version string.
//! - `.repubz` — the same JSON stored in a single-entry ZIP container.
//!
//! The JSON wrapper is [`NativeFile`]; its `version` is checked on load.
//!
//! ## Round-trip fidelity
//!
//! The DTO layer covers the full model as of this writing: all shape kinds
//! (auto shapes, text frames, pictures, lines, groups, tables and text
//! effects), their styles, text content, effects, and document/page/layer
//! structure. Fields not yet serialized are listed in `docs/TODO.md`; nothing
//! is silently dropped.

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod dto;
pub mod error;

use std::io::{Read, Write};
use std::path::Path;

use repub_model::Document;
use serde::{Deserialize, Serialize};

pub use dto::DocumentDto;
pub use error::{NativeError, NativeResult};

/// Current native format version.
pub const FORMAT_VERSION: &str = "2.0";

/// File extension for the uncompressed native format.
pub const EXTENSION: &str = "repub";

/// File extension for the compressed native format.
pub const EXTENSION_COMPRESSED: &str = "repubz";

/// The ZIP entry name used inside a `.repubz` container.
const ZIP_ENTRY: &str = "document.json";

/// The versioned JSON wrapper written to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeFile {
    /// Format version.
    pub version: String,
    /// The serialized document.
    pub document: DocumentDto,
}

impl NativeFile {
    /// Wrap a document in a current-version [`NativeFile`].
    pub fn new(document: &Document) -> Self {
        Self {
            version: FORMAT_VERSION.to_string(),
            document: DocumentDto::from(document),
        }
    }

    /// Rebuild the model document from this file.
    pub fn into_document(self) -> Document {
        self.document.into_document()
    }
}

/// Options controlling how the native format is saved.
#[derive(Debug, Clone)]
pub struct SaveOptions {
    /// Whether to store the JSON inside a ZIP container (`.repubz`).
    pub compress: bool,
    /// Whether to pretty-print the JSON (ignored when compressing).
    pub pretty: bool,
}

impl Default for SaveOptions {
    fn default() -> Self {
        Self {
            compress: false,
            pretty: true,
        }
    }
}

impl SaveOptions {
    /// Options producing a compressed `.repubz`.
    pub fn compressed() -> Self {
        Self {
            compress: true,
            pretty: false,
        }
    }

    /// Options producing compact (non-pretty) uncompressed JSON.
    pub fn compact() -> Self {
        Self {
            compress: false,
            pretty: false,
        }
    }
}

/// Whether a version string is compatible with this reader.
///
/// Accepts the current `2.x` series.
fn is_version_compatible(version: &str) -> bool {
    version.starts_with("2.")
}

/// Detect the native format from a path's extension.
///
/// Returns `Some(true)` for the compressed `.repubz`, `Some(false)` for the
/// uncompressed `.repub` (or `.json`), and `None` for anything else.
pub fn detect_format(path: &Path) -> Option<bool> {
    path.extension()
        .and_then(|e| match e.to_string_lossy().to_lowercase().as_str() {
            "repub" => Some(false),
            "repubz" => Some(true),
            "json" => Some(false),
            _ => None,
        })
}

/// Serialize a document to native-format bytes.
///
/// When `options.compress` is set the bytes are a ZIP container; otherwise
/// they are JSON.
pub fn save_bytes(document: &Document, options: &SaveOptions) -> NativeResult<Vec<u8>> {
    let file = NativeFile::new(document);
    if options.compress {
        let json = serde_json::to_vec(&file)?;
        let mut buffer = Vec::new();
        {
            let cursor = std::io::Cursor::new(&mut buffer);
            let mut zip = zip::ZipWriter::new(cursor);
            let opts = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zip.start_file(ZIP_ENTRY, opts)?;
            zip.write_all(&json)?;
            zip.finish()?;
        }
        Ok(buffer)
    } else if options.pretty {
        Ok(serde_json::to_vec_pretty(&file)?)
    } else {
        Ok(serde_json::to_vec(&file)?)
    }
}

/// Deserialize a document from native-format bytes.
///
/// `compressed` selects the container: ZIP when `true`, JSON when `false`.
pub fn load_bytes(bytes: &[u8], compressed: bool) -> NativeResult<Document> {
    let file: NativeFile = if compressed {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)?;
        let mut entry = archive.by_name(ZIP_ENTRY).map_err(|_| {
            NativeError::InvalidStructure(format!("missing {ZIP_ENTRY} in archive"))
        })?;
        let mut json = Vec::new();
        entry.read_to_end(&mut json)?;
        serde_json::from_slice(&json)?
    } else {
        serde_json::from_slice(bytes)?
    };

    if !is_version_compatible(&file.version) {
        return Err(NativeError::VersionMismatch {
            expected: FORMAT_VERSION.to_string(),
            found: file.version,
        });
    }
    Ok(file.into_document())
}

/// Save a document to `path`.
///
/// The container is chosen by `options.compress`, independent of the file's
/// extension; callers that want extension-driven behavior should set it from
/// [`detect_format`].
pub fn save(document: &Document, path: &Path, options: &SaveOptions) -> NativeResult<()> {
    let bytes = save_bytes(document, options)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Load a document from `path`, inferring the container from the extension.
pub fn load(path: &Path) -> NativeResult<Document> {
    if !path.exists() {
        return Err(NativeError::FileNotFound(path.to_path_buf()));
    }
    let compressed = detect_format(path).unwrap_or(false);
    let bytes = std::fs::read(path)?;
    load_bytes(&bytes, compressed)
}

/// Serialize a document to a JSON string.
pub fn to_string(document: &Document, pretty: bool) -> NativeResult<String> {
    let file = NativeFile::new(document);
    if pretty {
        Ok(serde_json::to_string_pretty(&file)?)
    } else {
        Ok(serde_json::to_string(&file)?)
    }
}

/// Deserialize a document from a JSON string.
pub fn from_string(json: &str) -> NativeResult<Document> {
    let file: NativeFile = serde_json::from_str(json)?;
    if !is_version_compatible(&file.version) {
        return Err(NativeError::VersionMismatch {
            expected: FORMAT_VERSION.to_string(),
            found: file.version,
        });
    }
    Ok(file.into_document())
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_color::Color;
    use repub_geometry::Rect;
    use repub_model::shape::Shape;
    use repub_model::styles::{Fill, Stroke};
    use repub_model::{PageOrientation, PageSize};

    fn sample_doc() -> Document {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        doc.metadata.title = Some("Test".to_string());
        doc.metadata.author = Some("Agent".to_string());
        let page = doc.page_mut(0).unwrap();

        let mut rect = Shape::rectangle(Rect::new(10.0, 20.0, 100.0, 50.0));
        rect.fill = Fill::Solid(Color::from_rgb8(200, 30, 40));
        rect.line = Some(Stroke::solid(Color::BLACK, 2.0));
        page.add_shape(rect);

        let text = Shape::text_frame_with_text(Rect::new(10.0, 90.0, 200.0, 40.0), "Hello");
        page.add_shape(text);

        let ellipse = Shape::ellipse(Rect::new(300.0, 300.0, 60.0, 60.0));
        page.add_shape(ellipse);

        doc
    }

    #[test]
    fn json_round_trip_preserves_key_fields() {
        let doc = sample_doc();
        let json = to_string(&doc, true).unwrap();
        let restored = from_string(&json).unwrap();

        assert_eq!(doc.page_count(), restored.page_count());
        assert_eq!(doc.metadata.title, restored.metadata.title);
        assert_eq!(doc.metadata.author, restored.metadata.author);

        let orig_page = doc.page(0).unwrap();
        let new_page = restored.page(0).unwrap();
        assert_eq!(orig_page.shape_count(), new_page.shape_count());
        assert_eq!(orig_page.size, new_page.size);

        let orig_shapes: Vec<_> = orig_page.all_shapes().collect();
        let new_shapes: Vec<_> = new_page.all_shapes().collect();
        assert_eq!(orig_shapes.len(), new_shapes.len());
        for (a, b) in orig_shapes.iter().zip(new_shapes.iter()) {
            assert_eq!(a.common.bounds, b.common.bounds);
            assert_eq!(a.fill, b.fill);
            assert_eq!(a.line, b.line);
            assert_eq!(a.kind.shape_type(), b.kind.shape_type());
        }
    }

    #[test]
    fn full_document_equality_round_trip() {
        let doc = sample_doc();
        let json = to_string(&doc, false).unwrap();
        let restored = from_string(&json).unwrap();
        // The DTO layer is lossless for this document, so the models compare
        // equal (IDs are preserved via from_raw).
        assert_eq!(doc, restored);
    }

    #[test]
    fn uncompressed_bytes_round_trip() {
        let doc = sample_doc();
        let bytes = save_bytes(&doc, &SaveOptions::default()).unwrap();
        let restored = load_bytes(&bytes, false).unwrap();
        assert_eq!(doc.total_shape_count(), restored.total_shape_count());
    }

    #[test]
    fn compressed_bytes_round_trip() {
        let doc = sample_doc();
        let bytes = save_bytes(&doc, &SaveOptions::compressed()).unwrap();
        let restored = load_bytes(&bytes, true).unwrap();
        assert_eq!(doc, restored);
    }

    #[test]
    fn save_and_load_from_disk() {
        let doc = sample_doc();
        let dir = std::env::temp_dir();
        let path = dir.join("repub_native_test.repub");
        save(&doc, &path, &SaveOptions::default()).unwrap();
        let restored = load(&path).unwrap();
        assert_eq!(doc, restored);
        let _ = std::fs::remove_file(&path);

        let zpath = dir.join("repub_native_test.repubz");
        save(&doc, &zpath, &SaveOptions::compressed()).unwrap();
        let zrestored = load(&zpath).unwrap();
        assert_eq!(doc, zrestored);
        let _ = std::fs::remove_file(&zpath);
    }

    #[test]
    fn detect_format_by_extension() {
        assert_eq!(detect_format(Path::new("d.repub")), Some(false));
        assert_eq!(detect_format(Path::new("d.repubz")), Some(true));
        assert_eq!(detect_format(Path::new("d.json")), Some(false));
        assert_eq!(detect_format(Path::new("d.pub")), None);
        assert_eq!(detect_format(Path::new("d.txt")), None);
    }

    #[test]
    fn version_mismatch_is_rejected() {
        let doc = sample_doc();
        let mut file = NativeFile::new(&doc);
        file.version = "1.0".to_string();
        let json = serde_json::to_string(&file).unwrap();
        assert!(matches!(
            from_string(&json),
            Err(NativeError::VersionMismatch { .. })
        ));
    }
}
