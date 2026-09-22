//! Purpose: Fuzzy ranking of verb and path candidates.

use super::*;

// ── fuzzy ranking ─────────────────────────────────────────────────

#[test]
fn test_verb_completion_accepts_a_fuzzy_query() {
    // "bkm" is not a prefix of "bookmark", but it is a subsequence.
    let completion = complete("bkm", Path::new(".")).expect("fuzzy verb match");
    let names = texts(&completion);
    assert!(
        names.contains(&"bookmark".to_string()),
        "candidates were {names:?}"
    );
}

#[test]
fn test_exact_prefix_candidate_ranks_first() {
    // "m" prefixes both "mark" and "mcp"; a fuzzy hit must not displace them.
    let completion = complete("m", Path::new(".")).expect("prefix match");
    let names = texts(&completion);
    assert_eq!(
        names.first(),
        Some(&"mcp".to_string()),
        "candidates were {names:?}"
    );
}

// ── fuzzy ranking: path candidates (review fix — dir_part pollution) ──
//
// Ranking is asserted on the pure helpers below, never end-to-end: with fish
// installed (the documented primary setup) `path_complete` delegates to fish,
// which does prefix — not subsequence — matching, so an e2e ordering test
// would assert one candidate set with fish and a different one without it.

#[test]
fn rank_by_leaf_ranks_exact_leaf_prefix_above_fuzzy_and_keeps_dir_part() {
    // Pure-function regression test for the dir_part-pollution bug: scoring
    // used to run against the full `dir_part+leaf` string, which meant
    // "sub/complete_b/".starts_with("com") was false even though the leaf
    // "complete_b" is an exact prefix match for "com".
    let candidates = vec![
        "sub/xcomz/".to_string(),
        "sub/command_a/".to_string(),
        "sub/complete_b/".to_string(),
    ];
    let ranked = rank_by_leaf("com", candidates);
    assert_eq!(
        ranked,
        vec![
            "sub/command_a/".to_string(),
            "sub/complete_b/".to_string(),
            "sub/xcomz/".to_string(),
        ]
    );
}

#[test]
fn rank_by_leaf_keeps_non_matching_leaf_instead_of_dropping_it() {
    // fish's own case-insensitive fallback can return "foo/" for a query
    // like "FOO"; smartcase then makes matching case-sensitive, so
    // `matcher::score` alone would reject it. fish already validated this
    // candidate, so ranking must never drop it — only order it.
    let ranked = rank_by_leaf("FOO", vec!["foo/".to_string()]);
    assert_eq!(ranked, vec!["foo/".to_string()]);
}

#[test]
fn rank_by_leaf_orders_matches_before_non_matching_leftovers() {
    let ranked = rank_by_leaf(
        "com",
        vec![
            "xcomz/".to_string(),
            "command_a/".to_string(),
            "FOO/".to_string(),
        ],
    );
    // "command_a/" is an exact leaf-prefix match, "xcomz/" a fuzzy one, and
    // "FOO/" doesn't match "com" at all — it must survive, but last.
    assert_eq!(
        ranked,
        vec![
            "command_a/".to_string(),
            "xcomz/".to_string(),
            "FOO/".to_string(),
        ]
    );
}

#[test]
fn fuzzy_leaf_matches_survive_fish_prefix_shadowing() {
    // fish only offers prefix matches: for "dwn" it returns "dwn-tools/" and
    // nothing else, shadowing the fuzzy match the design promises ("dwn"
    // matches "Downloads/"). The builtin fuzzy scan must always contribute,
    // with or without fish installed.
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("Downloads")).unwrap();
    fs::create_dir(tmp.path().join("dwn-tools")).unwrap();

    let completion = complete("cd dwn", tmp.path()).expect("candidates");
    let names = texts(&completion);
    assert!(
        names.contains(&"dwn-tools/".to_string()),
        "prefix match lost: {names:?}"
    );
    assert!(
        names.contains(&"Downloads/".to_string()),
        "fuzzy match must not be shadowed by fish's prefix matches: {names:?}"
    );
}

/// In the merged pool, an exact leaf-prefix match ranks ABOVE a fuzzy-only one,
/// regardless of which engine each came from. `fuzzy_leaf_matches_survive_*`
/// pins membership; this pins their order.
#[test]
fn merged_completion_ranks_prefix_before_fuzzy() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("Downloads")).expect("dir"); // fuzzy match for "dwn"
    fs::create_dir(tmp.path().join("dwn-tools")).expect("dir"); // exact prefix for "dwn"

    let completion = complete("cd dwn", tmp.path()).expect("candidates");
    let prefix_at = completion
        .candidates
        .iter()
        .position(|c| c.text == "dwn-tools/")
        .expect("prefix candidate present");
    let fuzzy_at = completion
        .candidates
        .iter()
        .position(|c| c.text == "Downloads/")
        .expect("fuzzy candidate present");
    assert!(
        prefix_at < fuzzy_at,
        "prefix match must outrank fuzzy: {:?}",
        completion.candidates
    );
}
