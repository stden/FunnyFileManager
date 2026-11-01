//! Unit tests: fuzzy matching and ranking.

use super::*;

#[test]
fn scores_account_for_runs_word_boundaries_and_each_skipped_character() {
    for (query, candidate, points) in [
        ("p", "project", 8),
        ("pr", "project", 18),
        ("ac", "abc", 8),
        ("ab", "x_ab", 16),
        ("ac", "a--c", 14),
        ("c", "abc", -1),
    ] {
        assert_eq!(
            score(query, candidate)
                .expect("matching subsequence")
                .points,
            points,
            "{query} against {candidate}"
        );
    }
}

#[test]
fn test_exact_prefix_scores_in_the_prefix_tier() {
    let score = score("cd", "cd").expect("prefix match");
    assert_eq!(score.tier, Tier::Prefix);
}

#[test]
fn test_subsequence_scores_in_the_fuzzy_tier() {
    // d-w-n appear in order inside "Downloads/", but not as a prefix.
    let score = score("dwn", "Downloads/").expect("fuzzy match");
    assert_eq!(score.tier, Tier::Fuzzy);
}

#[test]
fn test_non_subsequence_does_not_match() {
    assert_eq!(score("zzz", "Downloads/"), None);
}

#[test]
fn test_out_of_order_characters_do_not_match() {
    // "nwd" has the right letters in the wrong order.
    assert_eq!(score("nwd", "Downloads/"), None);
}

#[test]
fn test_empty_query_matches_everything_in_the_prefix_tier() {
    let score = score("", "anything").expect("empty query matches");
    assert_eq!(score.tier, Tier::Prefix);
}

#[test]
fn test_lowercase_query_is_case_insensitive() {
    assert!(score("dow", "Downloads/").is_some());
}

#[test]
fn test_uppercase_in_query_forces_case_sensitivity() {
    // Smartcase: any uppercase character makes the whole query case-sensitive.
    assert!(score("Dow", "Downloads/").is_some());
    assert_eq!(score("Dow", "downloads/"), None);
}

#[test]
fn test_consecutive_run_outscores_a_scattered_match() {
    let tight = score("cat", "cat.txt").expect("match");
    let loose = score("cat", "c_a_t.txt").expect("match");
    assert!(
        tight.points > loose.points,
        "tight={tight:?} should outscore loose={loose:?}"
    );
}

#[test]
fn test_rank_puts_prefix_matches_above_fuzzy_ones() {
    // "cd" is a prefix of "cd" and a subsequence of "copy_dir".
    let ranked = rank("cd", vec!["copy_dir".to_owned(), "cd".to_owned()]);
    assert_eq!(ranked, vec!["cd".to_owned(), "copy_dir".to_owned()]);
}

#[test]
fn test_rank_drops_non_matching_candidates() {
    let ranked = rank("cd", vec!["cd".to_owned(), "zzz".to_owned()]);
    assert_eq!(ranked, vec!["cd".to_owned()]);
}

#[test]
fn ascii_fold_does_not_match_dotted_capital_i() {
    assert_eq!(score("i", "İstanbul"), None);
}

#[test]
fn a_unicode_word_still_scores_as_a_prefix() {
    let scored = score("str", "straße").expect("prefix");
    assert_eq!(scored.tier, Tier::Prefix);
    assert_eq!(scored.points, 28);
}

#[test]
fn test_rank_is_deterministic_for_equally_scored_candidates() {
    // All three are prefix matches of equal length, so the lexicographic
    // tiebreak decides — ranking must never vary between runs.
    let ranked = rank("a", vec!["ac".to_owned(), "aa".to_owned(), "ab".to_owned()]);
    assert_eq!(
        ranked,
        vec!["aa".to_owned(), "ab".to_owned(), "ac".to_owned()]
    );
}
