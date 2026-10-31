//! Purpose: a known edit line skips hashing the unchanged prefix and still
//! yields the same tokens, including when a block comment continues.

use super::*;

fn rust_lines(count: usize) -> Vec<String> {
    (0..count)
        .map(|index| format!("let v{index} = {index};"))
        .collect()
}

fn runs_of<'a>(cache: &HighlightCache, lines: &'a [String]) -> Vec<Vec<(&'a str, Token)>> {
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| cache.runs(index, line))
        .collect()
}

#[test]
fn a_hinted_edit_matches_a_full_walk_and_skips_the_prefix_hash() {
    let path = Path::new("large.rs");
    let mut lines = rust_lines(1_000);
    let mut hinted = HighlightCache::default();
    let mut plain = HighlightCache::default();
    hinted.prepare_revision(path, &lines, 1_000, 1);
    plain.prepare_revision(path, &lines, 1_000, 1);

    lines[500] = "fn updated() -> u32 { 7 }".to_owned();
    let before_lines = crate::highlight::LINE_PASSES.get();
    let before_hashes = crate::highlight::HASH_PASSES.get();
    hinted.prepare_hinted(path, &lines, 1_000, 2, Some((500, 501)));
    let hinted_lines = crate::highlight::LINE_PASSES.get() - before_lines;
    let hinted_hashes = crate::highlight::HASH_PASSES.get() - before_hashes;

    let before_hashes = crate::highlight::HASH_PASSES.get();
    plain.prepare_revision(path, &lines, 1_000, 2);
    let plain_hashes = crate::highlight::HASH_PASSES.get() - before_hashes;

    assert_eq!(
        hinted_lines, 1,
        "the edit is one line and continuation holds"
    );
    assert!(hinted_hashes < 20, "hinted hashes {hinted_hashes}");
    assert!(plain_hashes > hinted_hashes, "plain hashes {plain_hashes}");
    assert_eq!(runs_of(&hinted, &lines), runs_of(&plain, &lines));
}

#[test]
fn a_hint_does_not_skip_a_block_comment_that_continues() {
    let path = Path::new("comment.rs");
    let mut lines = rust_lines(40);
    let mut cache = HighlightCache::default();
    cache.prepare_revision(path, &lines, 40, 1);
    lines[10] = "/* opened".to_owned();
    let before = crate::highlight::LINE_PASSES.get();
    cache.prepare_hinted(path, &lines, 40, 2, Some((10, 11)));
    assert!(crate::highlight::LINE_PASSES.get() - before > 1);
    assert!(cache
        .runs(11, &lines[11])
        .iter()
        .all(|(_, token)| *token == Token::Comment));
}
