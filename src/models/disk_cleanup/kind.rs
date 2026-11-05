//! Purpose: What a cleanup row removes under its folder, and whether the row
//! starts ticked.

/// The deletion scope chosen when a cleanup candidate is constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupKind {
    /// The folder itself, with everything in it.
    Path,
    /// What the current user owns inside the folder; the folder stays.
    Contents,
    /// `*.hprof` files directly inside the folder.
    HeapDumps,
    /// The files or folders one [`TempScope`] names under the folder.
    Temp(TempScope),
}

impl CleanupKind {
    /// Whether the row starts ticked. What the row removes decides it:
    /// temporary files cost nothing to delete, while a cache that a tool
    /// fetches or rebuilds starts unticked.
    #[must_use]
    pub const fn starts_checked(self) -> bool {
        match self {
            Self::Contents | Self::HeapDumps => true,
            Self::Path => false,
            Self::Temp(scope) => scope.is_temporary(),
        }
    }
}

/// The names a `Temp` row matches under its folder. Each names one tool's
/// temporary output, so the row never removes what sits beside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempScope {
    /// `*.lastUpdated` and `resolver-status.properties` anywhere under the
    /// Maven repository: markers of an unfinished download. Artifacts and
    /// `_remote.repositories` stay.
    MavenPartialDownloads,
    /// `*.log` files directly in each Gradle version folder (`8.11.1`) under
    /// the daemon folder. The registry and the running daemon's state stay.
    GradleDaemonLogs,
    /// `transforms-*` folders directly under Gradle's caches, which Gradle
    /// rebuilds on the next build.
    GradleTransforms,
}

impl TempScope {
    /// The pattern the row shows after its folder.
    #[must_use]
    pub const fn pattern(self) -> &'static str {
        match self {
            Self::MavenPartialDownloads => "**/*.lastUpdated, **/resolver-status.properties",
            Self::GradleDaemonLogs => "*/*.log",
            Self::GradleTransforms => "transforms-*",
        }
    }

    /// Temporary output starts ticked; a folder Gradle rebuilds does not.
    #[must_use]
    pub const fn is_temporary(self) -> bool {
        !matches!(self, Self::GradleTransforms)
    }

    /// Whether the scope matches folders, which go whole. The others match
    /// files, and a file pattern never removes a folder.
    #[must_use]
    pub const fn names_folders(self) -> bool {
        matches!(self, Self::GradleTransforms)
    }
}
