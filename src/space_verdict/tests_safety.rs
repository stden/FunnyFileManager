//! Unit tests: the labels and reasons a row shows, the regenerable names, and
//! safety over every combination of facts.

use super::tests::{review, tracked, IGNORED, KEPT};
use super::*;

#[test]
fn labels_and_reasons_say_what_the_verdict_means() {
    assert_eq!(Verdict::Keep(Keep::Tracked).label(), "KEEP");
    assert_eq!(Verdict::Safe(Regenerable::CargoTarget).label(), "SAFE");
    assert_eq!(
        review(ReviewReason::NotIgnored, SizeNote::Ordinary).label(),
        "REVIEW"
    );
    assert!(Verdict::Safe(Regenerable::CargoTarget)
        .reason()
        .contains("rebuilt by cargo build"));
    assert_eq!(
        review(ReviewReason::NotUnderGit, SizeNote::Ordinary).reason(),
        "not under git"
    );
    assert!(review(ReviewReason::NotIgnored, SizeNote::Large)
        .reason()
        .contains("large"));
}

#[test]
fn regenerable_names_match_exactly() {
    assert_eq!(regenerable("target"), Some(Regenerable::CargoTarget));
    assert_eq!(regenerable("Target"), None, "case counts");
    assert_eq!(regenerable("targets"), None);
    assert_eq!(regenerable("notes"), None);
}

/// Names of every kind the rules tell apart: regenerable, plain, git's own.
const NAMES: [&str; 12] = [
    "target",
    "node_modules",
    "dist",
    "build",
    "__pycache__",
    ".pytest_cache",
    ".mypy_cache",
    ".ruff_cache",
    ".gradle",
    "src",
    ".git",
    "notes.md",
];

/// Every git state an entry can be in.
fn git_states() -> [GitFacts; 7] {
    [
        GitFacts::Outside,
        GitFacts::InsideGitDir,
        GitFacts::Unreadable,
        tracked(Ignoring::Ignored),
        tracked(Ignoring::NotIgnored),
        IGNORED,
        KEPT,
    ]
}

/// Every combination of the facts the verdict reads, for each name.
fn every_entry() -> Vec<EntryFacts> {
    NAMES
        .into_iter()
        .flat_map(|name| git_states().into_iter().map(move |git| (name, git)))
        .flat_map(|(name, git)| [Link::Plain, Link::Symlink].map(move |link| (name, git, link)))
        .flat_map(|(name, git, link)| {
            [Holding::Plain, Holding::GitMetadata].map(move |holding| (name, git, link, holding))
        })
        .flat_map(|(name, git, link, holding)| {
            [1, LARGE_BYTES + 1].map(move |bytes| EntryFacts {
                name: name.to_owned(),
                bytes,
                link,
                holding,
                git,
            })
        })
        .collect()
}

/// Safety over every combination of facts: a tracked entry is never SAFE, and
/// SAFE needs either a cache outside git or an untracked ignored entry inside
/// one. A symlink, and anything holding a repository, is never SAFE either.
#[test]
fn safe_only_where_git_agrees_or_the_entry_is_a_cache_outside_git() {
    for entry in every_entry() {
        let verdict = classify(&entry);
        let Verdict::Safe(kind) = verdict else {
            continue;
        };
        let outside_cache = entry.git == GitFacts::Outside && kind.is_cache();
        assert!(
            entry.link == Link::Plain
                && entry.holding == Holding::Plain
                && (outside_cache || entry.git == IGNORED),
            "{entry:?} judged {verdict:?}"
        );
    }
}
