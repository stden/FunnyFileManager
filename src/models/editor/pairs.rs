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
mod tests;
