//! Purpose: State for the Disk Cleanup dialog (`:cleanup`).

use std::path::{Path, PathBuf};

use super::FileSize;

/// The deletion scope chosen when a cleanup candidate is constructed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupKind {
    Path,
    Contents,
    HeapDumps,
}

/// Availability and measurement state of a cleanup candidate on disk.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum CandidateState {
    #[default]
    Absent,
    Present {
        size: Option<FileSize>,
        checked: bool,
    },
}

/// One item in the cleanup checklist.
#[derive(Debug, Clone)]
pub struct CleanupCandidate {
    pub label: &'static str,
    pub path: PathBuf,
    pub kind: CleanupKind,
    pub state: CandidateState,
}

impl CleanupCandidate {
    #[must_use]
    pub const fn exists(&self) -> bool {
        matches!(self.state, CandidateState::Present { .. })
    }

    #[must_use]
    pub const fn is_checked(&self) -> bool {
        matches!(self.state, CandidateState::Present { checked: true, .. })
    }

    #[must_use]
    pub const fn size(&self) -> Option<FileSize> {
        match self.state {
            CandidateState::Present { size, .. } => size,
            CandidateState::Absent => None,
        }
    }

    pub const fn toggle_checked(&mut self) {
        if let CandidateState::Present { checked, .. } = &mut self.state {
            *checked = !*checked;
        }
    }
}

/// Which control has the cursor inside the cleanup dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupField {
    Item(usize),
    Run,
    Cancel,
}

/// All state owned by the Disk Cleanup overlay.
#[derive(Debug, Clone)]
pub struct DiskCleanupDialog {
    pub candidates: Vec<CleanupCandidate>,
    pub field: CleanupField,
}

impl DiskCleanupDialog {
    #[must_use]
    pub fn new() -> Self {
        Self {
            candidates: build_candidates(),
            field: CleanupField::Item(0),
        }
    }

    #[must_use]
    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.candidates
            .iter()
            .filter(|c| c.is_checked())
            .map(|c| c.path.clone())
            .collect()
    }

    #[must_use]
    pub fn selected_bytes(&self) -> u64 {
        self.candidates
            .iter()
            .filter(|c| c.is_checked())
            .filter_map(CleanupCandidate::size)
            .map(FileSize::bytes)
            .sum()
    }
}

impl Default for DiskCleanupDialog {
    fn default() -> Self {
        Self::new()
    }
}

fn build_candidates() -> Vec<CleanupCandidate> {
    let home = crate::config::user_home();
    let mut rows = vec![CleanupCandidate {
        kind: CleanupKind::Contents,
        ..candidate("Temp files", std::env::temp_dir())
    }];
    if let Some(home) = &home {
        rows.extend([
            candidate("Gradle caches", home.join(".gradle").join("caches")),
            candidate("npm cache", home.join(".npm").join("_cacache")),
        ]);
    }
    rows.extend(platform_caches(home.as_deref()));
    rows.push(candidate("Cargo build artifacts", PathBuf::from("target")));
    if let Some(home) = home {
        rows.extend([
            candidate("Gradle wrapper cache", home.join(".gradle").join("wrapper")),
            CleanupCandidate {
                kind: CleanupKind::HeapDumps,
                ..candidate("JVM heap dumps", home)
            },
        ]);
    }
    rows
}

#[cfg(windows)]
fn platform_caches(home: Option<&Path>) -> Vec<CleanupCandidate> {
    let Some(local) = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| Some(home?.join("AppData").join("Local")))
    else {
        return Vec::new();
    };
    vec![
        candidate("JetBrains caches", local.join("JetBrains")),
        candidate("Docker WSL disk", local.join("Docker").join("wsl")),
        candidate("Playwright browsers", local.join("ms-playwright")),
        candidate("pip cache", local.join("pip").join("Cache")),
    ]
}

#[cfg(not(windows))]
fn platform_caches(home: Option<&Path>) -> Vec<CleanupCandidate> {
    let Some(caches) = cache_dir(home) else {
        return Vec::new();
    };
    vec![
        candidate("JetBrains caches", caches.join("JetBrains")),
        candidate("Playwright browsers", caches.join("ms-playwright")),
        candidate("pip cache", caches.join("pip")),
    ]
}

#[cfg(target_os = "macos")]
fn cache_dir(home: Option<&Path>) -> Option<PathBuf> {
    Some(home?.join("Library").join("Caches"))
}

#[cfg(not(any(windows, target_os = "macos")))]
fn cache_dir(home: Option<&Path>) -> Option<PathBuf> {
    std::env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| Some(home?.join(".cache")))
}

fn candidate(label: &'static str, path: PathBuf) -> CleanupCandidate {
    CleanupCandidate {
        label,
        path,
        kind: CleanupKind::Path,
        state: CandidateState::Absent,
    }
}

#[cfg(test)]
mod tests;
