//! Purpose: Cached matching bracket and XML/HTML tag coordinates around the editor caret.

/// Document line and character column of one matched `()`, `[]`, or `{}` delimiter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BracketPos {
    pub(crate) line: usize,
    pub(crate) column: usize,
}

/// Character-column range `[start_col, end_col)` of an opening or closing XML/HTML tag on `line`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagSpan {
    pub(crate) line: usize,
    pub(crate) start_col: usize,
    pub(crate) end_col: usize,
}

impl TagSpan {
    #[must_use]
    pub(crate) const fn new(line: usize, start_col: usize, end_col: usize) -> Self {
        Self {
            line,
            start_col,
            end_col,
        }
    }

    #[must_use]
    pub(crate) const fn contains(self, line: usize, col: usize) -> bool {
        self.line == line && col >= self.start_col && col < self.end_col
    }

    #[must_use]
    pub(crate) fn matches_cursor(self, lines: &[String], line: usize, col: usize) -> bool {
        if self.contains(line, col) {
            return true;
        }
        if self.line == line && col == self.end_col {
            let next_ch = lines.get(line).and_then(|s| s.chars().nth(col));
            return next_ch.is_none_or(char::is_whitespace);
        }
        false
    }
}

/// Revision and caret position for which [`CachedPairs`] was computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairCacheKey {
    pub(crate) revision: u64,
    pub(crate) cursor_line: usize,
    pub(crate) cursor_col: usize,
}

/// Cached delimiter and XML/HTML tag pairs at [`PairCacheKey`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedPairs {
    pub(crate) key: PairCacheKey,
    pub(crate) brackets: Option<[BracketPos; 2]>,
    pub(crate) xml_tags: Option<[TagSpan; 2]>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(src: &[&str]) -> Vec<String> {
        src.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn tag_span_new_and_contains_cover_boundary_conditions() {
        let span = TagSpan::new(2, 5, 10);
        assert!(span.contains(2, 5), "start is inclusive");
        assert!(span.contains(2, 9), "last col is inclusive");
        assert!(!span.contains(2, 10), "end col is exclusive");
        assert!(!span.contains(1, 7), "wrong line");
    }

    #[test]
    fn matches_cursor_is_true_when_cursor_is_inside_or_just_past_whitespace_end() {
        let src = lines(&["<div> text"]);
        let span = TagSpan::new(0, 0, 5); // `<div>` columns 0..5
        // Inside the tag.
        assert!(span.matches_cursor(&src, 0, 3));
        // At end_col with a space following — trailing-whitespace rule.
        assert!(span.matches_cursor(&src, 0, 5));
        // At end_col with a non-whitespace following — not a match.
        let src2 = lines(&["<div>x"]);
        assert!(!span.matches_cursor(&src2, 0, 5));
    }

    #[test]
    fn matches_cursor_false_for_unrelated_position() {
        let src = lines(&["<b>hello"]);
        let span = TagSpan::new(0, 0, 3);
        assert!(!span.matches_cursor(&src, 0, 7));
        assert!(!span.matches_cursor(&src, 1, 0));
    }
}
