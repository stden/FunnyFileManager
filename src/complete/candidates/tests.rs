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

#[test]
fn service_candidates_offer_subcommands_only_in_the_first_argument_slot() {
    let ctx = CompletionContext::for_cwd(Path::new("."));
    let jira = candidates_for(PathSyntax::Posix, "jira", "j", &ctx);
    assert!(jira.iter().any(|candidate| candidate.text == "jql"));
    assert_eq!(
        candidates_for(PathSyntax::Posix, "mcps", "e", &ctx)[0].text,
        "edit"
    );
    assert!(candidates_for(PathSyntax::Posix, "ai", "c", &ctx)
        .iter()
        .any(|candidate| candidate.text == "codex"));
    assert_eq!(
        candidates_for(PathSyntax::Posix, "jira jql", "", &ctx),
        Vec::<Candidate>::new()
    );
    assert_eq!(
        candidates_for(PathSyntax::Posix, "mcps edit", "", &ctx),
        Vec::<Candidate>::new()
    );
    assert_eq!(
        candidates_for(PathSyntax::Posix, "ai codex", "", &ctx),
        Vec::<Candidate>::new()
    );
}
