//! Purpose: The words a verdict shows — its label, and the reason after it — and
//! what each kind of regenerable entry is said to be rebuilt by. Text only.

use super::{Keep, Regenerable, ReviewReason, SizeNote, Verdict};

impl Regenerable {
    /// Whether this is a cache that is safe even outside git.
    #[must_use]
    pub const fn is_cache(self) -> bool {
        matches!(
            self,
            Self::PythonBytecode | Self::PytestCache | Self::MypyCache | Self::RuffCache
        )
    }

    /// What the row says about this kind.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::CargoTarget => "Cargo build output, rebuilt by cargo build",
            Self::NodeModules => "installed packages, rebuilt by the package manager",
            Self::BuildOutput => "build output, rebuilt by the build",
            Self::GradleCache => "Gradle cache, rebuilt by Gradle",
            Self::PythonBytecode => "Python bytecode cache, rebuilt on import",
            Self::PytestCache => "pytest cache, rebuilt by pytest",
            Self::MypyCache => "mypy cache, rebuilt by mypy",
            Self::RuffCache => "ruff cache, rebuilt by ruff",
        }
    }
}

impl Verdict {
    /// The word a row shows: `KEEP`, `SAFE` or `REVIEW`.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Keep(_) => "KEEP",
            Self::Safe(_) => "SAFE",
            Self::Review { .. } => "REVIEW",
        }
    }

    /// The short reason a row shows after the label.
    #[must_use]
    pub fn reason(self) -> String {
        match self {
            Self::Keep(Keep::GitMetadata) => "git's own files".to_owned(),
            Self::Keep(Keep::HoldsGit) => "holds a .git folder".to_owned(),
            Self::Keep(Keep::Symlink) => "a link, never followed".to_owned(),
            Self::Keep(Keep::Tracked) => "tracked by git".to_owned(),
            Self::Safe(kind) => kind.reason().to_owned(),
            Self::Review { reason, size } => {
                let text = match reason {
                    ReviewReason::NotUnderGit => "not under git",
                    ReviewReason::NotIgnored => "not ignored by git",
                    ReviewReason::IgnoredNotCache => "ignored by git, not a known cache",
                    ReviewReason::GitUnreadable => "git could not list this repository",
                };
                match size {
                    SizeNote::Ordinary => text.to_owned(),
                    SizeNote::Large => format!("{text}; large (over 100 MB)"),
                }
            }
        }
    }
}
