//! Purpose: Unit test for `HighlightCache::runs` fallback when cached spans exceed a line's bounds.

use super::*;

#[test]
fn runs_falls_back_to_normal_when_cached_spans_are_out_of_bounds() {
    let lines = vec!["let answer = 42;".to_owned()];
    let mut cache = HighlightCache::default();
    cache.prepare(Path::new("bounds.rs"), &lines, 1);
    assert_eq!(cache.runs(0, "x"), vec![("x", Token::Normal)]);
}
