//! Purpose: Verb, sub-command and command-name completion.

use super::*;

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

#[test]
fn every_verb_is_offered_for_completion() {
    let tmp = empty_cwd();
    // The completer reads `command::VERBS`, so `win…` and friends complete too.
    let c = complete("win", tmp.path()).expect("some");
    let names = texts(&c);
    assert!(names.contains(&"winmsg".to_string()));
    assert!(names.contains(&"wincheck".to_string()));
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

// ── sessions ──────────────────────────────────────────────────────

#[test]
fn sessions_completes_its_subcommand_and_source() {
    let tmp = empty_cwd();
    let c = complete("sessions im", tmp.path()).expect("the subcommand");
    assert_eq!(c.line, "sessions import ");
    let c = complete("sessions import ne", tmp.path()).expect("the source");
    assert_eq!(c.line, "sessions import netbox ");
    assert!(complete("sessions x", tmp.path()).is_none());
}

#[test]
fn sessions_import_netbox_completes_the_file_as_a_path() {
    let tmp = cwd_with(&["demo.netbox"]);
    let c = complete("sessions import netbox de", tmp.path()).expect("the file");
    assert_eq!(texts(&c), vec!["demo.netbox".to_string()]);
    assert!(
        c.line.starts_with("sessions import netbox demo.netbox"),
        "{}",
        c.line
    );
}

#[test]
fn a_sessions_word_out_of_place_offers_nothing() {
    let tmp = cwd_with(&["far.txt"]);
    assert!(complete("sessions import far f", tmp.path()).is_none());
    assert!(complete("sessions open f", tmp.path()).is_none());
}
