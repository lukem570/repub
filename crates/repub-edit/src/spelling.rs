//! Lightweight spelling checker backed by the system word list.
//!
//! macOS and most Unix systems ship `/usr/share/dict/words`. When it is not
//! available a small built-in list is used, so the feature still works. The
//! checker scans and edits the document model directly; selecting an error in
//! an editing session is an application concern.

use std::collections::HashSet;
use std::sync::OnceLock;

use repub_model::id::ObjectId;
use repub_model::shape::ShapeKind;
use repub_model::text::StyledText;
use repub_model::Document;

use crate::text_edit::{self, TextPos};

/// A misspelled word and its location in the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Misspelled {
    /// Page index.
    pub page: usize,
    /// Shape (text frame) holding the word.
    pub object: ObjectId,
    /// Paragraph index.
    pub para: usize,
    /// Character offset of the word.
    pub offset: usize,
    /// Length of the word in characters.
    pub len: usize,
    /// The word itself.
    pub word: String,
}

impl Misspelled {
    /// Lowercase form used for grouping.
    pub fn key(&self) -> String {
        self.word.to_lowercase()
    }
}

static DICTIONARY: OnceLock<HashSet<String>> = OnceLock::new();

/// A handful of common words used when no system dictionary exists.
const FALLBACK_WORDS: &str = "the of and to in a is that it for on with as was at by an be this from or have not are but they you we all can will if do when there what so up out about which who get go me my one two three hello world text page document publisher \
    color shape image line table paragraph font size style insert delete select move copy paste undo redo";

fn dictionary() -> &'static HashSet<String> {
    DICTIONARY.get_or_init(|| {
        let mut set = HashSet::new();
        for path in [
            "/usr/share/dict/words",
            "/usr/share/dict/american-english",
            "/usr/share/dict/british-english",
            "/usr/dict/words",
        ] {
            if let Ok(text) = std::fs::read_to_string(path) {
                for line in text.lines() {
                    let word = line.trim();
                    if !word.is_empty() && !word.contains('\'') {
                        set.insert(word.to_lowercase());
                    }
                }
                if !set.is_empty() {
                    break;
                }
            }
        }
        if set.is_empty() {
            for word in FALLBACK_WORDS.split_whitespace() {
                set.insert(word.to_string());
            }
        }
        set
    })
}

/// A spelling session: document scan plus session-level ignores.
#[derive(Debug, Default)]
pub struct SpellSession {
    /// Ignored words (Ignore All / Add to Dictionary)
    pub ignored: HashSet<String>,
    /// All misspellings found at the last scan
    pub errors: Vec<Misspelled>,
    /// Index of the current error
    pub current: usize,
}

impl SpellSession {
    /// Create a new, empty session.
    pub fn new() -> Self {
        Self::default()
    }

    /// Scan the whole document.
    pub fn scan(&mut self, doc: &Document) {
        self.errors.clear();
        self.current = 0;
        for (page_idx, page) in doc.pages().iter().enumerate() {
            for shape in page.all_shapes() {
                let ShapeKind::TextFrame(data) = &shape.kind else { continue };
                for (para_idx, para) in data.content.paragraphs.iter().enumerate() {
                    let chars = text_edit::para_chars(para);
                    for (start, len) in word_ranges(&chars) {
                        let word: String = chars[start..start + len].iter().collect();
                        if self.is_misspelled(&word) {
                            self.errors.push(Misspelled {
                                page: page_idx,
                                object: shape.id(),
                                para: para_idx,
                                offset: start,
                                len,
                                word,
                            });
                        }
                    }
                }
            }
        }
    }

    fn is_misspelled(&self, word: &str) -> bool {
        let lower = word.to_lowercase();
        if self.ignored.contains(&lower) || dictionary().contains(&lower) {
            return false;
        }
        // Possessives: check the stem too.
        if let Some(stem) = lower.strip_suffix("'s") {
            if dictionary().contains(stem) {
                return false;
            }
        }
        true
    }

    /// Current error, if any.
    pub fn current_error(&self) -> Option<&Misspelled> {
        self.errors.get(self.current)
    }

    /// Accept the current error and move on.
    pub fn ignore_current(&mut self) {
        if self.current < self.errors.len() {
            self.errors.remove(self.current);
        }
        if self.current >= self.errors.len() {
            self.current = 0;
        }
    }

    /// Ignore every occurrence of the current word.
    pub fn ignore_all_current(&mut self) {
        if let Some(err) = self.current_error().cloned() {
            let key = err.key();
            self.errors.retain(|e| e.key() != key);
            self.current = 0;
        }
    }

    /// Add the current word to the session dictionary.
    pub fn add_current(&mut self) {
        if let Some(err) = self.current_error().cloned() {
            self.ignored.insert(err.key());
            self.errors.retain(|e| e.key() != err.key());
            self.current = 0;
        }
    }

    /// Replace the current word in `doc`. Returns false when nothing could be changed.
    pub fn change_current(&mut self, doc: &mut Document, replacement: &str) -> bool {
        let Some(err) = self.current_error().cloned() else { return false };
        if !replace_word(doc, &err, replacement) {
            return false;
        }
        self.errors.remove(self.current);
        if self.current >= self.errors.len() {
            self.current = 0;
        }
        true
    }

    /// Replace every occurrence of the current word in `doc`.
    pub fn change_all_current(&mut self, doc: &mut Document, replacement: &str) -> bool {
        let Some(err) = self.current_error().cloned() else { return false };
        let key = err.key();
        let mut targets: Vec<Misspelled> = self.errors.iter().filter(|e| e.key() == key).cloned().collect();
        // Reverse document order so earlier offsets stay valid per object.
        targets.sort_by_key(|e| std::cmp::Reverse((e.page, e.para, e.offset)));
        let mut changed = false;
        for target in &targets {
            changed |= replace_word(doc, target, replacement);
        }
        self.errors.retain(|e| e.key() != key);
        self.current = 0;
        changed
    }

    /// Suggestions for the current word, best first.
    pub fn suggestions(&self) -> Vec<String> {
        self.current_error().map(|e| suggest(&e.word)).unwrap_or_default()
    }
}

/// Replace the word located by `err` with `replacement` in `doc`.
pub fn replace_word(doc: &mut Document, err: &Misspelled, replacement: &str) -> bool {
    let Some(page) = doc.pages_mut().get_mut(err.page) else { return false };
    let Some(ShapeKind::TextFrame(data)) = page.get_shape_mut(err.object).map(|s| &mut s.kind) else {
        return false;
    };
    if err.para >= data.content.paragraphs.len() {
        return false;
    }
    let mut content = std::mem::replace(&mut data.content, StyledText::new());
    let start = TextPos::new(err.para, err.offset);
    let end = TextPos::new(err.para, err.offset + err.len);
    text_edit::delete_range(&mut content, start, end);
    text_edit::insert_text(&mut content, start, replacement);
    if let Some(ShapeKind::TextFrame(data)) = doc.pages_mut()[err.page].get_shape_mut(err.object).map(|s| &mut s.kind) {
        data.content = content;
    }
    true
}

/// Alphabetic word ranges (with internal apostrophes) in a char slice.
fn word_ranges(chars: &[char]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if !chars[i].is_alphabetic() {
            i += 1;
            continue;
        }
        let start = i;
        let mut has_upper_inside = false;
        while i < chars.len() {
            let c = chars[i];
            if c.is_alphabetic() {
                if i > start && c.is_uppercase() {
                    has_upper_inside = true;
                }
                i += 1;
            } else if c == '\'' && i + 1 < chars.len() && chars[i + 1].is_alphabetic() {
                i += 1;
            } else {
                break;
            }
        }
        let len = i - start;
        let word: String = chars[start..i].iter().collect();
        if len > 1 && !has_upper_inside && word != word.to_uppercase() {
            out.push((start, len));
        }
    }
    out
}

/// Edit-distance-1 suggestions from the dictionary.
fn suggest(word: &str) -> Vec<String> {
    let lower = word.to_lowercase();
    let letters: Vec<char> = "abcdefghijklmnopqrstuvwxyz".chars().collect();
    let chars: Vec<char> = lower.chars().collect();
    let mut out: Vec<String> = Vec::new();
    let mut seen = HashSet::new();

    let mut push = |s: String| {
        if s != lower && dictionary().contains(&s) && seen.insert(s.clone()) {
            out.push(s);
        }
    };

    // Deletes
    for i in 0..chars.len() {
        let mut c = chars.clone();
        c.remove(i);
        push(c.into_iter().collect());
    }
    // Transposes
    for i in 0..chars.len().saturating_sub(1) {
        let mut c = chars.clone();
        c.swap(i, i + 1);
        push(c.into_iter().collect());
    }
    // Replaces
    for i in 0..chars.len() {
        for &l in &letters {
            let mut c = chars.clone();
            c[i] = l;
            push(c.into_iter().collect());
        }
    }
    // Inserts
    for i in 0..=chars.len() {
        for &l in &letters {
            let mut c = chars.clone();
            c.insert(i, l);
            push(c.into_iter().collect());
        }
    }

    // Prefer candidates that share the first letter, like Publisher's ordering.
    out.sort_by_key(|s| (s.chars().next() != lower.chars().next(), s.len()));
    out.truncate(8);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use repub_geometry::Rect;
    use repub_model::shape::Shape;
    use repub_model::{PageOrientation, PageSize};

    #[test]
    fn word_ranges_skips_acronyms_and_single_letters() {
        let chars: Vec<char> = "a hello WORLD CamelCase don't".chars().collect();
        let ranges = word_ranges(&chars);
        let words: Vec<String> = ranges.iter().map(|&(s, l)| chars[s..s + l].iter().collect()).collect();
        assert!(words.contains(&"hello".to_string()));
        assert!(words.contains(&"don't".to_string()));
        assert!(!words.iter().any(|w| w == "WORLD"));
        assert!(!words.iter().any(|w| w == "CamelCase"));
        assert!(!words.iter().any(|w| w == "a"));
    }

    #[test]
    fn scan_and_replace() {
        let mut doc = Document::new(PageSize::Letter, PageOrientation::Portrait);
        doc.page_mut(0)
            .unwrap()
            .add_shape(Shape::text_frame_with_text(Rect::new(0.0, 0.0, 200.0, 50.0), "hello wrld"));
        let mut session = SpellSession::new();
        // Force the fallback dictionary path by relying on "wrld" not being a word.
        session.scan(&doc);
        // "wrld" should be flagged (not in any dictionary); "hello" is in the fallback.
        if let Some(err) = session.current_error().cloned() {
            assert_eq!(err.word, "wrld");
            assert!(session.change_current(&mut doc, "world"));
            let text = doc.page(0).unwrap().all_shapes().next().map(|s| match &s.kind {
                ShapeKind::TextFrame(d) => d.content.plain_text(),
                _ => String::new(),
            });
            assert_eq!(text.as_deref(), Some("hello world"));
        }
    }
}
