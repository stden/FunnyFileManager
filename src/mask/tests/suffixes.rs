//! Regression tests for common suffix masks and Unicode case folding.

use super::super::{fold_chars, glob_slice, Pattern};
use super::Mask;

#[test]
fn ascii_suffixes_skip_folding_even_with_a_unicode_basename() {
    for (mask, name, expected) in [
        ("***", "日本語", true),
        ("*.rs", "日本語.RS", true),
        ("**.rs", "日本語.tmp", false),
        ("*.html", ".ht", false),
    ] {
        let mut folded = None;
        assert_eq!(
            Pattern::new(mask.to_owned()).matches(name, &mut folded),
            expected
        );
        assert!(folded.is_none(), "{mask:?} folded {name:?}");
    }
}

const NAMES: [&str; 10] = [
    "",
    "s",
    "T.RS",
    "K",
    "xKs",
    "İ",
    "xİ",
    "ος",
    "i\u{307}",
    "日本語",
];

#[test]
fn suffix_masks_agree_with_the_folded_glob() {
    for pattern in [
        "*",
        "***",
        "*k",
        "**ks",
        "*.rs",
        "*i\u{307}",
        "*σ",
        "*.長",
        "*?s",
        "*s*",
    ] {
        let mask = Mask::parse(pattern);
        let chars = fold_chars(pattern);
        for name in NAMES {
            let expected = glob_slice(
                &chars,
                &fold_chars(name),
                |c| c == '*',
                |p, n| p == '?' || p == n,
            );
            assert_eq!(mask.matches(name), expected, "{pattern:?} on {name:?}");
        }
    }
}
