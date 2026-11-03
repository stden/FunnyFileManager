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

#[test]
fn prepare_window_skips_offscreen_prefix_lexing_and_matches_full_walk() {
    let path = Path::new("deep.py");
    let mut lines: Vec<String> = (0..2_000)
        .map(|i| format!("def fn_{i}(x): return x + {i}"))
        .collect();
    lines[3] = "\"\"\"doc start".to_owned();
    lines[6] = "doc end\"\"\"".to_owned();

    let mut windowed = HighlightCache::default();
    let before = crate::highlight::LINE_PASSES.get();
    windowed.prepare_window(path, &lines, 1_960, 2_000, 1, None);
    let passes = crate::highlight::LINE_PASSES.get() - before;
    assert!(passes < 60, "expected < 60 line passes, got {passes}");

    let mut full = HighlightCache::default();
    full.prepare_revision(path, &lines, 2_000, 1);
    for (idx, line) in lines.iter().enumerate().take(2_000).skip(1_960) {
        assert_eq!(windowed.runs(idx, line), full.runs(idx, line));
    }

    // Scrolling back up to the top fills previously fast-forwarded spans lazily.
    windowed.prepare_window(path, &lines, 0, 15, 1, None);
    for (idx, line) in lines.iter().enumerate().take(15) {
        assert_eq!(windowed.runs(idx, line), full.runs(idx, line));
    }

    // Editing an off-screen line while scrolled down propagates continuation without lexing the prefix.
    lines[500] = "/*".to_owned();
    let rs_path = Path::new("deep.rs");
    let mut rs_lines = rust_lines(600);
    windowed.prepare_window(rs_path, &rs_lines, 560, 600, 1, None);
    rs_lines[10] = "/* unclosed".to_owned();
    windowed.prepare_window(rs_path, &rs_lines, 560, 600, 2, Some((10, 11)));
    assert!(windowed
        .runs(580, &rs_lines[580])
        .iter()
        .all(|(_, t)| *t == Token::Comment));

    for (file, head, mid, tail) in [
        ("doc.md", "```", "fenced", "```"),
        (".env", "A=\"open", "middle", "end\""),
        ("page.html", "<!-- open", "still comment", "end -->"),
        ("tag.xml", "<item", "attr=\"1\">", "<leaf/>"),
        ("cfg.json", "{\"a\": 1,", "\"b\": 2,", "\"c\": 3}"),
        ("plain.unknown", "a", "b", "c"),
    ] {
        let buf = vec![
            head.to_owned(),
            mid.to_owned(),
            tail.to_owned(),
            "x = 1".to_owned(),
        ];
        let mut w = HighlightCache::default();
        let mut f = HighlightCache::default();
        w.prepare_window(Path::new(file), &buf, 2, 4, 1, None);
        f.prepare_revision(Path::new(file), &buf, 4, 1);
        assert_eq!(w.runs(2, &buf[2]), f.runs(2, &buf[2]), "{file}");
        assert_eq!(w.runs(3, &buf[3]), f.runs(3, &buf[3]), "{file}");
    }
}

#[test]
fn deleting_open_comment_and_plain_advance_preserve_continuation() {
    let mut plain = Highlighter::new(Path::new("plain.unknown"));
    let plain_cont = plain.continuation().expect("plain continuation");
    plain.resume(plain_cont);
    plain.advance_only("plain line");
    let path = Path::new("shrink.rs");
    let mut lines = vec![
        "/* open".to_owned(),
        "let x = 1;".to_owned(),
        "let y = 2;".to_owned(),
    ];
    let mut cache = HighlightCache::default();
    cache.prepare_window(path, &lines, 0, 3, 1, None);
    lines.remove(0);
    cache.prepare_window(path, &lines, 0, 2, 2, Some((0, 0)));
    assert!(cache
        .runs(0, &lines[0])
        .iter()
        .any(|(_, t)| *t == Token::Keyword));
}
