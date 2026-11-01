//! Unit tests: the Space analysis verdict rules, as pure functions of facts.

use super::*;

pub(super) const IGNORED: GitFacts = GitFacts::Entry {
    tracking: Tracking::Untracked,
    ignoring: Ignoring::Ignored,
};
pub(super) const KEPT: GitFacts = GitFacts::Entry {
    tracking: Tracking::Untracked,
    ignoring: Ignoring::NotIgnored,
};

pub(super) fn facts(name: &str, git: GitFacts) -> EntryFacts {
    EntryFacts {
        name: name.to_owned(),
        bytes: 1,
        link: Link::Plain,
        holding: Holding::Plain,
        git,
    }
}

/// An entry with a `.git` folder somewhere below it.
pub(super) fn nested_repo(name: &str, git: GitFacts) -> EntryFacts {
    EntryFacts {
        holding: Holding::GitMetadata,
        ..facts(name, git)
    }
}

pub(super) fn tracked(ignoring: Ignoring) -> GitFacts {
    GitFacts::Entry {
        tracking: Tracking::Tracked,
        ignoring,
    }
}

pub(super) fn review(reason: ReviewReason, size: SizeNote) -> Verdict {
    Verdict::Review { reason, size }
}

#[test]
fn git_metadata_is_kept_by_name_and_inside_a_git_dir() {
    assert_eq!(
        classify(&facts(".git", KEPT)),
        Verdict::Keep(Keep::GitMetadata)
    );
    assert_eq!(
        classify(&facts("objects", GitFacts::InsideGitDir)),
        Verdict::Keep(Keep::GitMetadata)
    );
}

#[test]
fn tracked_entries_are_kept_even_when_git_ignores_them() {
    assert_eq!(
        classify(&facts("src", tracked(Ignoring::NotIgnored))),
        Verdict::Keep(Keep::Tracked)
    );
    // An ignored folder that still holds a tracked file is kept.
    assert_eq!(
        classify(&facts("target", tracked(Ignoring::Ignored))),
        Verdict::Keep(Keep::Tracked)
    );
}

/// Git metadata anywhere below an entry keeps the entry, even one git ignores:
/// a vendored repository inside `target/` is still somebody's repository.
#[test]
fn an_entry_holding_a_git_directory_is_kept_even_when_ignored() {
    assert_eq!(
        classify(&nested_repo("target", IGNORED)),
        Verdict::Keep(Keep::HoldsGit)
    );
    assert_eq!(
        classify(&nested_repo("node_modules", IGNORED)),
        Verdict::Keep(Keep::HoldsGit)
    );
}

#[test]
fn symlinks_are_kept_and_never_followed() {
    let link = EntryFacts {
        link: Link::Symlink,
        ..facts("target", IGNORED)
    };
    assert_eq!(classify(&link), Verdict::Keep(Keep::Symlink));
}

#[test]
fn ignored_regenerable_folders_are_safe_and_name_their_kind() {
    let cases = [
        ("target", Regenerable::CargoTarget),
        ("node_modules", Regenerable::NodeModules),
        ("dist", Regenerable::BuildOutput),
        ("build", Regenerable::BuildOutput),
        ("__pycache__", Regenerable::PythonBytecode),
        (".pytest_cache", Regenerable::PytestCache),
        (".mypy_cache", Regenerable::MypyCache),
        (".ruff_cache", Regenerable::RuffCache),
        (".gradle", Regenerable::GradleCache),
    ];
    for (name, kind) in cases {
        assert_eq!(
            classify(&facts(name, IGNORED)),
            Verdict::Safe(kind),
            "{name}"
        );
    }
}
