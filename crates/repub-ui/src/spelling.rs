//! Lightweight spelling checker backed by the system word list.
//!
//! macOS and most Unix systems ship `/usr/share/dict/words`. When it is not
//! available a small built-in list is used, so the feature still works.

use std::collections::HashSet;
use std::sync::OnceLock;

use repub_model::ObjectId;
use repub_model::shape::ShapeKind;

use crate::state::AppState;
use crate::text_edit::{self, TextPos};

/// A misspelled word and its location in the document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Misspelled {
    pub page: usize,
    pub object: ObjectId,
    pub para: usize,
    pub offset: usize,
    pub len: usize,
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
    pub fn new() -> Self {
        Self::default()
    }

    /// Scan the whole document.
    pub fn scan(&mut self, state: &AppState) {
        self.errors.clear();
        self.current = 0;
        let Some(doc) = state.document.as_ref() else { return };
        for (page_idx, page) in doc.pages().iter().enumerate() {
            for obj in page.layers().iter().flat_map(|l| l.shapes()) {
                let ShapeKind::TextFrame(tf) = &obj.kind else { continue };
                for (para_idx, para) in tf.content.paragraphs.iter().enumerate() {
                    let chars = text_edit::para_chars(para);
                    for (start, len) in word_ranges(&chars) {
                        let word: String = chars[start..start + len].iter().collect();
                        if self.is_misspelled(&word) {
                            self.errors.push(Misspelled {
                                page: page_idx,
                                object: obj.common.id,
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

    /// Replace the current word. Returns false when nothing could be changed.
    pub fn change_current(&mut self, state: &mut AppState, replacement: &str) -> bool {
        let Some(err) = self.current_error().cloned() else { return false };
        if !replace_word(state, &err, replacement) {
            return false;
        }
        self.errors.remove(self.current);
        if self.current >= self.errors.len() {
            self.current = 0;
        }
        true
    }

    /// Replace every occurrence of the current word.
    pub fn change_all_current(&mut self, state: &mut AppState, replacement: &str) -> bool {
        let Some(err) = self.current_error().cloned() else { return false };
        let key = err.key();
        let mut targets: Vec<Misspelled> = self.errors.iter().filter(|e| e.key() == key).cloned().collect();
        // Reverse document order so earlier offsets stay valid per object.
        targets.sort_by_key(|m| std::cmp::Reverse((m.page, m.para, m.offset)));
        let mut changed = false;
        for target in &targets {
            changed |= replace_word(state, target, replacement);
        }
        self.errors.retain(|e| e.key() != key);
        self.current = 0;
        changed
    }

    /// Suggestions for the current word, best first.
    pub fn suggestions(&self) -> Vec<String> {
        self.current_error().map(|e| suggest(&e.word)).unwrap_or_default()
    }

    /// Select the current error and start text editing over it.
    pub fn select_current(&mut self, state: &mut AppState, now: f64) {
        let Some(err) = self.current_error().cloned() else { return };
        state.current_page = err.page;
        state.selection = vec![err.object];
        text_edit::begin(state, err.object, Some(TextPos::new(err.para, err.offset)), now);
        if let Some(edit) = state.text_edit.as_mut() {
            edit.anchor = TextPos::new(err.para, err.offset + err.len);
        }
    }
}

fn replace_word(state: &mut AppState, err: &Misspelled, replacement: &str) -> bool {
    let Some(page) = state.document.as_mut().and_then(|d| d.pages_mut().get_mut(err.page)) else {
        return false;
    };
    let Some(shape) = page.get_shape_mut(err.object) else { return false };
    let ShapeKind::TextFrame(tf) = &mut shape.kind else { return false };
    if err.para >= tf.content.paragraphs.len() {
        return false;
    }
    let mut content = std::mem::replace(&mut tf.content, repub_model::text::StyledText::new());
    let start = TextPos::new(err.para, err.offset);
    let end = TextPos::new(err.para, err.offset + err.len);
    text_edit::delete_range(&mut content, start, end);
    text_edit::insert_text(&mut content, start, replacement);
    tf.content = content;
    state.mark_dirty();
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
