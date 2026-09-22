//! Purpose: State for the Disk Cleanup dialog (`:cleanup`).
//!
//! A list of well-known cache directories the user can tick and then wipe in
//! one go. Sizes are measured when the dialog opens; entries that do not exist
//! on this machine are shown dimmed and start unchecked.

use std::path::PathBuf;

use super::FileSize;

/// One item in the cleanup checklist.
#[derive(Debug, Clone)]
pub struct CleanupCandidate {
    /// Short human-readable label shown in the dialog.
    pub label: &'static str,
    /// Absolute path of the directory or file to remove.
    pub path: PathBuf,
    /// `None` while still measuring or if the entry does not exist.
    pub size: Option<FileSize>,
    /// Whether the checkbox is ticked.
    pub checked: bool,
    /// Whether this entry exists on disk (false → dimmed, non-interactive).
    pub exists: bool,
}

/// Which control has the cursor inside the cleanup dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupField {
    /// The checklist row at this index.
    Item(usize),
    /// The "Clean up" action button.
    Run,
    /// The "Cancel" button.
    Cancel,
}

/// All state owned by the Disk Cleanup overlay.
#[derive(Debug, Clone)]
pub struct DiskCleanupDialog {
    pub candidates: Vec<CleanupCandidate>,
    pub field: CleanupField,
}

impl DiskCleanupDialog {
    /// Build the dialog with the default candidate list.
    /// Sizes start as `None`; call [`crate::ops::disk_cleanup::measure`] to
    /// fill them in before the overlay opens.
    #[must_use]
    pub fn new() -> Self {
        Self {
            candidates: build_candidates(),
            field: CleanupField::Item(0),
        }
    }

    /// Paths that are both checked and confirmed to exist.
    #[must_use]
    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.candidates
            .iter()
            .filter(|c| c.checked && c.exists)
            .map(|c| c.path.clone())
            .collect()
    }

    /// Total bytes across all checked, existing candidates.
    #[must_use]
    pub fn selected_bytes(&self) -> u64 {
        self.candidates
            .iter()
            .filter(|c| c.checked && c.exists)
            .filter_map(|c| c.size.map(FileSize::bytes))
            .sum()
    }
}

impl Default for DiskCleanupDialog {
    fn default() -> Self {
        Self::new()
    }
}

// ── candidate list ────────────────────────────────────────────────────────────

fn build_candidates() -> Vec<CleanupCandidate> {
    let home = home_dir();
    let [jetbrains, docker, playwright, pip] = appdata_candidates();

    vec![
        candidate("Windows Temp (%TEMP%)", temp_dir()),
        candidate(
            "Gradle caches (~/.gradle/caches)",
            home.join(".gradle").join("caches"),
        ),
        candidate(
            "npm cache (~/.npm/_cacache)",
            home.join(".npm").join("_cacache"),
        ),
        jetbrains,
        docker,
        playwright,
        candidate(
            "Cargo build artifacts (./target)",
            // Relative — ops resolves this against the active panel path.
            PathBuf::from("target"),
        ),
        candidate(
            "Gradle wrapper cache (~/.gradle/wrapper)",
            home.join(".gradle").join("wrapper"),
        ),
        pip,
        // The operations layer handles individual heap dumps, not the directory.
        candidate("JVM heap dumps (~/*.hprof)", home),
    ]
}

fn appdata_candidates() -> [CleanupCandidate; 4] {
    [
        candidate(
            "JetBrains caches (AppData/Local/JetBrains)",
            local_appdata().join("JetBrains"),
        ),
        candidate(
            "Docker WSL disk (AppData/Local/Docker)",
            local_appdata().join("Docker").join("wsl"),
        ),
        candidate(
            "Playwright browsers (~/.cache/ms-playwright)",
            local_appdata().join("ms-playwright"),
        ),
        candidate(
            "pip cache (~/.cache/pip)",
            local_appdata().join("pip").join("Cache"),
        ),
    ]
}

fn candidate(label: &'static str, path: PathBuf) -> CleanupCandidate {
    CleanupCandidate {
        label,
        path,
        size: None,
        checked: false,
        exists: false,
    }
}

fn home_dir() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

fn temp_dir() -> PathBuf {
    std::env::var_os("TEMP")
        .or_else(|| std::env::var_os("TMP"))
        .map_or_else(
            || home_dir().join("AppData").join("Local").join("Temp"),
            PathBuf::from,
        )
}

fn local_appdata() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map_or_else(|| home_dir().join("AppData").join("Local"), PathBuf::from)
}
