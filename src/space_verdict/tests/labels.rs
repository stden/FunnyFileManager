//! Purpose: Every verdict explains the retention or rebuild action to the user.

use super::*;

#[test]
fn safe_entries_name_the_tool_that_can_rebuild_them() {
    let cases = [
        (
            Regenerable::CargoTarget,
            "Cargo build output, rebuilt by cargo build",
        ),
        (
            Regenerable::NodeModules,
            "installed packages, rebuilt by the package manager",
        ),
        (
            Regenerable::BuildOutput,
            "build output, rebuilt by the build",
        ),
        (Regenerable::GradleCache, "Gradle cache, rebuilt by Gradle"),
        (
            Regenerable::PythonBytecode,
            "Python bytecode cache, rebuilt on import",
        ),
        (Regenerable::PytestCache, "pytest cache, rebuilt by pytest"),
        (Regenerable::MypyCache, "mypy cache, rebuilt by mypy"),
        (Regenerable::RuffCache, "ruff cache, rebuilt by ruff"),
    ];
    for (kind, expected) in cases {
        let verdict = Verdict::Safe(kind);
        assert_eq!(verdict.label(), "SAFE");
        assert_eq!(verdict.reason(), expected);
    }
}

#[test]
fn retained_entries_explain_which_git_or_link_safety_rule_applies() {
    for (kind, expected) in [
        (Keep::GitMetadata, "git's own files"),
        (Keep::HoldsGit, "holds a .git folder"),
        (Keep::Symlink, "a link, never followed"),
        (Keep::Tracked, "tracked by git"),
    ] {
        let verdict = Verdict::Keep(kind);
        assert_eq!(verdict.label(), "KEEP");
        assert_eq!(verdict.reason(), expected);
    }
}

#[test]
fn review_reasons_add_the_large_file_note_only_above_the_threshold() {
    for (reason, expected) in [
        (ReviewReason::NotUnderGit, "not under git"),
        (ReviewReason::NotIgnored, "not ignored by git"),
        (
            ReviewReason::IgnoredNotCache,
            "ignored by git, not a known cache",
        ),
        (
            ReviewReason::GitUnreadable,
            "git could not list this repository",
        ),
    ] {
        let ordinary = review(reason, SizeNote::Ordinary);
        assert_eq!(ordinary.label(), "REVIEW");
        assert_eq!(ordinary.reason(), expected);
        assert_eq!(
            review(reason, SizeNote::Large).reason(),
            format!("{expected}; large (over 100 MB)")
        );
    }
}
