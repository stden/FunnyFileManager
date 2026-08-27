//! Purpose: Unit tests for the command-line tab-completion engine.
//!
use super::shellwords;
use super::*;
use std::fs;
use tempfile::TempDir;

/// A cwd that exists but is empty, for cases where the dir doesn't matter.
fn empty_cwd() -> TempDir {
    TempDir::new().expect("tempdir")
}

/// The `text` of each ranked candidate, for asserting on completion output.
fn texts(c: &Completion) -> Vec<String> {
    c.candidates.iter().map(|cand| cand.text.clone()).collect()
}

// ── verb completion ───────────────────────────────────────────────

#[test]
fn completes_unique_verb_prefix_with_trailing_space() {
    let tmp = empty_cwd();
    let c = complete("pw", tmp.path()).expect("some");
    assert_eq!(c.line, "pwd ");
    assert_eq!(texts(&c), vec!["pwd".to_string()]);
}

#[test]
fn empty_input_offers_all_verbs_at_common_prefix() {
    let tmp = empty_cwd();
    // All verbs share no common prefix, so the line is unchanged but the
    // candidate list is returned for display.
    let c = complete("", tmp.path()).expect("some");
    assert_eq!(c.line, "");
    assert!(texts(&c).contains(&"cd".to_string()));
    assert!(c.candidates.len() > 1);
}

#[test]
fn ambiguous_verb_prefix_advances_to_common_prefix() {
    let tmp = empty_cwd();
    // "quit" and ... only "quit" starts with "qu"; unique.
    let c = complete("qu", tmp.path()).expect("some");
    assert_eq!(c.line, "quit ");
}

#[test]
fn unknown_verb_prefix_returns_none() {
    let tmp = empty_cwd();
    assert!(complete("zzz", tmp.path()).is_none());
}

#[test]
fn argument_less_verb_offers_nothing() {
    let tmp = empty_cwd();
    assert!(complete("pwd ", tmp.path()).is_none());
}

// ── sub-command completion ────────────────────────────────────────

#[test]
fn sort_subcommand_completes() {
    let tmp = empty_cwd();
    let c = complete("sort ex", tmp.path()).expect("some");
    assert_eq!(c.line, "sort extension ");
}

#[test]
fn sort_ambiguous_lists_candidates() {
    let tmp = empty_cwd();
    // Empty token after "sort " → name/extension/size, no common prefix.
    let c = complete("sort ", tmp.path()).expect("some");
    assert_eq!(c.candidates.len(), 3);
    assert_eq!(c.line, "sort ");
}

#[test]
fn bookmark_first_arg_completes() {
    let tmp = empty_cwd();
    let c = complete("bookmark s", tmp.path()).expect("some");
    assert_eq!(c.line, "bookmark set ");
}

#[test]
fn bookmark_set_slot_completes_digit() {
    // `set` may target any slot, so the plain digit list survives unchanged —
    // this is exercised through the context-free wrapper (no bookmarks needed).
    let tmp = empty_cwd();
    let c = complete("bookmark set 7", tmp.path()).expect("some");
    assert_eq!(c.line, "bookmark set 7 ");
}

#[test]
fn mark_completes_all() {
    let tmp = empty_cwd();
    let c = complete("mark a", tmp.path()).expect("some");
    assert_eq!(c.line, "mark all ");
}

// ── path completion (built-in fallback) ───────────────────────────

#[test]
fn cd_completes_unique_directory_with_slash_no_space() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("Documents")).unwrap();
    let c = complete("cd Doc", tmp.path()).expect("some");
    assert_eq!(c.line, "cd Documents/");
}

#[test]
fn cd_completes_unique_file_with_trailing_space() {
    let tmp = empty_cwd();
    fs::write(tmp.path().join("notes.txt"), b"x").unwrap();
    let c = complete("cd not", tmp.path()).expect("some");
    assert_eq!(c.line, "cd notes.txt ");
}

#[test]
fn cd_ambiguous_advances_to_common_prefix() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("report_a")).unwrap();
    fs::create_dir(tmp.path().join("report_b")).unwrap();
    let c = complete("cd rep", tmp.path()).expect("some");
    assert_eq!(c.line, "cd report_");
    assert_eq!(c.candidates.len(), 2);
}

#[test]
fn cd_completes_within_nested_dir_part() {
    let tmp = empty_cwd();
    fs::create_dir_all(tmp.path().join("src/handlers")).unwrap();
    let c = complete("cd src/han", tmp.path()).expect("some");
    assert_eq!(c.line, "cd src/handlers/");
}

#[test]
fn cd_no_match_returns_none() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("alpha")).unwrap();
    assert!(complete("cd zzz", tmp.path()).is_none());
}

#[test]
fn cd_completes_a_directory_name_containing_spaces() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("My Documents")).unwrap();
    let c = complete("cd My", tmp.path()).expect("some");
    assert_eq!(c.line, "cd My Documents/");
    // The completed line must parse back to the same single path argument.
    assert_eq!(
        crate::command::Command::parse(&c.line),
        crate::command::Command::Cd(PathBuf::from("My Documents/"))
    );
}

#[test]
fn cd_completes_inside_a_directory_name_containing_spaces() {
    let tmp = empty_cwd();
    fs::create_dir_all(tmp.path().join("My Documents/reports")).unwrap();
    let c = complete("cd My Documents/rep", tmp.path()).expect("some");
    assert_eq!(c.line, "cd My Documents/reports/");
}

#[test]
fn every_verb_is_offered_for_completion() {
    let tmp = empty_cwd();
    // The completer reads `command::VERBS`, so `win…` and friends complete too.
    let c = complete("win", tmp.path()).expect("some");
    let names = texts(&c);
    assert!(names.contains(&"winmsg".to_string()));
    assert!(names.contains(&"wincheck".to_string()));
}

// ── helpers ───────────────────────────────────────────────────────

#[test]
fn split_last_token_handles_trailing_space() {
    assert_eq!(shellwords::last_word("cd "), ("cd ", ""));
    assert_eq!(shellwords::last_word("cd foo"), ("cd ", "foo"));
    assert_eq!(shellwords::last_word("foo"), ("", "foo"));
}

#[test]
fn split_last_token_splits_on_multi_byte_whitespace() {
    // U+3000 IDEOGRAPHIC SPACE is 3 bytes and U+00A0 NO-BREAK SPACE is 2;
    // splitting one byte past the separator's start would land mid-character
    // and panic. Both are `char::is_whitespace`, so both are reachable.
    assert_eq!(
        shellwords::last_word("cd\u{3000}foo"),
        ("cd\u{3000}", "foo")
    );
    assert_eq!(
        shellwords::last_word("cd\u{00a0}foo"),
        ("cd\u{00a0}", "foo")
    );
    assert_eq!(shellwords::last_word("cd\u{3000}"), ("cd\u{3000}", ""));
    assert_eq!(shellwords::last_word("\u{a0}"), ("\u{a0}", ""));
}

#[test]
fn multibyte_whitespace_does_not_panic() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("Documents")).unwrap();
    // Pasted text routinely carries NBSP; completing it must not abort FFM.
    let _ = complete("cd\u{a0}Doc", tmp.path());
    let _ = complete("\u{a0}", tmp.path());
    let _ = complete("cd \u{3000}Doc", tmp.path());
}

#[test]
fn split_target_keeps_the_whole_cd_argument() {
    assert_eq!(split_target("cd My Doc"), ("cd ", "My Doc"));
    assert_eq!(split_target("cd  My"), ("cd  ", "My"));
    assert_eq!(split_target("cd "), ("cd ", ""));
    assert_eq!(split_target("cd"), ("", "cd"));
    // Other verbs still complete token by token.
    assert_eq!(split_target("sort ex"), ("sort ", "ex"));
}

#[test]
fn longest_common_prefix_is_char_safe() {
    let v = vec!["caña".to_string(), "cañon".to_string()];
    assert_eq!(longest_common_prefix(&v), "cañ");
}

#[test]
fn resolve_dir_expands_home() {
    let tmp = empty_cwd();
    // Relative joins onto cwd.
    assert_eq!(resolve_dir("", tmp.path()), tmp.path());
    assert_eq!(resolve_dir("sub/", tmp.path()), tmp.path().join("sub/"));
}

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
fn resolve_dir_expands_tilde_like_cd_does() {
    // Under the env lock: `command::tests` runs `expand_tilde` with $HOME
    // removed, and an unexpanded `~` would join onto cwd and fail the last
    // assertion here.
    let _lock = crate::test_support::editor_env_lock();
    let tmp = empty_cwd();
    let home = resolve_dir("~/", tmp.path());
    assert_eq!(home, crate::command::expand_tilde(Path::new("~/")));
    assert!(!home.starts_with(tmp.path()), "~ must not join onto cwd");
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

// ── `cd` path completion: the two engines and their failure modes ──────

/// Whether the fish engine `path_complete` merges in is actually installed.
/// Mirrors production: `fish_path_complete` gives up the same way when the
/// process cannot be spawned.
///
/// Callers that branch on this and then spawn fish must hold
/// [`crate::test_support::env_lock`] for the whole check+spawn window: the
/// fake-`PATH` tests temporarily shadow `fish`, and a concurrent lookup would
/// either hang on the sleeper or miss a real install.
fn fish_is_installed() -> bool {
    Command::new("fish").arg("--version").output().is_ok()
}

/// The directory part names a regular file, so it cannot be listed. Completion
/// must offer nothing rather than falling back to the file's siblings.
#[test]
fn cd_completion_of_an_unlistable_directory_part_offers_nothing() {
    let tmp = empty_cwd();
    fs::write(tmp.path().join("notes.txt"), b"not a directory").expect("file");
    fs::create_dir(tmp.path().join("notes_dir")).expect("dir");
    fs::write(tmp.path().join("notes_dir").join("inner.txt"), b"x").expect("inner file");

    assert!(
        builtin_path_complete("notes.txt/", "i", tmp.path()).is_empty(),
        "a regular file must not be scanned as a directory"
    );
    assert!(
        complete("cd notes.txt/i", tmp.path()).is_none(),
        "a file used as a directory part must not complete"
    );

    // Control: the identical leaf completes when the directory part is real,
    // so the case above fails on the unlistable directory and nothing else.
    let real = complete("cd notes_dir/i", tmp.path()).expect("real directory completes");
    assert_eq!(real.line, "cd notes_dir/inner.txt ");
}

/// A quoted `cd` token: `"` belongs to no filename, so the built-in scan scores
/// nothing and any candidate can only have come from fish. This pins the merge
/// in `path_complete` — dropping it would leave quoted paths uncompletable on
/// the documented (fish-installed) setup.
#[test]
fn quoted_cd_token_completes_through_the_merged_fish_engine() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("alpha")).expect("dir");

    assert!(
        builtin_path_complete("", "\"al", tmp.path()).is_empty(),
        "the quote must not match any filename"
    );

    // Serialize with the fake-PATH fish tests (they shadow `fish` on PATH).
    let _lock = crate::test_support::env_lock();
    let completed = complete("cd \"al", tmp.path());
    if fish_is_installed() {
        let completion = completed.expect("fish must supply the quoted-token candidate");
        assert_eq!(texts(&completion), vec!["alpha/".to_string()]);
        assert_eq!(completion.line, "cd alpha/");
    } else {
        assert!(
            completed.is_none(),
            "without fish there is nothing to merge in"
        );
    }
}

// ── `complete` vs `complete_with` ───────────────────────────────────

#[test]
fn complete_and_complete_with_for_cwd_agree() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("Documents")).unwrap();
    let a = complete("cd Doc", tmp.path()).expect("some");
    let b = complete_with("cd Doc", &CompletionContext::for_cwd(tmp.path())).expect("some");
    assert_eq!(a.line, b.line);
    assert_eq!(texts(&a), texts(&b));
}

// ── bookmark slot completion (populated slots + path hints) ────────

use std::path::PathBuf;

fn ctx_with_bookmarks<'a>(
    cwd: &'a std::path::Path,
    slots: &'a [Option<PathBuf>],
) -> CompletionContext<'a> {
    CompletionContext {
        cwd,
        bookmarks: slots,
    }
}

#[test]
fn bookmark_go_offers_only_populated_slots_with_path_hints() {
    let tmp = empty_cwd();
    let mut slots = vec![None; 10];
    slots[0] = Some(PathBuf::from("/srv/app"));
    slots[3] = Some(PathBuf::from("/home/me/dl"));

    let c = complete_with("bookmark go ", &ctx_with_bookmarks(tmp.path(), &slots))
        .expect("populated slots complete");
    assert_eq!(texts(&c), vec!["0".to_string(), "3".to_string()]);
    // The path rides along as the dropdown hint.
    let hint0 = c
        .candidates
        .iter()
        .find(|c| c.text == "0")
        .unwrap()
        .hint
        .as_deref();
    let hint3 = c
        .candidates
        .iter()
        .find(|c| c.text == "3")
        .unwrap()
        .hint
        .as_deref();
    assert_eq!(hint0, Some("/srv/app"));
    assert_eq!(hint3, Some("/home/me/dl"));
}

#[test]
fn bookmark_go_with_no_populated_slots_completes_nothing() {
    let tmp = empty_cwd();
    let slots = vec![None; 10];
    assert!(
        complete_with("bookmark go ", &ctx_with_bookmarks(tmp.path(), &slots)).is_none(),
        "no populated slot means nothing to offer"
    );
}

#[test]
fn bookmark_go_filters_populated_slots_by_typed_digit() {
    let tmp = empty_cwd();
    let mut slots = vec![None; 10];
    slots[3] = Some(PathBuf::from("/a"));
    slots[7] = Some(PathBuf::from("/b"));

    let c = complete_with("bookmark go 3", &ctx_with_bookmarks(tmp.path(), &slots))
        .expect("slot 3 matches");
    assert_eq!(texts(&c), vec!["3".to_string()]);
    assert_eq!(c.line, "bookmark go 3 ");
}

#[test]
fn bookmark_go_without_context_bookmarks_offers_nothing() {
    // The bare `complete()` wrapper has no bookmark slots, so `go` cannot list
    // any — it must not fall back to the blind 0-9 digit list.
    let tmp = empty_cwd();
    assert!(complete("bookmark go ", tmp.path()).is_none());
}

// ── fish engine: output parsing, merge dedup, and ordering ─────────────

/// The fish output parser keeps the value column, drops the tab-separated
/// description, and skips blank lines. Pure (no fish spawn), so the
/// description-stripping is exercised on every platform — unlike the live
/// engine, whose `cd` output happens to carry no description here.
#[test]
fn parse_fish_completions_keeps_values_drops_descriptions_and_blanks() {
    let raw = "alpha/\tDirectory\nbeta/\ngamma/\tSubcommand\n\n\n";
    assert_eq!(
        parse_fish_completions(raw),
        vec![
            "alpha/".to_string(),
            "beta/".to_string(),
            "gamma/".to_string(),
        ],
    );
    assert!(parse_fish_completions("").is_empty());
    // A value with interior spaces (a path with spaces) survives whole; only
    // the tab boundary splits.
    assert_eq!(
        parse_fish_completions("my docs/\tDirectory"),
        vec!["my docs/".to_string()],
    );
}

/// The live fish engine offers the directories that actually exist in `cwd`.
/// Runs real `fish`, so it is skipped where fish is not installed.
///
/// The token is the path fragment after `cd ` — `fish_path_complete` prefixes
/// `cd` itself when it builds the line for fish.
#[test]
fn fish_path_complete_offers_existing_directories() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("alpha")).expect("dir");
    fs::create_dir(tmp.path().join("alpine")).expect("dir");

    // Serialize with the fake-PATH fish tests (they shadow `fish` on PATH).
    let _lock = crate::test_support::env_lock();
    if !fish_is_installed() {
        return;
    }

    let Some(candidates) = fish_path_complete("al", tmp.path()) else {
        // fish is installed but produced nothing (unusual config); nothing to assert.
        return;
    };
    assert!(
        candidates.iter().any(|c| c == "alpha/"),
        "existing directory must be offered: {candidates:?}"
    );
    assert!(
        candidates.iter().any(|c| c == "alpine/"),
        "the other matching directory must be offered too: {candidates:?}"
    );
}

/// fish yields nothing for a leaf that matches no directory, and the engine
/// reports that as `None` (not `Some([])`) so the caller's fallback is clean.
#[test]
fn fish_path_complete_returns_none_when_nothing_matches() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("alpha")).expect("dir");

    // Serialize with the fake-PATH fish tests (they shadow `fish` on PATH).
    let _lock = crate::test_support::env_lock();
    if !fish_is_installed() {
        return;
    }

    assert!(
        fish_path_complete("zzq_no_such_prefix", tmp.path()).is_none(),
        "no match must be None, never Some(empty)"
    );
}

/// A hung fish must not freeze Tab: the engine gives up after its deadline and
/// returns `None` so the built-in scan remains the only source. Uses a fake
/// `fish` on `PATH` that sleeps, so this does not need a real fish install.
#[cfg(unix)]
#[test]
fn fish_path_complete_times_out_on_a_hung_fish() {
    use std::os::unix::fs::PermissionsExt;
    use std::time::{Duration, Instant};

    let tmp = empty_cwd();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).expect("bin");
    let fish = bin.join("fish");
    fs::write(&fish, "#!/bin/sh\nsleep 30\n").expect("write fake fish");
    let mut perms = fs::metadata(&fish).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&fish, perms).expect("chmod");

    let _lock = crate::test_support::env_lock();
    let previous_path = std::env::var_os("PATH");
    let mut new_path = bin.into_os_string();
    if let Some(rest) = &previous_path {
        new_path.push(":");
        new_path.push(rest);
    }
    std::env::set_var("PATH", &new_path);

    let started = Instant::now();
    let result = fish_path_complete("al", tmp.path());

    match previous_path {
        Some(p) => std::env::set_var("PATH", p),
        None => std::env::remove_var("PATH"),
    }

    assert!(
        result.is_none(),
        "a hung fish must yield None, not hang the caller"
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "timed out in {:?}, expected ~1.5s",
        started.elapsed()
    );
}

/// A fish that exits non-zero is treated like "no completions" so the built-in
/// scan remains the only source rather than a hard error.
#[cfg(unix)]
#[test]
fn fish_path_complete_returns_none_when_fish_exits_nonzero() {
    use std::os::unix::fs::PermissionsExt;

    let tmp = empty_cwd();
    let bin = tmp.path().join("bin");
    fs::create_dir(&bin).expect("bin");
    let fish = bin.join("fish");
    fs::write(&fish, "#!/bin/sh\nexit 1\n").expect("write fake fish");
    let mut perms = fs::metadata(&fish).expect("meta").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&fish, perms).expect("chmod");

    let _lock = crate::test_support::env_lock();
    let previous_path = std::env::var_os("PATH");
    let mut new_path = bin.into_os_string();
    if let Some(rest) = &previous_path {
        new_path.push(":");
        new_path.push(rest);
    }
    std::env::set_var("PATH", &new_path);

    let result = fish_path_complete("al", tmp.path());

    match previous_path {
        Some(p) => std::env::set_var("PATH", p),
        None => std::env::remove_var("PATH"),
    }

    assert!(
        result.is_none(),
        "non-zero fish exit must be None: {result:?}"
    );
}

/// A directory both engines return appears exactly once: `path_complete`
/// deduplicates the built-in scan against fish's candidates. Holds with fish
/// (both engines emit `alpha/`) and without it (only the built-in scan does).
#[test]
fn merged_completion_deduplicates_a_shared_directory() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("alpha")).expect("dir");

    let completion = complete("cd al", tmp.path()).expect("alpha/ completes");
    let hits = completion
        .candidates
        .iter()
        .filter(|c| c.text == "alpha/")
        .count();
    assert_eq!(
        hits, 1,
        "shared entry must not be doubled: {:?}",
        completion.candidates
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

#[cfg(unix)]
#[test]
fn fish_poll_loop_waits_for_a_slow_completer() {
    use std::os::unix::fs::PermissionsExt;

    let temp = tempfile::TempDir::new().expect("temp dir");
    let stub = temp.path().join("slow-fish.sh");
    std::fs::write(
        &stub,
        "#!/bin/sh\nsleep 0.08\nprintf 'alpha\\tDirectory\\n'\n",
    )
    .expect("write stub");
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o755)).expect("chmod");

    super::FISH_PROGRAM_FOR_TESTS.with(|slot| *slot.borrow_mut() = Some(stub));
    // Retry while the freshly-written stub is still ETXTBSY.
    //
    // `fs::write` closes its own handle, but a sibling test thread that forks
    // while it is open hands a copy of that write descriptor to its child, and
    // `exec` of the file then fails with "text file busy" until the child
    // exits. This suite forks constantly (every `ops::shell` test), and a
    // standalone probe measured the race at ~20% of attempts against eight
    // spawning threads — which is what made this test fail about one run in
    // five. `fish_path_complete` swallows the spawn error on purpose (a fish
    // that will not start is not a completion), so the retry belongs here.
    let candidates = (0..20).find_map(|_| {
        let result = super::fish_path_complete("al", temp.path());
        if result.is_none() {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        result
    });
    super::FISH_PROGRAM_FOR_TESTS.with(|slot| *slot.borrow_mut() = None);

    // The stub sleeps past several 20ms polls before answering, so the loop's
    // wait branch runs; the answer must still arrive intact afterwards.
    assert_eq!(candidates, Some(vec!["alpha".to_owned()]));
}

// ── argument completion for arbitrary commands ────────────────────

/// A cwd holding the given entries; directories are the names ending in `/`.
fn cwd_with(entries: &[&str]) -> TempDir {
    let tmp = TempDir::new().expect("tempdir");
    for entry in entries {
        match entry.strip_suffix('/') {
            Some(dir) => fs::create_dir(tmp.path().join(dir)).expect("mkdir"),
            None => fs::write(tmp.path().join(entry), b"x").expect("write"),
        }
    }
    tmp
}

#[test]
fn an_argument_of_an_arbitrary_command_completes_as_a_path() {
    // The whole point of the change: only `cd` used to complete a path, so
    // `cat REA<Tab>` offered nothing at all.
    let tmp = cwd_with(&["README.md", "unrelated.txt"]);
    let c = complete("cat REA", tmp.path()).expect("path candidates");
    assert_eq!(c.line, "cat README.md ");
}

#[test]
fn every_argument_position_completes_a_path_not_just_the_first() {
    let tmp = cwd_with(&["alpha.txt", "beta.txt"]);
    let c = complete("diff alpha.txt bet", tmp.path()).expect("path candidates");
    assert_eq!(c.line, "diff alpha.txt beta.txt ");
}

#[test]
fn a_completed_name_with_a_space_comes_back_escaped() {
    // Spliced raw, `sh -c` would read this as two arguments.
    let tmp = cwd_with(&["My Documents/"]);
    let c = complete("cat My", tmp.path()).expect("path candidates");
    assert_eq!(c.line, "cat My\\ Documents/");
    assert_eq!(shellwords::unescape(&texts(&c)[0]), "My Documents/");
}

#[test]
fn an_already_escaped_token_matches_the_name_it_stands_for() {
    let tmp = cwd_with(&["My Documents/"]);
    // Re-Tabbing into a name that was completed once must not start over
    // against the literal text `My\ Doc`.
    let c = complete("cat My\\ Doc", tmp.path()).expect("path candidates");
    assert_eq!(c.line, "cat My\\ Documents/");
}

#[test]
fn a_built_in_verb_with_no_modelled_argument_still_offers_nothing() {
    // `pwd` takes no arguments; guessing a path there would be worse than
    // silence, and it must not fall through to the shell-argument rule.
    let tmp = cwd_with(&["README.md"]);
    assert!(complete("pwd REA", tmp.path()).is_none());
}

#[test]
fn cd_keeps_its_unescaped_whole_line_argument() {
    // `Command::parse` strips quotes for `cd` rather than resolving escapes,
    // so its candidates must stay literal or the two would disagree.
    let tmp = cwd_with(&["My Documents/"]);
    let c = complete("cd My", tmp.path()).expect("path candidates");
    assert_eq!(c.line, "cd My Documents/");
}

// ── command-name completion ───────────────────────────────────────

#[test]
fn a_first_word_holding_a_separator_completes_as_a_path() {
    // `./build.sh` and `~/bin/tool` name a program by location, not by name.
    let tmp = cwd_with(&["scripts/"]);
    let c = complete("./scr", tmp.path()).expect("path candidates");
    assert_eq!(c.line, "./scripts/");
}

#[test]
fn a_unique_verb_prefix_is_not_diluted_by_path_programs() {
    // `pwdx` exists on many hosts; it must not turn an accepted keystroke
    // into a menu when `pwd` is the only verb that matches.
    let tmp = empty_cwd();
    let c = complete("pw", tmp.path()).expect("verb");
    assert_eq!(c.line, "pwd ");
    assert_eq!(texts(&c), vec!["pwd".to_string()]);
}

#[test]
fn built_in_verbs_outrank_path_executables() {
    let tmp = empty_cwd();
    // "c" matches the verb `cd` and, on any real host, a pile of programs.
    let c = complete("c", tmp.path()).expect("candidates");
    let names = texts(&c);
    assert_eq!(
        names.first(),
        Some(&"cd".to_string()),
        "a built-in must lead, candidates were {names:?}"
    );
}

// ── tilde: expansion syntax vs. a file whose name starts with `~` ──

/// Run `body` with `$HOME` pointed at `home`, holding the crate-wide env lock.
///
/// `HOME` is process-global and `history_file_path` reads it too, so a test
/// that sets it without the lock can redirect a concurrent test's history.
fn with_home<T>(home: &Path, body: impl FnOnce() -> T) -> T {
    let _lock = crate::test_support::env_lock();
    let previous = std::env::var_os("HOME");
    std::env::set_var("HOME", home);
    let result = body();
    match previous {
        Some(value) => std::env::set_var("HOME", value),
        None => std::env::remove_var("HOME"),
    }
    result
}

#[test]
fn a_home_relative_argument_keeps_its_tilde_unescaped() {
    // Escaping it hands `sh -c` a literal directory named `~`, which does not
    // exist — the completion would produce a command that cannot work.
    let home = TempDir::new().expect("tempdir");
    fs::create_dir(home.path().join("Documents")).expect("mkdir");
    let cwd = empty_cwd();

    let line = with_home(home.path(), || {
        complete("cat ~/Doc", cwd.path()).map(|c| c.line)
    });
    assert_eq!(line.as_deref(), Some("cat ~/Documents/"));
}

#[test]
fn a_file_whose_name_starts_with_a_tilde_is_escaped() {
    // Here the tilde is part of a name, so the shell must not expand it.
    let cwd = cwd_with(&["~backup"]);
    let c = complete("cat ~back", cwd.path()).expect("path candidates");
    assert_eq!(c.line, "cat \\~backup ");
}

#[test]
fn cd_completes_a_home_relative_path_without_escapes() {
    // `Command::parse` strips quotes for `cd` rather than resolving escapes,
    // so its candidates must stay literal in both directions.
    let home = TempDir::new().expect("tempdir");
    fs::create_dir(home.path().join("Documents")).expect("mkdir");
    let cwd = empty_cwd();

    let line = with_home(home.path(), || {
        complete("cd ~/Doc", cwd.path()).map(|c| c.line)
    });
    assert_eq!(line.as_deref(), Some("cd ~/Documents/"));
}

#[test]
fn home_syntax_is_decided_by_the_slash() {
    assert!(is_home_syntax("~"));
    assert!(is_home_syntax("~/"));
    assert!(is_home_syntax("~/Documents"));
    assert!(is_home_syntax("~user/mail"));
    assert!(!is_home_syntax("~backup"));
    assert!(!is_home_syntax("plain"));
    assert!(!is_home_syntax(""));
}
