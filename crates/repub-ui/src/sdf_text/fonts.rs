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
    pub path: PathBuf,
    /// Face index within a collection (0 for single fonts)
    pub index: u32,
    /// OS/2 weight class (400 regular, 700 bold)
    pub weight: u16,
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
                let start = std::time::Instant::now();
                let r = registry();
                log::info!(
                    "Found {} font families in {:.0} ms",
                    r.families.len(),
                    start.elapsed().as_secs_f64() * 1000.0
                );
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
        let mut out: Vec<String> = self
            .families
            .values()
            .map(|(name, _)| name.clone())
            .collect();
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
        let windir =
            std::env::var_os("WINDIR").map_or_else(|| PathBuf::from("C:\\Windows"), PathBuf::from);
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
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
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
fn faces_in(path: &Path) -> Vec<(String, FontFile)> {
    if std::fs::metadata(path).map_or(true, |m| m.len() > MAX_FILE) {
        return Vec::new();
    }
    let Ok(data) = std::fs::read(path) else {
        return Vec::new();
    };
    let count = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
    let mut out = Vec::new();
    for index in 0..count {
        let Ok(face) = ttf_parser::Face::parse(&data, index) else {
            continue;
        };
        // Hidden system families (".SF NS", ".Aqua...") aren't for documents
        let Some(name) = family_name(&face).filter(|n| !n.starts_with('.')) else {
            continue;
        };
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
#[derive(serde::Serialize, serde::Deserialize)]
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

fn scan() -> Registry {
    let mut files = Vec::new();
    for dir in font_dirs() {
        walk(&dir, 0, &mut files);
    }
    // Files unchanged since the last scan come from the cache; the rest are read
    let cache_path = cache_dir().map(|d| d.join("fonts.json"));
    let cached: std::collections::HashMap<PathBuf, CachedFile> = cache_path
        .as_ref()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|bytes| serde_json::from_slice::<Vec<CachedFile>>(&bytes).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|c| (c.path.clone(), c))
        .collect();
    let mut entries: Vec<CachedFile> = Vec::new();
    let mut to_read: Vec<(PathBuf, u64, u64)> = Vec::new();
    for path in files {
        let Some((size, modified)) = file_stamp(&path) else {
            continue;
        };
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
            if let (Some(dir), Ok(json)) = (p.parent(), serde_json::to_vec(&entries)) {
                let _ = std::fs::create_dir_all(dir);
                let _ = std::fs::write(p, json);
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
