//! Unicode-compliant line breaking.

use unicode_linebreak::{linebreaks, BreakOpportunity as UnicodeBreakOpportunity};
use unicode_segmentation::UnicodeSegmentation;

/// A break opportunity in text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BreakOpportunity {
    /// No break allowed
    None,
    /// Break allowed (soft break)
    Allowed,
    /// Break required (hard break, e.g., newline)
    Required,
    /// Force break to next column
    ColumnBreak,
}

/// A segment of text with its break opportunity.
#[derive(Debug, Clone)]
pub struct TextSegment {
    /// The text content
    pub text: String,
    /// Byte offset in original text
    pub offset: usize,
    /// Length in bytes
    pub len: usize,
    /// Break opportunity after this segment
    pub break_after: BreakOpportunity,
}

/// Unicode code point for column break (private use area).
/// This can be inserted into text to force a column break.
pub const COLUMN_BREAK_CHAR: char = '\u{E000}';

/// Line breaker using Unicode Line Breaking Algorithm (UAX #14).
pub struct LineBreaker {
    /// Minimum characters before hyphenation
    min_before: usize,
    /// Minimum characters after hyphenation
    min_after: usize,
}

impl Default for LineBreaker {
    fn default() -> Self {
        Self {
            min_before: 2,
            min_after: 3,
        }
    }
}

impl LineBreaker {
    /// Create a new line breaker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set hyphenation parameters.
    pub fn with_hyphenation(mut self, min_before: usize, min_after: usize) -> Self {
        self.min_before = min_before;
        self.min_after = min_after;
        self
    }

    /// Find all break opportunities in text.
    pub fn find_breaks(&self, text: &str) -> Vec<TextSegment> {
        if text.is_empty() {
            return vec![];
        }

        let mut segments = Vec::new();
        let mut last_break = 0;

        for (offset, break_type) in linebreaks(text) {
            if offset > last_break {
                let segment_text = &text[last_break..offset];
                segments.push(TextSegment {
                    text: segment_text.to_string(),
                    offset: last_break,
                    len: offset - last_break,
                    break_after: match break_type {
                        UnicodeBreakOpportunity::Mandatory => BreakOpportunity::Required,
                        UnicodeBreakOpportunity::Allowed => BreakOpportunity::Allowed,
                    },
                });
            }
            last_break = offset;
        }

        // Handle remaining text
        if last_break < text.len() {
            segments.push(TextSegment {
                text: text[last_break..].to_string(),
                offset: last_break,
                len: text.len() - last_break,
                break_after: BreakOpportunity::None,
            });
        }

        segments
    }

    /// Find break opportunities by word.
    pub fn word_segments(&self, text: &str) -> Vec<TextSegment> {
        let mut segments = Vec::new();
        let mut offset = 0;

        for word in text.split_word_bounds() {
            let len = word.len();
            let break_after = if word.contains(COLUMN_BREAK_CHAR) {
                BreakOpportunity::ColumnBreak
            } else if word.chars().all(char::is_whitespace) {
                if word.contains('\n') {
                    BreakOpportunity::Required
                } else {
                    BreakOpportunity::Allowed
                }
            } else {
                BreakOpportunity::None
            };

            segments.push(TextSegment {
                text: word.to_string(),
                offset,
                len,
                break_after,
            });

            offset += len;
        }

        // Mark the last word as allowing a break (end of text)
        if let Some(last) = segments.last_mut() {
            if last.break_after == BreakOpportunity::None {
                last.break_after = BreakOpportunity::Allowed;
            }
        }

        segments
    }

    /// Find the best break point to fit text in a given width.
    pub fn find_break_for_width<F>(&self, text: &str, max_width: f64, measure: F) -> Option<usize>
    where
        F: Fn(&str) -> f64,
    {
        let segments = self.word_segments(text);
        let mut current_width = 0.0;
        let mut last_break = 0;
        let mut last_break_offset = 0;

        for segment in &segments {
            let segment_width = measure(&segment.text);

            if current_width + segment_width > max_width && last_break > 0 {
                // We've exceeded the width, return the last break point
                return Some(last_break_offset);
            }

            current_width += segment_width;

            if segment.break_after != BreakOpportunity::None {
                last_break = segment.offset + segment.len;
                last_break_offset = last_break;

                if segment.break_after == BreakOpportunity::Required {
                    return Some(last_break_offset);
                }
            }
        }

        // Check if entire text fits
        if current_width <= max_width {
            return None; // No break needed
        }

        // Return last break point, or start of text if no break found
        if last_break_offset > 0 {
            Some(last_break_offset)
        } else {
            // Emergency break: find any character that fits
            let mut width = 0.0;
            for (i, c) in text.char_indices() {
                let char_width = measure(&text[i..i + c.len_utf8()]);
                if width + char_width > max_width {
                    return if i > 0 { Some(i) } else { Some(c.len_utf8()) };
                }
                width += char_width;
            }
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_breaks() {
        let breaker = LineBreaker::new();
        let text = "Hello, world! This is a test.";
        let segments = breaker.find_breaks(text);

        assert!(!segments.is_empty());
        let total_len: usize = segments.iter().map(|s| s.len).sum();
        assert_eq!(total_len, text.len());
    }

    #[test]
    fn test_word_segments() {
        let breaker = LineBreaker::new();
        let text = "Hello world";
        let segments = breaker.word_segments(text);

        assert_eq!(segments.len(), 3); // "Hello", " ", "world"
        assert_eq!(segments[0].text, "Hello");
        assert_eq!(segments[1].text, " ");
        assert_eq!(segments[2].text, "world");
    }

    #[test]
    fn test_find_break_for_width() {
        let breaker = LineBreaker::new();
        let text = "Hello world test";

        let measure = |s: &str| s.chars().count() as f64;

        let break_point = breaker.find_break_for_width(text, 12.0, measure);
        assert!(break_point.is_some());
        let bp = break_point.unwrap();
        assert!(bp <= 12);
    }

    #[test]
    fn test_hard_break() {
        let breaker = LineBreaker::new();
        let text = "Line 1\nLine 2";
        let segments = breaker.word_segments(text);

        let newline_seg = segments.iter().find(|s| s.text.contains('\n'));
        assert!(newline_seg.is_some());
        assert_eq!(newline_seg.unwrap().break_after, BreakOpportunity::Required);
    }

    #[test]
    fn test_column_break() {
        let breaker = LineBreaker::new();
        let text = format!("Column 1{}Column 2", COLUMN_BREAK_CHAR);
        let segments = breaker.word_segments(&text);

        let col_break_seg = segments.iter().find(|s| s.text.contains(COLUMN_BREAK_CHAR));
        assert!(col_break_seg.is_some());
        assert_eq!(
            col_break_seg.unwrap().break_after,
            BreakOpportunity::ColumnBreak
        );
    }
}
