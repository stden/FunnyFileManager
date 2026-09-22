//! Cached spans must track edits and language changes, including multiline state.
use super::*;

#[test]
fn edits_invalidate_multiline_state_and_paths_change_the_language() {
    let root = tempfile::TempDir::new().expect("directory");
    let rust = root.path().join("source.rs");
    let mut lines = vec!["/* open".to_owned(), "let answer = 42;".to_owned()];
    let mut cache = HighlightCache::default();
    cache.prepare(&rust, &lines, 1);
    cache.prepare(&rust, &lines, 2);
    assert!(cache
        .runs(1, &lines[1])
        .iter()
        .all(|(_, token)| *token == Token::Comment));
    lines[0] = "// closed on this line".to_owned();
    cache.prepare(&rust, &lines, 2);
    assert!(cache.runs(1, &lines[1]).contains(&("let", Token::Keyword)));
    cache.prepare(&root.path().join("source.unknown"), &lines, 2);
    assert_eq!(
        cache.runs(1, &lines[1]),
        vec![(lines[1].as_str(), Token::Normal)]
    );
}
