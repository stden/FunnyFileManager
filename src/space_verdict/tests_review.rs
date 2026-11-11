//! Unit tests: an untracked entry is reviewed, not safe; outside git only the
//! four caches are safe; a large entry says so; an unreadable repository makes
//! nothing safe.

use super::tests::{facts, review, tracked, IGNORED, KEPT};
use super::*;

#[test]
fn untracked_entries_git_does_not_ignore_are_review() {
    assert_eq!(
        classify(&facts("notes.md", KEPT)),
        review(ReviewReason::NotIgnored, SizeNote::Ordinary)
    );
    // A regenerable name that git does not ignore is user work until shown otherwise.
    assert_eq!(
        classify(&facts("target", KEPT)),
        review(ReviewReason::NotIgnored, SizeNote::Ordinary)
    );
}

#[test]
fn ignored_entries_that_are_no_known_cache_are_review_not_safe() {
    for name in ["debug.log", "secrets.env", "output"] {
        assert_eq!(
            classify(&facts(name, IGNORED)),
            review(ReviewReason::IgnoredNotCache, SizeNote::Ordinary),
            "{name}"
        );
    }
}

#[test]
fn outside_git_only_the_four_caches_are_safe() {
    let caches = [
        ("__pycache__", Regenerable::PythonBytecode),
        (".pytest_cache", Regenerable::PytestCache),
        (".mypy_cache", Regenerable::MypyCache),
        (".ruff_cache", Regenerable::RuffCache),
    ];
    for (name, kind) in caches {
        assert_eq!(
            classify(&facts(name, GitFacts::Outside)),
            Verdict::Safe(kind),
            "{name}"
        );
    }
    for name in ["target", "node_modules", "dist", ".gradle", "notes.md"] {
        assert_eq!(
            classify(&facts(name, GitFacts::Outside)),
            review(ReviewReason::NotUnderGit, SizeNote::Ordinary),
            "{name}"
        );
    }
}

#[test]
fn entries_over_the_large_threshold_that_are_not_safe_say_large() {
    let big = |name: &str, git| EntryFacts {
        bytes: LARGE_BYTES + 1,
        ..facts(name, git)
    };
    assert_eq!(
        classify(&big("archive.iso", KEPT)),
        review(ReviewReason::NotIgnored, SizeNote::Large)
    );
    assert_eq!(
        classify(&big("disk.img", GitFacts::Outside)),
        review(ReviewReason::NotUnderGit, SizeNote::Large)
    );
    // The label is for what is not safe: a large safe folder stays SAFE, a large kept one stays KEEP.
    assert_eq!(
        classify(&big("target", IGNORED)),
        Verdict::Safe(Regenerable::CargoTarget)
    );
    assert_eq!(
        classify(&big("src", tracked(Ignoring::NotIgnored))),
        Verdict::Keep(Keep::Tracked)
    );
    // Exactly the threshold is not over it.
    let at_threshold = EntryFacts {
        bytes: LARGE_BYTES,
        ..facts("archive.iso", KEPT)
    };
    assert_eq!(
        classify(&at_threshold),
        review(ReviewReason::NotIgnored, SizeNote::Ordinary)
    );
}

#[test]
fn an_unreadable_repository_makes_nothing_safe() {
    for name in ["target", "__pycache__", "notes.md"] {
        assert_eq!(
            classify(&facts(name, GitFacts::Unreadable)),
            review(ReviewReason::GitUnreadable, SizeNote::Ordinary),
            "{name}"
        );
    }
}
