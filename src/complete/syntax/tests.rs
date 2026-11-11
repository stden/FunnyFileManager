//! Purpose: Pins both path spellings — which characters separate, which one is appended, and where a token splits — on every OS.

use super::PathSyntax::{self, Posix, Windows};

#[test]
fn host_syntax_follows_the_build_target() {
    let expected = if cfg!(windows) { Windows } else { Posix };
    assert_eq!(PathSyntax::HOST, expected);
    assert_eq!(PathSyntax::HOST.main_separator(), std::path::MAIN_SEPARATOR);
}

#[test]
fn only_windows_treats_a_backslash_as_a_separator() {
    assert!(Posix.is_separator('/'));
    assert!(!Posix.is_separator('\\'));
    assert!(Windows.is_separator('/'));
    assert!(Windows.is_separator('\\'));
    assert!(!Windows.is_separator(':'));
}

#[test]
fn a_token_splits_after_its_last_separator() {
    assert_eq!(Posix.split_token("src/han"), ("src/", "han"));
    assert_eq!(Posix.split_token("han"), ("", "han"));
    assert_eq!(Posix.split_token("src/"), ("src/", ""));
    assert_eq!(Windows.split_token("C:\\work\\fo"), ("C:\\work\\", "fo"));
    assert_eq!(Windows.split_token("C:\\work\\"), ("C:\\work\\", ""));
    assert_eq!(Windows.split_token("C:/work/fo"), ("C:/work/", "fo"));
    assert_eq!(Windows.split_token("C:\\work/fo"), ("C:\\work/", "fo"));
    assert_eq!(Windows.split_token("sub"), ("", "sub"));
}

#[test]
fn a_backslash_is_part_of_the_leaf_under_posix() {
    // The user's report, seen from Linux: `C:\work\` is one leaf that no name
    // matches. Under Posix that is right — a backslash is a name character.
    assert_eq!(Posix.split_token("C:\\work\\"), ("", "C:\\work\\"));
    assert_eq!(Posix.split_token("a\\b"), ("", "a\\b"));
}

#[test]
fn a_candidate_splits_before_its_own_trailing_separator() {
    assert_eq!(Posix.split_leaf("sub/handlers/"), ("sub/", "handlers/"));
    assert_eq!(Posix.split_leaf("notes.txt"), ("", "notes.txt"));
    assert_eq!(Posix.split_leaf("a\\b/"), ("", "a\\b/"));
    assert_eq!(
        Windows.split_leaf("C:\\work\\foo\\"),
        ("C:\\work\\", "foo\\")
    );
    assert_eq!(Windows.split_leaf("foo\\"), ("", "foo\\"));
    assert_eq!(Windows.split_leaf("C:/work/foo/"), ("C:/work/", "foo/"));
}

#[test]
fn the_appended_separator_is_the_one_typed_last_else_the_main_one() {
    assert_eq!(Posix.dir_suffix(""), '/');
    assert_eq!(Posix.dir_suffix("src/"), '/');
    assert_eq!(Posix.dir_suffix("a\\b\\"), '/');
    assert_eq!(Windows.dir_suffix(""), '\\');
    assert_eq!(Windows.dir_suffix("sub"), '\\');
    assert_eq!(Windows.dir_suffix("C:\\work\\"), '\\');
    assert_eq!(Windows.dir_suffix("C:/work/"), '/');
    assert_eq!(Windows.dir_suffix("C:\\work/"), '/');
    assert_eq!(Windows.dir_suffix("C:/work\\"), '\\');
}

#[test]
fn a_trailing_separator_marks_a_directory() {
    assert!(Posix.ends_with_separator("docs/"));
    assert!(!Posix.ends_with_separator("docs\\"));
    assert!(!Posix.ends_with_separator(""));
    assert!(Windows.ends_with_separator("docs\\"));
    assert!(Windows.ends_with_separator("docs/"));
    assert!(!Windows.ends_with_separator("docs"));
}

#[test]
fn a_bare_drive_names_its_root_under_windows_only() {
    assert_eq!(Windows.drive_root("C:").as_deref(), Some("C:\\"));
    assert_eq!(Windows.drive_root("d:").as_deref(), Some("d:\\"));
    // Anything but `<letter>:` is a path or a name, not a drive.
    assert_eq!(Windows.drive_root("C:\\"), None);
    assert_eq!(Windows.drive_root("C"), None);
    assert_eq!(Windows.drive_root("CD:"), None);
    assert_eq!(Windows.drive_root("1:"), None);
    assert_eq!(Windows.drive_root(""), None);
    assert_eq!(Posix.drive_root("C:"), None);
}
