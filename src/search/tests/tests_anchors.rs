//! Purpose: SIMD ASCII and Unicode anchor prefilters, backward non-alphabetic anchors,
//! whole-word offset windows, and Turkish `İ` / Kelvin `K` safety exclusions.

use super::*;

#[test]
fn backward_case_insensitive_search_uses_non_alphabetic_anchor_without_missing_hits() {
    let buffer = lines(&["ERR_42 noise eRr_42_extra Err_42 tail"]);
    let query = with("err_42", |s| {
        s.direction = Direction::Backward;
        s.words = WordMatch::WholeWords;
    });

    let first = query.find(&buffer, Caret::new(0, 36)).unwrap();
    assert_eq!(first, Caret::new(0, 26));
    let second = query.find(&buffer, first).unwrap();
    assert_eq!(second, Caret::new(0, 0));
}

#[test]
fn ascii_ignore_case_with_interior_anchor_handles_short_caret_and_exact_edges() {
    let buffer = lines(&["xx abc_def yy ABC_DEF"]);
    let backward = with("abc_def", |s| {
        s.direction = Direction::Backward;
        s.words = WordMatch::WholeWords;
    });
    assert_eq!(
        backward.find(&buffer, Caret::new(0, 2)),
        Some(Caret::new(0, 14))
    );
    assert_eq!(
        backward.find(&buffer, Caret::new(0, 14)),
        Some(Caret::new(0, 3))
    );

    let exact_back = with("abc_def", |s| {
        s.case = Case::Match;
        s.direction = Direction::Backward;
        s.words = WordMatch::WholeWords;
    });
    assert_eq!(
        exact_back.find(&buffer, Caret::new(0, 20)),
        Some(Caret::new(0, 3))
    );
}

#[test]
fn mixed_unicode_and_ascii_needle_prefilters_non_matching_lines_and_finds_match() {
    let buffer = lines(&[
        "обычная строка без якоря",
        "вторая строка тоже мимо",
        "начало Привет_МИР конец",
    ]);
    assert_eq!(
        search("привет_мир").find(&buffer, Caret::new(0, 0)),
        Some(Caret::new(2, 7))
    );
    assert_eq!(search("über=1").find(&buffer, Caret::new(0, 0)), None);
}

#[test]
fn safe_ascii_anchor_never_skips_turkish_dotted_i_or_kelvin_sign() {
    let kelvin_and_dotted = lines(&["hugo", "word İ\u{212A} end"]);
    assert_eq!(
        search("ik").find(&kelvin_and_dotted, Caret::new(0, 0)),
        Some(Caret::new(1, 5))
    );

    let turkish = lines(&["hugo", "port İstanbul dock"]);
    assert_eq!(
        search("istanbul").find(&turkish, Caret::new(0, 0)),
        Some(Caret::new(1, 5))
    );
}
