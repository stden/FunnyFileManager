//! Purpose: `cd` path completion through the built-in directory scan.

use super::*;

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
