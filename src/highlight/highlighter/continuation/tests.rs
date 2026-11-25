//! Purpose: Unit tests for dotenv `advance_only` unquoted fast path.

use std::path::Path;

use super::Highlighter;

#[test]
fn dotenv_advance_only_skips_unquoted_lines_and_tracks_quoted_lines() {
    let mut fast = Highlighter::new(Path::new(".env"));
    let mut full = Highlighter::new(Path::new(".env"));
    for line in ["PLAIN=value", "QUOTED=\"open", "still_inside", "done\""] {
        fast.advance_only(line);
        drop(full.line(line));
        assert_eq!(fast.continuation(), full.continuation());
    }
}
