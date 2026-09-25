//! Purpose: Path completion for arbitrary shell-command arguments, escaping and tilde handling.

use super::*;

// ── argument completion for arbitrary commands ────────────────────

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

#[cfg(unix)]
#[test]
fn a_completed_name_with_a_space_comes_back_escaped() {
    // Spliced raw, `sh -c` would read this as two arguments.
    let tmp = cwd_with(&["My Documents/"]);
    let c = complete("cat My", tmp.path()).expect("path candidates");
    assert_eq!(c.line, "cat My\\ Documents/");
    assert_eq!(
        shellwords::unescape(PathSyntax::HOST, &texts(&c)[0]),
        "My Documents/"
    );
}

#[cfg(unix)]
#[test]
fn an_already_escaped_token_matches_the_name_it_stands_for() {
    let tmp = cwd_with(&["My Documents/"]);
    // Re-Tabbing into a name that was completed once must not start over
    // against the literal text `My\ Doc`.
    let c = complete("cat My\\ Doc", tmp.path()).expect("path candidates");
    assert_eq!(c.line, "cat My\\ Documents/");
}

#[cfg(windows)]
#[test]
fn a_completed_name_with_a_space_comes_back_double_quoted_on_windows() {
    // cmd has no backslash escape: the name is quoted as a whole, and the
    // separator stays outside, where the C runtime cannot read `\"` as a
    // literal quote.
    let tmp = cwd_with(&["My Documents/"]);
    let c = complete("cat My", tmp.path()).expect("path candidates");
    assert_eq!(c.line, "cat \"My Documents\"\\");
    assert_eq!(
        shellwords::unescape(PathSyntax::HOST, &texts(&c)[0]),
        "My Documents\\"
    );
}

#[cfg(windows)]
#[test]
fn a_quoted_token_matches_the_name_it_stands_for_on_windows() {
    let tmp = cwd_with(&["My Documents/"]);
    let c = complete("cat \"My Doc", tmp.path()).expect("path candidates");
    assert_eq!(c.line, "cat \"My Documents\"\\");
}

#[cfg(windows)]
#[test]
fn a_backslash_path_argument_completes_on_windows() {
    // `dir C:\work\fi` used to unescape to `C:workfi` and match nothing.
    // The temp path goes through `escape` in both directions: under a profile
    // directory named with a space (`John Doe`) it holds one, and then the
    // typed directory and the completed file are both double-quoted, `"…"\REA`
    // style.
    let tmp = cwd_with(&["README.md"]);
    let root = tmp.path().display();
    let typed = shellwords::escape(PathSyntax::HOST, &format!("{root}\\"));
    let c = complete(&format!("type {typed}REA"), tmp.path()).expect("path candidates");
    let expected = shellwords::escape(PathSyntax::HOST, &format!("{root}\\README.md"));
    assert_eq!(c.line, format!("type {expected} "));
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
    assert_eq!(c.line, format!("cd My Documents{MAIN_SEPARATOR}"));
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

#[cfg(unix)]
#[test]
fn a_file_whose_name_starts_with_a_tilde_is_escaped() {
    // Here the tilde is part of a name, so the shell must not expand it.
    let cwd = cwd_with(&["~backup"]);
    let c = complete("cat ~back", cwd.path()).expect("path candidates");
    assert_eq!(c.line, "cat \\~backup ");
}

#[cfg(windows)]
#[test]
fn a_file_whose_name_starts_with_a_tilde_is_not_escaped_on_windows() {
    // cmd expands no tilde, so there is nothing to protect the name from.
    let cwd = cwd_with(&["~backup"]);
    let c = complete("cat ~back", cwd.path()).expect("path candidates");
    assert_eq!(c.line, "cat ~backup ");
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
