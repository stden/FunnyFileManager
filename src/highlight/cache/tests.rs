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

#[test]
fn a_cloned_cache_starts_empty_and_leaves_the_original_intact() {
    let root = tempfile::TempDir::new().expect("directory");
    let rust = root.path().join("source.rs");
    let lines = vec!["let answer = 42;".to_owned()];
    let mut cache = HighlightCache::default();
    cache.prepare(&rust, &lines, 1);

    let copy = cache.clone();

    assert_eq!(
        copy.runs(0, &lines[0]),
        vec![(lines[0].as_str(), Token::Normal)],
        "the copy holds no spans until it is prepared itself"
    );
    assert!(cache.runs(0, &lines[0]).contains(&("let", Token::Keyword)));
}

#[test]
fn revision_based_prepare_reuses_spans_until_revision_or_path_changes() {
    let root = tempfile::TempDir::new().expect("directory");
    let rust = root.path().join("source.rs");
    let mut lines = vec!["/* open".to_owned(), "let answer = 42;".to_owned()];
    let mut cache = HighlightCache::default();
    let before = crate::highlight::FILE_PASSES.get();

    cache.prepare_revision(&rust, &lines, 1, 1);
    cache.prepare_revision(&rust, &lines, 2, 1);
    assert_eq!(crate::highlight::FILE_PASSES.get() - before, 1);
    assert!(cache
        .runs(1, &lines[1])
        .iter()
        .all(|(_, token)| *token == Token::Comment));

    lines[0] = "// closed".to_owned();
    cache.prepare_revision(&rust, &lines, 2, 2);
    assert_eq!(
        crate::highlight::FILE_PASSES.get() - before,
        1,
        "incremental revision update reuses the engine without a full file pass"
    );
    assert!(cache.runs(1, &lines[1]).contains(&("let", Token::Keyword)));
}

#[test]
fn single_line_edit_in_large_file_converges_after_one_line() {
    let root = tempfile::TempDir::new().expect("directory");
    let rust = root.path().join("large.rs");
    let mut lines: Vec<String> = (0..1_000).map(|i| format!("let v{i} = {i};")).collect();
    let mut cache = HighlightCache::default();

    cache.prepare_revision(&rust, &lines, 1_000, 1);
    let before_lines = crate::highlight::LINE_PASSES.get();

    // Edit a single line near the middle of the file; state before and after
    // remains Outside, so only line 500 should be re-tokenized.
    lines[500] = "fn updated() -> u32 { 7 }".to_owned();
    cache.prepare_revision(&rust, &lines, 1_000, 2);

    assert_eq!(
        crate::highlight::LINE_PASSES.get() - before_lines,
        1,
        "only the edited line should be re-tokenized when continuation state converges"
    );
    assert!(cache
        .runs(500, &lines[500])
        .contains(&("fn", Token::Keyword)));
    assert!(cache
        .runs(999, &lines[999])
        .contains(&("let", Token::Keyword)));
    assert_eq!(cache, cache.clone());
}

#[test]
fn cache_reconciles_line_growth_and_scanner_continuations() {
    let root = tempfile::TempDir::new().expect("directory");
    let mut cache = HighlightCache::default();
    let rust = root.path().join("grow.rs");
    let mut lines = vec!["let a = 1;".to_owned(), "let b = 2;".to_owned()];
    cache.prepare_revision(&rust, &lines, 2, 1);
    lines.push("let tail = 1;".to_owned());
    cache.prepare_revision(&rust, &lines, 3, 2);
    assert!(cache.runs(2, &lines[2]).contains(&("let", Token::Keyword)));
    lines.insert(2, "fn offscreen() {}".to_owned());
    cache.prepare_revision(&rust, &lines, 1, 3);
    cache.prepare_revision(&rust, &lines, 4, 3);
    assert!(cache.runs(2, &lines[2]).contains(&("fn", Token::Keyword)));

    for (name, first, second) in [
        ("doc.md", "```", "inside"),
        (".env", "A=\"open", "closed\""),
        ("page.html", "<!-- open", "closed -->"),
    ] {
        let path = root.path().join(name);
        let mut buf = vec![first.to_owned(), second.to_owned()];
        cache.prepare(&path, &buf, 2);
        buf[0].push(' ');
        cache.prepare(&path, &buf, 2);
    }
}

#[test]
fn single_line_insert_and_delete_in_large_file_converge_in_constant_passes() {
    let root = tempfile::TempDir::new().expect("directory");
    let rust = root.path().join("insert_delete.rs");
    let mut lines: Vec<String> = (0..1_000).map(|i| format!("let v{i} = {i};")).collect();
    let mut cache = HighlightCache::default();

    cache.prepare_revision(&rust, &lines, 1_000, 1);
    let before_insert = crate::highlight::LINE_PASSES.get();
    lines.insert(500, "fn inserted() -> u32 { 1 }".to_owned());
    cache.prepare_revision(&rust, &lines, 1_001, 2);
    assert_eq!(
        crate::highlight::LINE_PASSES.get() - before_insert,
        1,
        "inserting one line with unchanged continuation state lexes only the inserted line"
    );
    assert!(cache
        .runs(500, &lines[500])
        .contains(&("fn", Token::Keyword)));
    assert!(cache
        .runs(1_000, &lines[1_000])
        .contains(&("let", Token::Keyword)));

    let before_delete = crate::highlight::LINE_PASSES.get();
    lines.remove(500);
    cache.prepare_revision(&rust, &lines, 1_000, 3);
    assert_eq!(
        crate::highlight::LINE_PASSES.get() - before_delete,
        0,
        "deleting a line whose continuation state matches its predecessor re-lexes zero lines"
    );
    assert!(cache
        .runs(999, &lines[999])
        .contains(&("let", Token::Keyword)));
}
