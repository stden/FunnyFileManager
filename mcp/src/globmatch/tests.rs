//! Unit tests for the shared glob matcher.

use super::{matches, max_depth, validate_pattern, MAX_PATTERN_COMPONENTS, MAX_PATTERN_LEN};
use std::sync::mpsc;
use std::time::Duration;

#[test]
fn single_star_stays_inside_one_component() {
    assert!(matches("src/main.rs", "src/*.rs"));
    assert!(!matches("src/deep/a.rs", "src/*.rs"));
    // The directory part of the pattern is honoured, not discarded.
    assert!(!matches("build.rs", "src/*.rs"));
}

#[test]
fn double_star_spans_components() {
    assert!(matches("src/main.rs", "src/**/*.rs"));
    assert!(matches("src/deep/deeper/a.rs", "src/**/*.rs"));
    assert!(!matches("build.rs", "src/**/*.rs"));
    assert!(matches("a/b/c.txt", "**"));
    assert!(matches("a/b/c.txt", "**/*.txt"));
}

#[test]
fn double_star_also_matches_zero_components() {
    // "**/*.rs" is the usual "every .rs file" pattern — it has to include the
    // ones sitting directly in the search root, not only nested ones.
    assert!(matches("main.rs", "**/*.rs"));
    assert!(!matches("main.txt", "**/*.rs"));
}

#[test]
fn root_level_pattern_does_not_match_nested_paths() {
    assert!(matches("build.rs", "*.rs"));
    assert!(!matches("src/main.rs", "*.rs"));
}

#[test]
fn question_mark_matches_exactly_one_character() {
    assert!(matches("mod1.rs", "mod?.rs"));
    assert!(!matches("mod12.rs", "mod?.rs"));
    assert!(!matches("mod.rs", "mod?.rs"));
    assert!(!matches("a/b.rs", "a?b.rs"));
}

#[test]
fn plain_names_match_exactly_and_case_insensitively() {
    assert!(matches("README.md", "readme.md"));
    assert!(matches("notes.TXT", "*.txt"));
    assert!(!matches("notes.txt", "notes"));
}

#[test]
fn star_matches_empty_and_multiple_stars() {
    assert!(matches("ab", "a*b"));
    assert!(matches("axxb", "a*b"));
    assert!(matches("axxbyyc", "a*b*c"));
    assert!(!matches("axxb", "a*c"));
}

#[test]
fn max_depth_is_bounded_without_double_star() {
    assert_eq!(max_depth("*.rs"), Some(0));
    assert_eq!(max_depth("src/*.rs"), Some(1));
    assert_eq!(max_depth("a/b/*.rs"), Some(2));
    assert_eq!(max_depth("src/**/*.rs"), None);
}

// ─── hostile patterns ───────────────────────────────────────────

#[test]
fn repeated_double_stars_mean_the_same_as_one() {
    // Collapsing `**` runs is only allowed because it changes nothing.
    for candidate in ["a/b/c.txt", "c.txt", "a/c.rs", ""] {
        assert_eq!(
            matches(candidate, "**/**/**/*.txt"),
            matches(candidate, "**/*.txt"),
            "collapsing `**` must not change the verdict for {candidate:?}"
        );
        assert_eq!(
            matches(candidate, "**/**"),
            matches(candidate, "**"),
            "collapsing `**` must not change the verdict for {candidate:?}"
        );
    }
    assert_eq!(max_depth("a/**/**/b"), None);
}

/// Match on a worker thread so that a regression fails the test instead of
/// hanging the whole run: the pre-fix matcher does not return at all on these
/// inputs, and a wall-clock assertion after the fact would never be reached.
fn matches_within(candidate: &str, pattern: &str, budget: Duration) -> bool {
    let (sender, receiver) = mpsc::channel();
    let (candidate, pattern) = (candidate.to_owned(), pattern.to_owned());
    // A late answer, after the budget ran out, has no receiver left: the
    // send error is the thread's result, and nobody joins the thread.
    std::thread::spawn(move || sender.send(matches(&candidate, &pattern)));
    receiver
        .recv_timeout(budget)
        .unwrap_or_else(|_| panic!("matching did not finish within {budget:?}"))
}

/// Regression: `match_components` recursed twice per `**` with no memoization,
/// so cost grew ~4.6× per extra `**` — 43 bytes of pattern took 3.4 s, and a
/// bigger one never returned (it overflowed the 2 MiB tokio worker stack and
/// aborted the server). The budget is deliberately generous; the point is
/// "milliseconds, not minutes".
#[test]
fn pathological_double_star_patterns_return_promptly() {
    let directories = (0..15)
        .map(|i| format!("dir{i}"))
        .collect::<Vec<_>>()
        .join("/");
    let candidate = format!("{directories}/target.rs");

    let budget = Duration::from_secs(2);

    // The reported shape: a long run of `**`, which collapsing folds to one.
    let collapsible = format!("{}x", "**/".repeat(45));
    assert!(!matches_within(&candidate, &collapsible, budget));

    // Interleaved `**` cannot be collapsed, so this exercises the iterative
    // backtrack itself rather than the run-folding shortcut.
    let interleaved = format!("{}x", "**/a/".repeat(20));
    assert!(!matches_within(&candidate, &interleaved, budget));

    // The success path has to stay fast too, not just the rejecting one.
    assert!(matches_within(&candidate, "**/dir7/**/*.rs", budget));
}

#[test]
fn oversized_patterns_are_rejected_and_never_match() {
    let too_deep = format!("{}x", "a/".repeat(MAX_PATTERN_COMPONENTS + 1));
    assert!(validate_pattern(&too_deep)
        .expect_err("too many components")
        .contains("components"));
    assert!(
        !matches("a/x", &too_deep),
        "an unvalidated caller must fail closed, not scan"
    );

    let too_long = "a".repeat(MAX_PATTERN_LEN + 1);
    assert!(validate_pattern(&too_long)
        .expect_err("too many bytes")
        .contains("bytes"));
    assert!(!matches(&too_long, &too_long), "fail closed on length too");

    validate_pattern("src/**/*.rs").expect("an ordinary pattern is fine");
}

#[test]
fn unicode_segment_matching_folds_case_across_wildcards() {
    assert!(matches("документы/Отчёт-2026.TXT", "**/отчёт-*.txt"));
    assert!(!matches("документы/Отчёт-2026.TXT", "**/приказ-*.txt"));
}
