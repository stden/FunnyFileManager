//! Purpose: State for the Disk Cleanup dialog (`:cleanup`).
//!
//! A list of well-known cache directories the user can tick and then wipe in
//! one go. Sizes are measured when the dialog opens; entries that do not exist
//! on this machine are shown dimmed and start unchecked.

use std::path::PathBuf;

use super::FileSize;

/// The deletion scope chosen when a cleanup candidate is constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupKind {
    /// Remove this file or directory, including directory contents.
    Path,
    /// Empty this directory but keep it: remove only what the current user
    /// owns inside it, and leave sockets and pipes of running programs alone.
    /// For shared folders such as the temporary directory.
    Contents,
    /// Remove only regular `*.hprof` files directly inside this directory.
    HeapDumps,
}

/// One item in the cleanup checklist.
#[derive(Debug, Clone)]
pub struct CleanupCandidate {
    /// What the row cleans, shown in the dialog beside `path`. It names the
    /// tool only, so it cannot disagree with the folder actually scanned.
    pub label: &'static str,
    /// Absolute path of the directory or file to remove.
    pub path: PathBuf,
    /// Explicit scope; labels and directory names never decide what is deleted.
    pub kind: CleanupKind,
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
//
// A label names the tool, never a folder: the dialog prints each row's path
// beside its label, so the text cannot promise one folder while the row
// measures and deletes another.

fn build_candidates() -> Vec<CleanupCandidate> {
    let home = home_dir();
    let mut rows = vec![
        CleanupCandidate {
            kind: CleanupKind::Contents,
            ..candidate("Temp files", std::env::temp_dir())
        },
        candidate("Gradle caches", home.join(".gradle").join("caches")),
        candidate("npm cache", home.join(".npm").join("_cacache")),
    ];
    rows.extend(platform_caches());
    rows.extend([
        // Relative — ops resolves this against the active panel path.
        candidate("Cargo build artifacts", PathBuf::from("target")),
        candidate("Gradle wrapper cache", home.join(".gradle").join("wrapper")),
        // The operations layer handles individual heap dumps, not the directory.
        CleanupCandidate {
            kind: CleanupKind::HeapDumps,
            ..candidate("JVM heap dumps", home)
        },
    ]);
    rows
}

/// Caches that live in `%LOCALAPPDATA%` on Windows.
#[cfg(windows)]
fn platform_caches() -> Vec<CleanupCandidate> {
    let local = std::env::var_os("LOCALAPPDATA")
        .map_or_else(|| home_dir().join("AppData").join("Local"), PathBuf::from);
    vec![
        candidate("JetBrains caches", local.join("JetBrains")),
        candidate("Docker WSL disk", local.join("Docker").join("wsl")),
        candidate("Playwright browsers", local.join("ms-playwright")),
        candidate("pip cache", local.join("pip").join("Cache")),
    ]
}

/// The same tools keep their caches in the user cache folder elsewhere.
#[cfg(not(windows))]
fn platform_caches() -> Vec<CleanupCandidate> {
    let caches = cache_dir();
    vec![
        candidate("JetBrains caches", caches.join("JetBrains")),
        candidate("Playwright browsers", caches.join("ms-playwright")),
        candidate("pip cache", caches.join("pip")),
    ]
}

#[cfg(target_os = "macos")]
fn cache_dir() -> PathBuf {
    home_dir().join("Library").join("Caches")
}

/// The XDG base directory rule: `$XDG_CACHE_HOME` when it is an absolute
/// path, else `~/.cache`.
#[cfg(not(any(windows, target_os = "macos")))]
fn cache_dir() -> PathBuf {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| home_dir().join(".cache"))
}

fn candidate(label: &'static str, path: PathBuf) -> CleanupCandidate {
    CleanupCandidate {
        label,
        path,
        kind: CleanupKind::Path,
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
