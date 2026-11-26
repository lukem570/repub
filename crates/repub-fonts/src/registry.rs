//! Installed fonts, by family.
//!
//! The system font folders are scanned once, on a background thread, reading
//! each font's family name, weight and italic flag (every face of a `.ttc`
//! collection). Text in a family that isn't installed falls back to the
//! bundled Selawik.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// One installed face.
#[derive(Debug, Clone)]
pub struct FontFile {
    /// Path to the font file.
    pub path: PathBuf,
    /// Face index within a collection (0 for single fonts)
    pub index: u32,
    /// OS/2 weight class (400 regular, 700 bold)
    pub weight: u16,
    /// Whether the face is italic.
    pub italic: bool,
}

/// Installed families: lower-case name -> (display name, faces).
pub struct Registry {
    families: BTreeMap<String, (String, Vec<FontFile>)>,
}

/// Families bundled with the app (always available).
pub const BUNDLED: &str = "Selawik";

/// Largest font file read (CJK collections run to tens of megabytes).
const MAX_FILE: u64 = 40 * 1024 * 1024;

static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Start scanning the font folders on a background thread (idempotent).
pub fn scan_in_background() {
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        let _ = std::thread::Builder::new()
            .name("font-scan".into())
            .spawn(|| {
                let _ = registry();
            });
    });
}

/// The registry, scanning on this thread if the background scan hasn't run.
pub fn registry() -> &'static Registry {
    REGISTRY.get_or_init(scan)
}

/// The registry if the scan has finished.
pub fn try_registry() -> Option<&'static Registry> {
    REGISTRY.get()
}

impl Registry {
    /// Family names for a font menu: the bundled family and every installed one.
    pub fn family_names(&self) -> Vec<String> {
        let mut out: Vec<String> = self.families.values().map(|(name, _)| name.clone()).collect();
        if !self.families.contains_key(&BUNDLED.to_lowercase()) {
            out.push(BUNDLED.to_owned());
        }
        out.sort_by_key(|n| n.to_lowercase());
        out
    }

    /// Whether `family` is installed.
    pub fn has(&self, family: &str) -> bool {
        self.families.contains_key(&family.trim().to_lowercase())
    }

    /// The installed face of `family` closest to `bold`/`italic`.
    pub fn find(&self, family: &str, bold: bool, italic: bool) -> Option<&FontFile> {
        let (_, faces) = self.families.get(&family.trim().to_lowercase())?;
        let want = if bold { 700 } else { 400 };
        faces.iter().min_by_key(|f| {
            let italic_miss = if f.italic == italic { 0 } else { 10_000 };
            italic_miss + (i32::from(f.weight) - want).unsigned_abs()
        })
    }
}

fn font_dirs() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut dirs: Vec<PathBuf> = Vec::new();
    if cfg!(target_os = "macos") {
        dirs.extend(["/System/Library/Fonts", "/Library/Fonts"].map(PathBuf::from));
        dirs.extend(home.map(|h| h.join("Library/Fonts")));
    } else if cfg!(target_os = "windows") {
        let windir = std::env::var_os("WINDIR")
            .map_or_else(|| PathBuf::from("C:\\Windows"), PathBuf::from);
        dirs.push(windir.join("Fonts"));
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            dirs.push(PathBuf::from(local).join("Microsoft\\Windows\\Fonts"));
        }
    } else {
        dirs.extend(["/usr/share/fonts", "/usr/local/share/fonts"].map(PathBuf::from));
        if let Some(h) = home {
            dirs.push(h.join(".local/share/fonts"));
            dirs.push(h.join(".fonts"));
        }
    }
    dirs
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth < 4 {
                walk(&path, depth + 1, out);
            }
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc"))
        {
            out.push(path);
        }
    }
}

/// A face's family name: the typographic family, else the plain family.
fn family_name(face: &ttf_parser::Face) -> Option<String> {
    let names = face.names();
    let get = |id: u16| {
        names
            .into_iter()
            .filter(|n| n.name_id == id && n.is_unicode())
            .find_map(|n| n.to_string())
            .filter(|s| !s.trim().is_empty())
    };
    get(ttf_parser::name_id::TYPOGRAPHIC_FAMILY).or_else(|| get(ttf_parser::name_id::FAMILY))
}

/// The faces in one font file: (family, face).
pub fn faces_in(path: &Path) -> Vec<(String, FontFile)> {
    if std::fs::metadata(path).map_or(true, |m| m.len() > MAX_FILE) {
        return Vec::new();
    }
    let Ok(data) = std::fs::read(path) else { return Vec::new() };
    let count = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
    let mut out = Vec::new();
    for index in 0..count {
        let Ok(face) = ttf_parser::Face::parse(&data, index) else { continue };
        // Hidden system families (".SF NS", ".Aqua...") aren't for documents
        let Some(name) = family_name(&face).filter(|n| !n.starts_with('.')) else { continue };
        // Glyph outlines are needed (bitmap-only emoji fonts are skipped)
        if face.tables().glyf.is_none() && face.tables().cff.is_none() {
            continue;
        }
        out.push((
            name,
            FontFile {
                path: path.to_path_buf(),
                index,
                weight: face.weight().to_number(),
                italic: face.is_italic(),
            },
        ));
    }
    out
}

/// Where Repub keeps caches (font list, baked glyphs).
pub fn cache_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("com", "repub", "Repub").map(|d| d.cache_dir().to_path_buf())
}

/// A font file as last scanned: identified by size and modification time.
struct CachedFile {
    path: PathBuf,
    size: u64,
    modified: u64,
    /// (family, index, weight, italic)
    faces: Vec<(String, u32, u16, bool)>,
}

/// Size and modification time (seconds) of a file.
pub fn file_stamp(path: &Path) -> Option<(u64, u64)> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();
    Some((meta.len(), modified))
}

/// Serialize the cache as a simple line-based text format. One record per
/// line: `size<TAB>modified<TAB>path<TAB>family<US>index<US>weight<US>italic...`
/// where faces are separated by the record separator (`\x1e`) and the four
/// face fields by the unit separator (`\x1f`). Paths cannot contain tabs,
/// newlines or these control characters on any supported platform.
fn serialize_cache(entries: &[CachedFile]) -> String {
    let mut out = String::new();
    for entry in entries {
        out.push_str(&entry.size.to_string());
        out.push('\t');
        out.push_str(&entry.modified.to_string());
        out.push('\t');
        out.push_str(&entry.path.to_string_lossy());
        for (family, index, weight, italic) in &entry.faces {
            out.push('\x1e');
            out.push_str(family);
            out.push('\x1f');
            out.push_str(&index.to_string());
            out.push('\x1f');
            out.push_str(&weight.to_string());
            out.push('\x1f');
            out.push(if *italic { '1' } else { '0' });
        }
        out.push('\n');
    }
    out
}

/// Parse the cache written by [`serialize_cache`]. Malformed lines are skipped.
fn deserialize_cache(text: &str) -> Vec<CachedFile> {
    let mut entries = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        let mut records = line.split('\x1e');
        let Some(head) = records.next() else { continue };
        let mut head_fields = head.splitn(3, '\t');
        let (Some(size), Some(modified), Some(path)) =
            (head_fields.next(), head_fields.next(), head_fields.next())
        else {
            continue;
        };
        let (Ok(size), Ok(modified)) = (size.parse::<u64>(), modified.parse::<u64>()) else {
            continue;
        };
        let mut faces = Vec::new();
        for record in records {
            let mut f = record.split('\x1f');
            let (Some(family), Some(index), Some(weight), Some(italic)) =
                (f.next(), f.next(), f.next(), f.next())
            else {
                continue;
            };
            let (Ok(index), Ok(weight)) = (index.parse::<u32>(), weight.parse::<u16>()) else {
                continue;
            };
            faces.push((family.to_string(), index, weight, italic == "1"));
        }
        entries.push(CachedFile {
            path: PathBuf::from(path),
            size,
            modified,
            faces,
        });
    }
    entries
}

fn scan() -> Registry {
    let mut files = Vec::new();
    for dir in font_dirs() {
        walk(&dir, 0, &mut files);
    }
    // Files unchanged since the last scan come from the cache; the rest are read
    let cache_path = cache_dir().map(|d| d.join("fonts.cache"));
    let cached: std::collections::HashMap<PathBuf, CachedFile> = cache_path
        .as_ref()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .map(|text| deserialize_cache(&text))
        .unwrap_or_default()
        .into_iter()
        .map(|c| (c.path.clone(), c))
        .collect();
    let mut entries: Vec<CachedFile> = Vec::new();
    let mut to_read: Vec<(PathBuf, u64, u64)> = Vec::new();
    for path in files {
        let Some((size, modified)) = file_stamp(&path) else { continue };
        match cached.get(&path) {
            Some(c) if c.size == size && c.modified == modified => {
                entries.push(CachedFile {
                    path,
                    size,
                    modified,
                    faces: c.faces.clone(),
                });
            }
            _ => to_read.push((path, size, modified)),
        }
    }
    let changed = !to_read.is_empty() || entries.len() != cached.len();
    if !to_read.is_empty() {
        // Reading and parsing font files: spread over all cores
        let threads = std::thread::available_parallelism()
            .map_or(4, |n| n.get())
            .clamp(1, 16);
        let chunk = to_read.len().div_ceil(threads).max(1);
        let read: Vec<CachedFile> = std::thread::scope(|scope| {
            let workers: Vec<_> = to_read
                .chunks(chunk)
                .map(|part| {
                    scope.spawn(move || {
                        part.iter()
                            .map(|(path, size, modified)| CachedFile {
                                path: path.clone(),
                                size: *size,
                                modified: *modified,
                                faces: faces_in(path)
                                    .into_iter()
                                    .map(|(name, f)| (name, f.index, f.weight, f.italic))
                                    .collect(),
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
            workers
                .into_iter()
                .flat_map(|w| w.join().unwrap_or_default())
                .collect()
        });
        entries.extend(read);
    }
    if changed {
        if let Some(p) = &cache_path {
            if let Some(dir) = p.parent() {
                let _ = std::fs::create_dir_all(dir);
                let _ = std::fs::write(p, serialize_cache(&entries));
            }
        }
    }
    let mut families: BTreeMap<String, (String, Vec<FontFile>)> = BTreeMap::new();
    for entry in entries {
        for (name, index, weight, italic) in entry.faces {
            let file = FontFile {
                path: entry.path.clone(),
                index,
                weight,
                italic,
            };
            families
                .entry(name.to_lowercase())
                .or_insert_with(|| (name.clone(), Vec::new()))
                .1
                .push(file);
        }
    }
    Registry { families }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_round_trips() {
        let entries = vec![
            CachedFile {
                path: PathBuf::from("/a/b/Foo.ttf"),
                size: 1234,
                modified: 5678,
                faces: vec![("Foo".to_string(), 0, 400, false)],
            },
            CachedFile {
                path: PathBuf::from("/x/Collection.ttc"),
                size: 9,
                modified: 10,
                faces: vec![
                    ("Alpha".to_string(), 0, 400, false),
                    ("Beta".to_string(), 1, 700, true),
                ],
            },
        ];
        let text = serialize_cache(&entries);
        let parsed = deserialize_cache(&text);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].path, PathBuf::from("/a/b/Foo.ttf"));
        assert_eq!(parsed[0].size, 1234);
        assert_eq!(parsed[0].faces[0], ("Foo".to_string(), 0, 400, false));
        assert_eq!(parsed[1].faces.len(), 2);
        assert_eq!(parsed[1].faces[1], ("Beta".to_string(), 1, 700, true));
    }

    #[test]
    fn cache_skips_malformed_lines() {
        let text = "garbage line\n1234\t5678\t/ok.ttf\x1eFam\x1f0\x1f400\x1f0\n";
        let parsed = deserialize_cache(text);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].path, PathBuf::from("/ok.ttf"));
    }
}
