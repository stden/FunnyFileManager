//! Purpose: `cd` path completion through the built-in directory scan.

use super::*;

// ── path completion (built-in fallback) ───────────────────────────

#[test]
fn cd_completes_unique_directory_with_slash_no_space() {
    let tmp = empty_cwd();
    fs::create_dir(tmp.path().join("Documents")).unwrap();
    let c = complete("cd Doc", tmp.path()).expect("some");
    assert_eq!(c.line, format!("cd Documents{MAIN_SEPARATOR}"));
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
    let expected = format!("My Documents{MAIN_SEPARATOR}");
    assert_eq!(c.line, format!("cd {expected}"));
    // The completed line must parse back to the same single path argument.
    assert_eq!(
        crate::command::Command::parse(&c.line),
        crate::command::Command::Cd(PathBuf::from(expected))
    );
}

/// No behaviour change off Windows: the `cd` token is raw, and a backslash
/// is a name character there, not a separator to split at.
#[cfg(unix)]
#[test]
fn cd_treats_a_backslash_as_a_name_character_on_unix() {
    let tmp = cwd_with(&["a\\b/"]);
    // fish, when installed, merges in its own escaped spelling of the same
    // directory; the built-in scan's literal one must be offered either way.
    let c = complete("cd a\\", tmp.path()).expect("the name holds a backslash");
    assert!(texts(&c).contains(&"a\\b/".to_owned()), "{:?}", texts(&c));
}

#[test]
fn cd_completes_inside_a_directory_name_containing_spaces() {
    let tmp = empty_cwd();
    fs::create_dir_all(tmp.path().join("My Documents/reports")).unwrap();
    let c = complete("cd My Documents/rep", tmp.path()).expect("some");
    assert_eq!(c.line, "cd My Documents/reports/");
}

/// A bare drive is offered as its root only when that root can be listed.
/// Off Windows `C:\` is a relative name, so a directory of that name inside
/// the panel directory stands in for the drive; the real thing is exercised
/// in `tests_cd_windows`.
#[cfg(unix)]
#[test]
fn a_bare_drive_completes_to_its_root_only_when_it_can_be_listed() {
    let tmp = cwd_with(&["C:\\/"]);
    assert_eq!(
        path_complete(PathSyntax::Windows, "C:", tmp.path()),
        vec!["C:\\".to_owned()]
    );
    assert!(path_complete(PathSyntax::Windows, "Q:", tmp.path()).is_empty());
    // Under Posix `C:` is a name like any other: it matches that directory
    // by prefix and gets a slash, whatever else fish merges in.
    let posix = path_complete(PathSyntax::Posix, "C:", tmp.path());
    assert!(posix.contains(&"C:\\/".to_owned()), "{posix:?}");
}

/// The separator appended to a directory candidate is the one `dir_part`
/// ends in, not the syntax's own: `C:/work/al` completes to `C:/work/alpha/`
/// on Windows. Off Windows the host syntax never sees a `\`, so this is the
/// one test that reaches that wiring with a syntax whose main separator
/// differs from the typed one.
#[test]
fn a_directory_candidate_keeps_the_separator_the_user_typed() {
    let tmp = cwd_with(&["alpha/"]);
    let dir_part = format!("{}/", tmp.path().display());
    assert_eq!(
        builtin_path_complete(PathSyntax::Windows, &dir_part, "al", tmp.path()),
        vec!["alpha/".to_owned()]
    );
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
