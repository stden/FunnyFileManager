//! Purpose: The Space analysis verdict for one entry of a folder — KEEP, SAFE or
//! REVIEW — as a pure function of facts gathered elsewhere.
//!
//! No filesystem and no git here: the caller measures the entry and asks git,
//! and this module only judges what it is handed. That keeps the rules readable
//! in one place and testable without a disk, and it is why nothing in this file
//! can remove anything.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Link {
    Symlink,
    Plain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tracking {
    /// The entry is a tracked file, or a folder holding one.
    Tracked,
    Untracked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ignoring {
    Ignored,
    NotIgnored,
}

/// What git says about one entry of a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitFacts {
    /// The folder is in no repository, so there are no git facts at all.
    Outside,
    /// The folder lies inside a `.git` directory.
    InsideGitDir,
    /// The folder is in a repository that git could not list.
    Unreadable,
    Entry {
        tracking: Tracking,
        ignoring: Ignoring,
    },
}

/// Whether a folder has a `.git` folder or file anywhere below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holding {
    GitMetadata,
    Plain,
}

/// Everything the verdict is decided from, for one entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EntryFacts {
    pub name: String,
    pub bytes: u64,
    pub link: Link,
    pub holding: Holding,
    pub git: GitFacts,
}

/// A cache or build output a tool rebuilds on its own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Regenerable {
    CargoTarget,
    NodeModules,
    BuildOutput,
    GradleCache,
    PythonBytecode,
    PytestCache,
    MypyCache,
    RuffCache,
}

/// Why an entry is kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keep {
    GitMetadata,
    /// A folder with a repository somewhere inside, whatever git says of it.
    HoldsGit,
    Symlink,
    Tracked,
}

/// Why an entry is only worth a review.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewReason {
    NotUnderGit,
    /// Untracked and not ignored: user work that git does not know about.
    NotIgnored,
    /// Ignored by git, but its name is no known cache, so only a person can say what it is for.
    IgnoredNotCache,
    GitUnreadable,
}

/// Whether the entry is over [`LARGE_BYTES`], which a REVIEW row says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeNote {
    Ordinary,
    Large,
}

/// Entries over this size that are not safe are labelled `large`.
pub const LARGE_BYTES: u64 = 100 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Keep(Keep),
    Safe(Regenerable),
    Review {
        reason: ReviewReason,
        size: SizeNote,
    },
}

/// The verdict for one entry.
///
/// The first rule that matches wins. Git's own files, links, tracked entries and
/// folders holding a repository are kept. An unlistable repository reviews
/// everything. Outside git only the four caches are safe; inside git an untracked
/// entry is safe only when git ignores it and its name is a known cache.
#[must_use]
pub fn classify(facts: &EntryFacts) -> Verdict {
    let size = if facts.bytes > LARGE_BYTES {
        SizeNote::Large
    } else {
        SizeNote::Ordinary
    };
    let review = |reason| Verdict::Review { reason, size };
    match (facts.name.as_str(), facts.link, facts.git) {
        (".git", _, _) | (_, _, GitFacts::InsideGitDir) => Verdict::Keep(Keep::GitMetadata),
        (_, Link::Symlink, _) => Verdict::Keep(Keep::Symlink),
        _ if facts.holding == Holding::GitMetadata => Verdict::Keep(Keep::HoldsGit),
        (
            _,
            _,
            GitFacts::Entry {
                tracking: Tracking::Tracked,
                ..
            },
        ) => Verdict::Keep(Keep::Tracked),
        (_, _, GitFacts::Unreadable) => review(ReviewReason::GitUnreadable),
        (name, _, GitFacts::Outside) => match regenerable(name) {
            Some(kind) if kind.is_cache() => Verdict::Safe(kind),
            _ => review(ReviewReason::NotUnderGit),
        },
        (name, _, GitFacts::Entry { ignoring, .. }) => match (ignoring, regenerable(name)) {
            (Ignoring::Ignored, Some(kind)) => Verdict::Safe(kind),
            (Ignoring::Ignored, None) => review(ReviewReason::IgnoredNotCache),
            (Ignoring::NotIgnored, _) => review(ReviewReason::NotIgnored),
        },
    }
}

/// The regenerable kind a name is known by, matched exactly.
#[must_use]
pub fn regenerable(name: &str) -> Option<Regenerable> {
    match name {
        "target" => Some(Regenerable::CargoTarget),
        "node_modules" => Some(Regenerable::NodeModules),
        "dist" | "build" => Some(Regenerable::BuildOutput),
        ".gradle" => Some(Regenerable::GradleCache),
        "__pycache__" => Some(Regenerable::PythonBytecode),
        ".pytest_cache" => Some(Regenerable::PytestCache),
        ".mypy_cache" => Some(Regenerable::MypyCache),
        ".ruff_cache" => Some(Regenerable::RuffCache),
        _ => None,
    }
}

mod labels;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_review;
#[cfg(test)]
mod tests_safety;
