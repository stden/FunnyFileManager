//! Purpose: Which first words name a program by location rather than by name.

use super::*;

#[test]
fn a_first_word_is_a_path_when_it_holds_the_hosts_separator() {
    assert!(looks_like_a_path(PathSyntax::Posix, "./build.sh"));
    assert!(looks_like_a_path(PathSyntax::Posix, "~/bin/tool"));
    assert!(looks_like_a_path(PathSyntax::Posix, "bin/tool"));
    assert!(!looks_like_a_path(PathSyntax::Posix, "C:\\tools\\prog"));
    assert!(looks_like_a_path(PathSyntax::Windows, "C:\\tools\\prog"));
    assert!(looks_like_a_path(PathSyntax::Windows, "C:/tools/prog"));
    assert!(looks_like_a_path(PathSyntax::Windows, ".\\build.cmd"));
    assert!(!looks_like_a_path(PathSyntax::Windows, "git"));
}

/// A leading `.` or `~` names a location before any separator is typed:
/// `.` alone is the folder the user will descend into, `~` the home.
#[test]
fn a_first_word_is_a_path_from_its_first_dot_or_tilde() {
    for syntax in [PathSyntax::Posix, PathSyntax::Windows] {
        assert!(looks_like_a_path(syntax, "."), "{syntax:?}");
        assert!(looks_like_a_path(syntax, "~"), "{syntax:?}");
        assert!(looks_like_a_path(syntax, ".hidden"), "{syntax:?}");
        assert!(!looks_like_a_path(syntax, "hidden."), "{syntax:?}");
    }
}
