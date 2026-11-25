//! Query-specific folding preserves matches while avoiding unrelated Unicode work.

use super::super::folding::Folding;

#[test]
fn ascii_queries_leave_unrelated_scripts_unfolded() {
    let mut output = String::from("Prefix Ж ");
    Folding::for_needle("needle").append("INFO Σ Ж 日本語", &mut output);
    assert_eq!(output, "Prefix Ж info Σ Ж 日本語");
}

#[test]
fn non_ascii_queries_still_fold_expansions_and_both_sigmas() {
    let mut output = String::new();
    Folding::for_needle("σ").append("İΣςЖABC", &mut output);
    assert_eq!(output, "i\u{307}σσжabc");
}

#[test]
fn ascii_queries_preserve_unicode_mappings_that_can_match_ascii() {
    let mut output = String::new();
    Folding::for_needle("key").append("\u{212A}EY İX", &mut output);
    assert_eq!(output, "key i\u{307}x");
    assert!(
        !output.contains("ix"),
        "the expanding lowercase keeps its mark"
    );
}

#[test]
fn unicode_tables_have_only_the_two_guarded_ascii_mappings() {
    let mappings: String = (128..=u32::from(char::MAX))
        .filter_map(char::from_u32)
        .filter(|letter| letter.to_lowercase().any(|lower| lower.is_ascii()))
        .collect();
    assert_eq!(mappings, "İ\u{212A}");
}
