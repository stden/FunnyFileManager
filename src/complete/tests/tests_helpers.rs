//! Purpose: Pure helpers: word splitting, common prefix and directory resolution.

use super::*;

// ── helpers ───────────────────────────────────────────────────────

#[test]
fn split_last_token_handles_trailing_space() {
    assert_eq!(shellwords::last_word(PathSyntax::HOST, "cd "), ("cd ", ""));
    assert_eq!(
        shellwords::last_word(PathSyntax::HOST, "cd foo"),
        ("cd ", "foo")
    );
    assert_eq!(shellwords::last_word(PathSyntax::HOST, "foo"), ("", "foo"));
}

#[test]
fn split_last_token_splits_on_multi_byte_whitespace() {
    // U+3000 IDEOGRAPHIC SPACE is 3 bytes and U+00A0 NO-BREAK SPACE is 2;
    // splitting one byte past the separator's start would land mid-character
    // and panic. Both are `char::is_whitespace`, so both are reachable.
    assert_eq!(
        shellwords::last_word(PathSyntax::HOST, "cd\u{3000}foo"),
        ("cd\u{3000}", "foo")
    );
    assert_eq!(
        shellwords::last_word(PathSyntax::HOST, "cd\u{00a0}foo"),
        ("cd\u{00a0}", "foo")
    );
    assert_eq!(
        shellwords::last_word(PathSyntax::HOST, "cd\u{3000}"),
        ("cd\u{3000}", "")
    );
    assert_eq!(
        shellwords::last_word(PathSyntax::HOST, "\u{a0}"),
        ("\u{a0}", "")
    );
}

#[test]
fn multibyte_whitespace_does_not_panic() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("Documents")).unwrap();
    // Pasted text routinely carries NBSP; completing it must not abort FFM.
    drop(complete("cd\u{a0}Doc", tmp.path()));
    drop(complete("\u{a0}", tmp.path()));
    drop(complete("cd \u{3000}Doc", tmp.path()));
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
fn a_directory_candidate_is_finished_without_a_space_under_its_own_syntax() {
    use crate::models::CandidateKind::{Path as P, Word};
    assert_eq!(finish_word_for(PathSyntax::Posix, "docs/", P), "docs/");
    assert_eq!(finish_word_for(PathSyntax::Windows, "docs\\", P), "docs\\");
    assert_eq!(finish_word_for(PathSyntax::Windows, "docs/", P), "docs/");
    // A backslash ends a name, not a directory, under Posix.
    assert_eq!(finish_word_for(PathSyntax::Posix, "docs\\", P), "docs\\ ");
    assert_eq!(
        finish_word_for(PathSyntax::Posix, "notes.txt", P),
        "notes.txt "
    );
    // A word is never a directory, whatever it ends in.
    assert_eq!(
        finish_word_for(PathSyntax::Windows, "sort\\", Word),
        "sort\\ "
    );
    assert_eq!(finish_word("docs/", P), "docs/");
}

#[test]
fn longest_common_prefix_is_char_safe() {
    let v = vec!["caña".to_owned(), "cañon".to_owned()];
    assert_eq!(longest_common_prefix(&v), "cañ");
    assert_eq!(common_prefix_len("caña", "cañon"), "cañ".len());
}

#[test]
fn resolve_dir_expands_home() {
    let tmp = empty_cwd();
    // Relative joins onto cwd.
    assert_eq!(resolve_dir("", tmp.path()), tmp.path());
    assert_eq!(resolve_dir("sub/", tmp.path()), tmp.path().join("sub/"));
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
