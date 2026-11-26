//! Multi-channel signed-distance-field (MTSDF) glyph atlas.
//!
//! Each glyph's vector outline is turned into an MTSDF with `fdsm` (a Rust port
//! of msdfgen). RGB holds three distances to differently coloured edge sets:
//! their median reconstructs the outline with sharp corners, which a single
//! distance (rounding every corner) cannot. Alpha holds the true single-channel
//! distance, used when glyphs are minified so far that the median breaks down.
//!
//! In every channel 0.5 is the outline and values above are inside; the
//! distance range is `±SPREAD` atlas texels.
//!
//! Built on a background thread with the bundled Selawik regular and bold;
//! installed fonts are added as text uses them (see [`super::fonts`]).

use std::collections::HashMap;
use std::sync::OnceLock;

use fdsm::bezier::scanline::FillRule;
use fdsm::shape::Shape;
use fdsm::transform::Transform;

/// Atlas resolution: texels per em.
pub const EM_PX: f32 = 64.0;
/// Distance range encoded on each side of the outline, in atlas texels.
pub const SPREAD: f32 = 8.0;
/// Atlas texture width and height.
pub const ATLAS_SIZE: usize = 2048;
/// Bytes per atlas texel (RGBA).
pub const CHANNELS: usize = 4;

/// One glyph's atlas location and placement.
#[derive(Debug, Clone, Copy)]
pub struct Glyph {
    /// Texture coordinates `[u0, v0, u1, v1]`. Zero-size for blank glyphs (space).
    pub uv: [f32; 4],
    /// Quad relative to the pen position on the baseline, in em units, y down:
    /// `[x0, y0, x1, y1]`. Includes the `SPREAD` padding.
    pub plane: [f32; 4],
    /// Horizontal advance in em units.
    pub advance: f32,
}

impl Glyph {
    /// Whether the glyph has visible ink (spaces don't).
    pub fn has_quad(&self) -> bool {
        self.uv[2] > self.uv[0]
    }
}

/// Face index of the bundled Selawik regular.
pub const REGULAR: usize = 0;
/// Face index of the bundled Selawik bold.
pub const BOLD: usize = 1;

/// One font face: its font data, baked glyphs and line metrics.
pub struct Face {
    data: std::sync::Arc<Vec<u8>>,
    index: u32,
    /// Installed font file (None for the bundled faces)
    path: Option<std::path::PathBuf>,
    /// Glyphs rendered in earlier sessions (see [`GlyphCache`])
    cache: GlyphCache,
    font: fontdue::Font,
    glyphs: HashMap<char, Glyph>,
    /// Characters the face has no glyph for (not looked up again)
    missing: std::collections::HashSet<char>,
    /// Ascent above the baseline, em units (positive).
    pub ascent: f32,
    /// Descent below the baseline, em units (positive).
    pub descent: f32,
    /// Extra gap between lines, em units.
    pub line_gap: f32,
    /// A true italic face (no synthetic slant needed)
    pub italic: bool,
}

impl Face {
    /// Glyph for `c`, falling back to `?` for characters the face lacks.
    pub fn glyph(&self, c: char) -> Option<&Glyph> {
        self.glyphs.get(&c).or_else(|| self.glyphs.get(&'?'))
    }

    /// Kerning adjustment between two characters, em units.
    pub fn kern(&self, left: char, right: char) -> f32 {
        self.font.horizontal_kern(left, right, EM_PX).unwrap_or(0.0) / EM_PX
    }

    /// Whether `c` has been baked (or is known to be missing).
    fn has(&self, c: char) -> bool {
        self.glyphs.contains_key(&c) || self.missing.contains(&c)
    }
}

/// Which face a style uses: lower-case family, bold, italic.
type FaceKey = (String, bool, bool);

/// Whether a style asks for a bold face.
pub fn is_bold(style: &repub_model::styles::CharacterStyle) -> bool {
    use repub_model::styles::FontWeight;
    matches!(
        style.font.weight,
        FontWeight::SemiBold | FontWeight::Bold | FontWeight::ExtraBold | FontWeight::Black
    )
}

/// Whether a style asks for italic.
pub fn is_italic(style: &repub_model::styles::CharacterStyle) -> bool {
    use repub_model::styles::FontStyle;
    matches!(style.font.style, FontStyle::Italic | FontStyle::Oblique)
}

fn key_of(style: &repub_model::styles::CharacterStyle) -> FaceKey {
    (
        style.font.family.trim().to_lowercase(),
        is_bold(style),
        is_italic(style),
    )
}

/// The atlas: RGBA8 MTSDF texels plus the faces drawn from it. Faces are
/// added when text first uses them, and glyphs baked as they're needed.
pub struct Atlas {
    faces: Vec<Face>,
    /// Style key -> face index (installed face, or the Selawik fallback)
    keys: HashMap<FaceKey, usize>,
    /// `ATLAS_SIZE * ATLAS_SIZE` RGBA texels (MSDF in RGB, true distance in A).
    pub pixels: Vec<u8>,
    /// Texture coordinate of a texel that is fully inside (255): used for
    /// underlines and other solid rectangles.
    pub solid_uv: [f32; 2],
    packer: Packer,
    /// First atlas row for installed faces' glyphs (the bundled faces sit above)
    shared_from: usize,
    /// Bumped whenever texels change, so the GPU copy is refreshed.
    pub generation: u64,
}

impl Atlas {
    /// Face `id` (Selawik regular for an unknown index).
    pub fn face(&self, id: usize) -> &Face {
        self.faces.get(id).unwrap_or(&self.faces[REGULAR])
    }

    /// The face a style is drawn with, and whether it needs a synthetic slant
    /// (italic asked for, but the face isn't italic). Faces not yet loaded
    /// ([`Self::load`]) use Selawik.
    pub fn resolve(&self, style: &repub_model::styles::CharacterStyle) -> (usize, bool) {
        let key = key_of(style);
        let id = self
            .keys
            .get(&key)
            .copied()
            .unwrap_or(if key.1 { BOLD } else { REGULAR });
        (id, key.2 && !self.face(id).italic)
    }

    /// Whether the style's face has been looked up (loaded or known missing).
    pub fn knows(&self, style: &repub_model::styles::CharacterStyle) -> bool {
        self.keys.contains_key(&key_of(style))
    }

    /// Whether face `id` has `c` baked (or knows it lacks it).
    pub fn has_glyph(&self, id: usize, c: char) -> bool {
        self.face(id).has(c)
    }

    /// Look up the style's installed face and load it (printable ASCII baked
    /// straight away). Families that aren't installed use Selawik.
    pub fn load(&mut self, style: &repub_model::styles::CharacterStyle) {
        let key = key_of(style);
        if self.keys.contains_key(&key) {
            return;
        }
        let fallback = if key.1 { BOLD } else { REGULAR };
        let installed = if key.0 == super::fonts::BUNDLED.to_lowercase() {
            None
        } else {
            // Waits for the font scan if it's still running (a fraction of a
            // second), so text is never laid out in a stand-in font first
            super::fonts::registry().find(&key.0, key.1, key.2).cloned()
        };
        // The same file may serve several keys (no bold face: bold uses the regular one)
        let id = match installed {
            Some(file) => {
                let existing = self
                    .faces
                    .iter()
                    .position(|f| f.index == file.index && f.path.as_ref() == Some(&file.path));
                match existing {
                    Some(id) => id,
                    None => match std::fs::read(&file.path) {
                        Ok(bytes) => match face_from(
                            std::sync::Arc::new(bytes),
                            file.index,
                            file.italic,
                            // Changed font files get a fresh glyph cache
                            &format!(
                                "{}:{:?}",
                                file.path.display(),
                                super::fonts::file_stamp(&file.path)
                            ),
                        ) {
                            Some(mut face) => {
                                face.path = Some(file.path.clone());
                                self.faces.push(face);
                                self.faces.len() - 1
                            }
                            None => fallback,
                        },
                        Err(_) => fallback,
                    },
                }
            }
            None => fallback,
        };
        self.keys.insert(key, id);
    }

    /// Bake `c` into face `id` if it isn't there yet.
    pub fn bake(&mut self, id: usize, c: char) {
        self.bake_many(id, &[c]);
    }

    /// Bake the characters of `chars` that face `id` doesn't have yet,
    /// rendering them on all cores.
    pub fn bake_many(&mut self, id: usize, chars: &[char]) {
        let Some(face) = self.faces.get(id) else {
            return;
        };
        let mut todo: Vec<char> = chars.iter().copied().filter(|&c| !face.has(c)).collect();
        todo.sort_unstable();
        todo.dedup();
        if todo.is_empty() {
            return;
        }
        // Glyphs rendered before come from the cache; the rest are rendered now
        // (and added to the cache)
        let (mut ready, fresh): (Vec<(char, Option<Rendered>)>, Vec<char>) = {
            let mut ready = Vec::new();
            let mut fresh = Vec::new();
            for c in todo {
                match face.cache.get(c) {
                    Some(r) => ready.push((c, r)),
                    None => fresh.push(c),
                }
            }
            (ready, fresh)
        };
        if !fresh.is_empty() {
            let data = face.data.clone();
            let rendered = render_many(&data, face.index, &fresh);
            self.faces[id].cache.append(&rendered);
            ready.extend(rendered);
        }
        for (c, r) in ready {
            let Some(r) = r else {
                self.faces[id].missing.insert(c);
                continue;
            };
            let mut glyph = place_glyph(&r, &mut self.pixels, &mut self.packer);
            if glyph.is_none() && self.packer.full {
                // Out of room: start over; text on screen bakes what it needs again
                self.evict();
                glyph = place_glyph(&r, &mut self.pixels, &mut self.packer);
            }
            if let Some(g) = glyph {
                self.faces[id].glyphs.insert(c, g);
            }
        }
        self.generation += 1;
    }
}

impl Atlas {
    /// Forget every face's glyphs and clear the atlas (all but the solid block).
    fn evict(&mut self) {
        for face in self.faces.iter_mut() {
            face.glyphs.clear();
            face.missing.clear();
        }
        self.pixels[self.shared_from * ATLAS_SIZE * CHANNELS..].fill(0);
        self.packer = Packer {
            x: 0,
            y: self.shared_from,
            row_height: 0,
            full: false,
        };
        self.generation += 1;
    }
}

static ATLAS: OnceLock<std::sync::RwLock<Atlas>> = OnceLock::new();

/// Start building the atlas on a background thread (idempotent). Also starts
/// the font scan, so installed fonts are known by the time text needs them.
pub fn build_in_background() {
    static STARTED: OnceLock<()> = OnceLock::new();
    STARTED.get_or_init(|| {
        super::fonts::scan_in_background();
        let spawned = std::thread::Builder::new()
            .name("sdf-atlas".into())
            .spawn(|| {
                let start = std::time::Instant::now();
                drop(get_or_build());
                log::info!(
                    "SDF text atlas built in {:.0} ms",
                    start.elapsed().as_secs_f64() * 1000.0
                );
            });
        if let Err(e) = spawned {
            log::error!("Could not start SDF atlas thread: {e}");
        }
    });
}

/// The atlas, once the background build has finished.
pub fn get() -> Option<std::sync::RwLockReadGuard<'static, Atlas>> {
    ATLAS
        .get()
        .map(|a| a.read().unwrap_or_else(|e| e.into_inner()))
}

/// The atlas, building it on this thread if needed (blocks well under a second).
/// Text editing uses this so caret positions are available without the GPU;
/// a concurrent background build is waited for rather than repeated.
pub fn get_or_build() -> std::sync::RwLockReadGuard<'static, Atlas> {
    ATLAS
        .get_or_init(|| std::sync::RwLock::new(build()))
        .read()
        .unwrap_or_else(|e| e.into_inner())
}

/// The atlas for adding faces and glyphs.
pub fn write() -> std::sync::RwLockWriteGuard<'static, Atlas> {
    ATLAS
        .get_or_init(|| std::sync::RwLock::new(build()))
        .write()
        .unwrap_or_else(|e| e.into_inner())
}

/// Simple shelf packer.
#[derive(Clone)]
struct Packer {
    x: usize,
    y: usize,
    row_height: usize,
    /// A glyph didn't fit
    full: bool,
}

impl Packer {
    fn place(&mut self, w: usize, h: usize) -> Option<(usize, usize)> {
        const GAP: usize = 1;
        if self.x + w + GAP > ATLAS_SIZE {
            self.x = 0;
            self.y += self.row_height + GAP;
            self.row_height = 0;
        }
        if self.y + h > ATLAS_SIZE {
            self.full = true;
            return None;
        }
        let at = (self.x, self.y);
        self.x += w + GAP;
        self.row_height = self.row_height.max(h);
        Some(at)
    }
}

fn build() -> Atlas {
    let mut pixels = vec![0u8; ATLAS_SIZE * ATLAS_SIZE * CHANNELS];
    let mut packer = Packer {
        x: 0,
        y: 0,
        row_height: 0,
        full: false,
    };

    // Solid block for underlines and fills
    let (sx, sy) = packer
        .place(4, 4)
        .expect("atlas has room for the solid block");
    for y in sy..sy + 4 {
        pixels[(y * ATLAS_SIZE + sx) * CHANNELS..(y * ATLAS_SIZE + sx + 4) * CHANNELS].fill(255);
    }
    let solid_uv = [
        (sx as f32 + 2.0) / ATLAS_SIZE as f32,
        (sy as f32 + 2.0) / ATLAS_SIZE as f32,
    ];

    // Glyphs are baked as text needs them (bundled faces too), below the solid block
    let regular = face_from(
        std::sync::Arc::new(include_bytes!("../../assets/fonts/selawk.ttf").to_vec()),
        0,
        false,
        "bundled:selawk.ttf",
    )
    .expect("bundled font parses");
    let bold = face_from(
        std::sync::Arc::new(include_bytes!("../../assets/fonts/selawkb.ttf").to_vec()),
        0,
        false,
        "bundled:selawkb.ttf",
    )
    .expect("bundled font parses");
    let shared_from = packer.y + packer.row_height + 1;
    let packer = Packer {
        x: 0,
        y: shared_from,
        row_height: 0,
        full: false,
    };
    Atlas {
        faces: vec![regular, bold],
        keys: HashMap::new(),
        pixels,
        solid_uv,
        packer,
        shared_from,
        generation: 1,
    }
}

/// A face from font data, with no glyphs baked yet.
fn face_from(
    data: std::sync::Arc<Vec<u8>>,
    index: u32,
    italic: bool,
    source: &str,
) -> Option<Face> {
    let settings = fontdue::FontSettings {
        collection_index: index,
        ..Default::default()
    };
    let font = fontdue::Font::from_bytes(data.as_slice(), settings).ok()?;
    let metrics = font.horizontal_line_metrics(EM_PX)?;
    Some(Face {
        ascent: metrics.ascent / EM_PX,
        descent: -metrics.descent / EM_PX,
        line_gap: metrics.line_gap / EM_PX,
        font,
        glyphs: HashMap::new(),
        missing: Default::default(),
        italic,
        cache: GlyphCache::open(&format!("{source}#{index}")),
        data,
        index,
        path: None,
    })
}

/// A glyph's MTSDF before it has a place in the atlas.
struct Rendered {
    /// RGBA8 texels, `w * h`, top row first
    texels: Vec<u8>,
    w: usize,
    h: usize,
    /// Quad in em units (see [`Glyph::plane`])
    plane: [f32; 4],
    advance: f32,
}

/// Render `c`'s MTSDF (`None`: the face has no such character; a blank
/// rendering for spaces). Independent of the atlas, so glyphs can be rendered
/// on several threads.
fn render_glyph(face: &ttf_parser::Face, c: char) -> Option<Rendered> {
    let id = face.glyph_index(c)?;
    let units_per_em = face.units_per_em() as f64;
    let advance = face.glyph_hor_advance(id).unwrap_or(0) as f32 / units_per_em as f32;
    let blank = Rendered {
        texels: Vec::new(),
        w: 0,
        h: 0,
        plane: [0.0; 4],
        advance,
    };

    let (Some(bbox), Some(mut shape)) = (
        face.glyph_bounding_box(id),
        fdsm_ttf_parser::load_shape_from_face(face, id),
    ) else {
        return Some(blank);
    };
    if bbox.width() <= 0 || bbox.height() <= 0 {
        return Some(blank);
    }

    // Font units -> atlas texels, y up (rows are flipped below), with
    // SPREAD + 1 texels of padding around the outline
    let scale = EM_PX as f64 / units_per_em;
    let pad = SPREAD as f64 + 1.0;
    let w = ((bbox.width() as f64) * scale + 2.0 * pad).ceil() as u32;
    let h = ((bbox.height() as f64) * scale + 2.0 * pad).ceil() as u32;
    let to_texels = nalgebra::convert::<_, nalgebra::Affine2<f64>>(nalgebra::Similarity2::new(
        nalgebra::Vector2::new(
            pad - bbox.x_min as f64 * scale,
            pad - bbox.y_min as f64 * scale,
        ),
        0.0,
        scale,
    ));
    shape.transform(&to_texels);

    // Colour edges so corners keep distinct channels, then generate, correct
    // interpolation artifacts (needs f32, before sign correction) and fix signs
    // by scanline fill
    let range = 2.0 * SPREAD as f64;
    let colored = Shape::edge_coloring_simple(shape, 0.03, 0x5eed_u64.wrapping_mul(c as u64 + 1));
    let prepared = colored.prepare();
    let mut field = image::Rgba32FImage::new(w, h);
    fdsm::generate::generate_mtsdf(&prepared, range, &mut field);
    fdsm::correct_error::correct_error_mtsdf(
        &mut field,
        &colored,
        &prepared,
        range,
        &Default::default(),
    );
    fdsm::render::correct_sign_mtsdf(&mut field, &prepared, FillRule::Nonzero);

    // Field row 0 is the bottom of the glyph; atlas rows run top-down
    let mut texels = vec![0u8; (w * h) as usize * CHANNELS];
    for y in 0..h {
        let row = (h - 1 - y) as usize;
        for x in 0..w {
            let at = (row * w as usize + x as usize) * CHANNELS;
            for (out, v) in texels[at..at + CHANNELS]
                .iter_mut()
                .zip(field.get_pixel(x, y).0)
            {
                *out = (v.clamp(0.0, 1.0) * 255.0).round() as u8;
            }
        }
    }

    // Plane: em units relative to the pen on the baseline, y down. The field's
    // top edge sits `h - pad` texels above the outline's bottom (y_min).
    let em = EM_PX as f64;
    let x0 = bbox.x_min as f64 / units_per_em - pad / em;
    let y0 = -(bbox.y_min as f64 / units_per_em + (h as f64 - pad) / em);
    let plane = [
        x0 as f32,
        y0 as f32,
        (x0 + w as f64 / em) as f32,
        (y0 + h as f64 / em) as f32,
    ];
    Some(Rendered {
        texels,
        w: w as usize,
        h: h as usize,
        plane,
        advance,
    })
}

/// Copy a rendered glyph into the atlas; `None` when it doesn't fit.
fn place_glyph(r: &Rendered, pixels: &mut [u8], packer: &mut Packer) -> Option<Glyph> {
    if r.w == 0 {
        return Some(Glyph {
            uv: [0.0; 4],
            plane: r.plane,
            advance: r.advance,
        });
    }
    let (ax, ay) = packer.place(r.w, r.h)?;
    for row in 0..r.h {
        let src = &r.texels[row * r.w * CHANNELS..(row + 1) * r.w * CHANNELS];
        let at = ((ay + row) * ATLAS_SIZE + ax) * CHANNELS;
        pixels[at..at + r.w * CHANNELS].copy_from_slice(src);
    }
    let atlas = ATLAS_SIZE as f32;
    let uv = [
        ax as f32 / atlas,
        ay as f32 / atlas,
        (ax + r.w) as f32 / atlas,
        (ay + r.h) as f32 / atlas,
    ];
    Some(Glyph {
        uv,
        plane: r.plane,
        advance: r.advance,
    })
}

/// Rendered glyphs of one face kept on disk between sessions, so a font used
/// before loads without rendering. One file per face (named by a hash of the
/// font's identity and the atlas settings); records are appended as glyphs
/// are rendered.
struct GlyphCache {
    path: Option<std::path::PathBuf>,
    glyphs: HashMap<char, Option<Rendered>>,
}

/// Bump when the rendering or record format changes.
const CACHE_VERSION: u32 = 1;

impl GlyphCache {
    fn open(source: &str) -> Self {
        // FNV-1a: stable across runs and Rust versions
        let id = format!("{source}|{EM_PX}|{SPREAD}|{CACHE_VERSION}");
        let hash = id.bytes().fold(0xcbf2_9ce4_8422_2325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3)
        });
        let path =
            super::fonts::cache_dir().map(|d| d.join("glyphs").join(format!("{hash:016x}.bin")));
        let glyphs = path
            .as_ref()
            .and_then(|p| std::fs::read(p).ok())
            .map(|b| Self::parse(&b))
            .unwrap_or_default();
        GlyphCache { path, glyphs }
    }

    /// Records: char u32, w u16, h u16 (0xFFFF: no such glyph), plane 4 x f32,
    /// advance f32, then w * h RGBA texels. A truncated last record is ignored.
    fn parse(mut b: &[u8]) -> HashMap<char, Option<Rendered>> {
        let mut out = HashMap::new();
        let u32_at = |b: &[u8], o: usize| u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]);
        while b.len() >= 28 {
            let c = char::from_u32(u32_at(b, 0));
            let (w, h) = (
                u16::from_le_bytes([b[4], b[5]]),
                u16::from_le_bytes([b[6], b[7]]),
            );
            let f = |i: usize| f32::from_bits(u32_at(b, 8 + 4 * i));
            let (plane, advance) = ([f(0), f(1), f(2), f(3)], f(4));
            let missing = w == 0xFFFF;
            let len = if missing {
                0
            } else {
                usize::from(w) * usize::from(h) * CHANNELS
            };
            if b.len() < 28 + len {
                break;
            }
            if let Some(c) = c {
                let r = (!missing).then(|| Rendered {
                    texels: b[28..28 + len].to_vec(),
                    w: usize::from(w),
                    h: usize::from(h),
                    plane,
                    advance,
                });
                out.insert(c, r);
            }
            b = &b[28 + len..];
        }
        out
    }

    fn get(&self, c: char) -> Option<Option<Rendered>> {
        self.glyphs.get(&c).map(|r| {
            r.as_ref().map(|r| Rendered {
                texels: r.texels.clone(),
                w: r.w,
                h: r.h,
                plane: r.plane,
                advance: r.advance,
            })
        })
    }

    fn append(&mut self, rendered: &[(char, Option<Rendered>)]) {
        let Some(path) = &self.path else { return };
        let mut bytes = Vec::new();
        for (c, r) in rendered {
            bytes.extend_from_slice(&(*c as u32).to_le_bytes());
            let (w, h, plane, advance) = match r {
                Some(r) => (r.w as u16, r.h as u16, r.plane, r.advance),
                None => (0xFFFF, 0xFFFF, [0.0; 4], 0.0),
            };
            bytes.extend_from_slice(&w.to_le_bytes());
            bytes.extend_from_slice(&h.to_le_bytes());
            for v in plane.iter().chain(std::iter::once(&advance)) {
                bytes.extend_from_slice(&v.to_bits().to_le_bytes());
            }
            if let Some(r) = r {
                bytes.extend_from_slice(&r.texels);
            }
            self.glyphs.insert(
                *c,
                r.as_ref().map(|r| Rendered {
                    texels: r.texels.clone(),
                    w: r.w,
                    h: r.h,
                    plane: r.plane,
                    advance: r.advance,
                }),
            );
        }
        use std::io::Write;
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            let _ = f.write_all(&bytes);
        }
    }
}

/// Render `chars` of a font on all cores.
fn render_many(data: &[u8], index: u32, chars: &[char]) -> Vec<(char, Option<Rendered>)> {
    let threads = std::thread::available_parallelism()
        .map_or(4, |n| n.get())
        .clamp(1, 16);
    let chunk = chars.len().div_ceil(threads).max(1);
    std::thread::scope(|scope| {
        let workers: Vec<_> = chars
            .chunks(chunk)
            .map(|part| {
                scope.spawn(move || {
                    let face = ttf_parser::Face::parse(data, index).ok();
                    part.iter()
                        .map(|&c| (c, face.as_ref().and_then(|f| render_glyph(f, c))))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .flat_map(|w| w.join().unwrap_or_default())
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyphs_sit_on_the_baseline() {
        let mut atlas = build();
        atlas.bake_many(REGULAR, &['H', 'p']);
        let pad = (SPREAD + 1.0) / EM_PX;
        // 'H' stands on the baseline and reaches cap height; 'p' descends below it
        let h = atlas.face(REGULAR).glyph('H').copied().unwrap();
        assert!(
            (h.plane[3] - pad).abs() < 1.5 / EM_PX,
            "H bottom at baseline + pad: {:?}",
            h.plane
        );
        assert!(
            h.plane[1] + pad < -0.5,
            "H top well above the baseline: {:?}",
            h.plane
        );
        let p = atlas.face(REGULAR).glyph('p').copied().unwrap();
        assert!(
            p.plane[3] - pad > 0.1,
            "p descends below the baseline: {:?}",
            p.plane
        );
        // Plane size matches the atlas cell
        let cell_w = (h.uv[2] - h.uv[0]) * ATLAS_SIZE as f32 / EM_PX;
        assert!((h.plane[2] - h.plane[0] - cell_w).abs() < 1e-4);
    }

    #[test]
    fn installed_faces_load_when_text_uses_them() {
        let registry = super::super::fonts::registry();
        let latin = [
            "Arial",
            "Helvetica",
            "Times New Roman",
            "Georgia",
            "Verdana",
            "DejaVu Sans",
            "Liberation Sans",
        ];
        let Some(family) = latin
            .iter()
            .find(|f| registry.has(f))
            .map(|f| f.to_string())
        else {
            return;
        };
        let mut atlas = build();
        let mut style = repub_model::styles::CharacterStyle::default();
        style.font.family = family.clone();
        assert!(!atlas.knows(&style));
        atlas.load(&style);
        let (id, _) = atlas.resolve(&style);
        assert!(id > BOLD, "{family} gets its own face");
        atlas.bake_many(id, &['A', 'B', 'C']);
        assert!(
            atlas.face(id).glyph('A').is_some_and(|g| g.has_quad()),
            "baked from the installed face"
        );
        let before = atlas.generation;
        atlas.bake(id, 'é');
        assert!(atlas.has_glyph(id, 'é'));
        assert!(atlas.generation >= before);
        // A family that isn't installed falls back to Selawik
        style.font.family = "No Such Font 123".into();
        atlas.load(&style);
        assert_eq!(atlas.resolve(&style).0, REGULAR);
    }

    #[test]
    fn a_full_atlas_makes_room() {
        let registry = super::super::fonts::registry();
        let latin = [
            "Arial",
            "Helvetica",
            "Times New Roman",
            "Georgia",
            "Verdana",
            "DejaVu Sans",
            "Liberation Sans",
        ];
        let Some(family) = latin
            .iter()
            .find(|f| registry.has(f))
            .map(|f| f.to_string())
        else {
            return;
        };
        let mut atlas = build();
        let mut style = repub_model::styles::CharacterStyle::default();
        style.font.family = family;
        atlas.load(&style);
        let (id, _) = atlas.resolve(&style);
        atlas.bake_many(id, &['B']);
        // Fill the atlas, then ask for one more glyph
        atlas.packer.y = ATLAS_SIZE - 1;
        atlas.bake(id, 'Ä');
        assert!(
            atlas.has_glyph(id, 'Ä') && atlas.face(id).glyph('Ä').is_some_and(|g| g.has_quad())
        );
        assert!(
            !atlas.has_glyph(id, 'B'),
            "older glyphs evicted, baked again when needed"
        );
    }

    #[test]
    fn glyph_outline_sits_at_half() {
        let mut atlas = build();
        atlas.bake_many(REGULAR, &['I', ' ']);
        let g = atlas
            .face(REGULAR)
            .glyph('I')
            .copied()
            .expect("I is in the atlas");
        assert!(g.has_quad());
        assert!(atlas
            .face(REGULAR)
            .glyph(' ')
            .is_some_and(|g| !g.has_quad()));
        // The middle of the 'I' stem is inside, the cell corner is outside, in
        // both the multi-channel median and the true distance (alpha)
        let texel = |u: f32, v: f32| {
            let at = ((v * ATLAS_SIZE as f32) as usize * ATLAS_SIZE
                + (u * ATLAS_SIZE as f32) as usize)
                * CHANNELS;
            let [r, g, b, a] = [
                atlas.pixels[at],
                atlas.pixels[at + 1],
                atlas.pixels[at + 2],
                atlas.pixels[at + 3],
            ];
            (r.max(g).min(r.min(g).max(b)), a)
        };
        let (cu, cv) = ((g.uv[0] + g.uv[2]) / 2.0, (g.uv[1] + g.uv[3]) / 2.0);
        let (median, alpha) = texel(cu, cv);
        assert!(
            median > 128 && alpha > 128,
            "stem centre should be inside: {median} {alpha}"
        );
        let (median, alpha) = texel(g.uv[0], g.uv[1]);
        assert!(
            median < 64 && alpha < 64,
            "corner should be well outside: {median} {alpha}"
        );
    }
}

#[cfg(test)]
mod timing {
    #[test]
    #[ignore]
    fn font_timings() {
        let t = std::time::Instant::now();
        let r = super::super::fonts::registry();
        eprintln!(
            "scan: {:?}, {} families",
            t.elapsed(),
            r.family_names().len()
        );
        let t = std::time::Instant::now();
        let mut atlas = super::build();
        eprintln!("bundled build: {:?}", t.elapsed());
        atlas.bake_many(
            super::REGULAR,
            &("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
                .chars()
                .collect::<Vec<_>>()),
        );
        eprintln!("bake 62 Selawik glyphs: {:?}", t.elapsed());
        for family in ["Arial", "Georgia", "Times New Roman"] {
            let mut style = repub_model::styles::CharacterStyle::default();
            style.font.family = family.into();
            let t = std::time::Instant::now();
            atlas.load(&style);
            let id = atlas.resolve(&style).0;
            atlas.bake_many(
                id,
                &("The quick brown fox jumps over the lazy dog 0123456789"
                    .chars()
                    .collect::<Vec<_>>()),
            );
            eprintln!("load {family} + bake a sentence: {:?}", t.elapsed());
        }
    }
}
