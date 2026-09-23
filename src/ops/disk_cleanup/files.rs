//! Purpose: Scoped cleanup filesystem access and accounting for successful deletions.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::models::{CleanupCandidate, CleanupKind, FileSize};

#[derive(Default)]
pub(super) struct CleanupOutcome {
    pub(super) freed: FileSize,
    pub(super) failures: usize,
}

impl CleanupOutcome {
    /// Count one removal; returns whether it succeeded.
    fn record(&mut self, result: io::Result<u64>) -> bool {
        match result {
            Ok(bytes) => {
                self.freed = FileSize::from_bytes(self.freed.bytes() + bytes);
                true
            }
            Err(error) => {
                self.failures += 1;
                crate::diagnostics::log_error(&format!("Cleanup removal failed: {error}"));
                false
            }
        }
    }

    fn remove_heap_dumps(&mut self, path: &Path) {
        match heap_dumps(path) {
            Ok(dumps) => {
                for dump in dumps {
                    self.record(fs::remove_file(dump.path).map(|()| dump.size.bytes()));
                }
            }
            Err(error) => {
                self.record(Err(error));
            }
        }
    }

    /// Empty `directory` of what the current user owns, keeping `directory`
    /// itself. Returns whether nothing is left in it.
    fn clear_contents(&mut self, directory: &Path) -> bool {
        let entries = match fs::read_dir(directory) {
            Ok(entries) => entries,
            Err(error) => return self.record(Err(error)),
        };
        let mut emptied = true;
        for entry in entries {
            emptied &= self.remove_owned(entry);
        }
        emptied
    }

    /// Remove one entry of a shared folder if the current user owns it: a
    /// file or link outright, a directory once emptied the same way. Returns
    /// whether it is gone.
    fn remove_owned(&mut self, entry: io::Result<fs::DirEntry>) -> bool {
        // `DirEntry::metadata` does not follow links: a link goes, its target stays.
        let (path, metadata) = match entry.and_then(|entry| Ok((entry.path(), entry.metadata()?))) {
            Ok(found) => found,
            Err(error) => return self.record(Err(error)),
        };
        if !owned_by_current_user(&metadata) {
            return false;
        }
        let kind = metadata.file_type();
        if kind.is_dir() {
            return self.clear_contents(&path) && self.record(fs::remove_dir(&path).map(|()| 0));
        }
        // Sockets and pipes are how running programs (tmux, an SSH agent) are
        // reached; they hold no data, and removing them cuts those programs off.
        (kind.is_file() || kind.is_symlink())
            && self.record(fs::remove_file(&path).map(|()| metadata.len()))
    }
}

pub(super) fn cleanup(candidates: &[CleanupCandidate]) -> CleanupOutcome {
    let mut outcome = CleanupOutcome::default();
    for candidate in candidates {
        match candidate.kind {
            CleanupKind::Path => {
                outcome.record(remove_path(&candidate.path));
            }
            CleanupKind::Contents => {
                outcome.clear_contents(&candidate.path);
            }
            CleanupKind::HeapDumps => outcome.remove_heap_dumps(&candidate.path),
        }
    }
    outcome
}

/// Whether the current user owns an entry. On Unix the temporary folder is
/// shared by every user, and another user's files there are never ours to
/// delete, even where the folder's permissions would allow it.
#[cfg(unix)]
fn owned_by_current_user(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.uid() == rustix::process::geteuid().as_raw()
}

/// Windows gives each user a temporary folder of their own, and its access
/// control already refuses to delete another user's files.
#[cfg(not(unix))]
const fn owned_by_current_user(_metadata: &fs::Metadata) -> bool {
    true
}

pub(super) fn measure(candidate: &CleanupCandidate) -> io::Result<Option<FileSize>> {
    let measured = match candidate.kind {
        CleanupKind::Path => size_of(&candidate.path).map(Some),
        CleanupKind::Contents => owned_size(&candidate.path).map(Some),
        CleanupKind::HeapDumps => heap_dumps(&candidate.path).map(|dumps| {
            let bytes = dumps.iter().map(|dump| dump.size.bytes()).sum();
            (!dumps.is_empty()).then_some(bytes)
        }),
    };
    match measured {
        Ok(bytes) => Ok(bytes.map(FileSize::from_bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

struct HeapDump {
    path: PathBuf,
    size: FileSize,
}

fn heap_dumps(path: &Path) -> io::Result<Vec<HeapDump>> {
    fs::read_dir(path)?
        .map(|entry| heap_dump(&entry?))
        .filter_map(Result::transpose)
        .collect()
}

fn heap_dump(entry: &fs::DirEntry) -> io::Result<Option<HeapDump>> {
    if !entry.file_name().to_string_lossy().ends_with(".hprof") || !entry.file_type()?.is_file() {
        return Ok(None);
    }
    Ok(Some(HeapDump {
        path: entry.path(),
        size: FileSize::from_bytes(entry.metadata()?.len()),
    }))
}

fn remove_path(path: &Path) -> io::Result<u64> {
    let size = size_of(path)?;
    if fs::symlink_metadata(path)?.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }
    Ok(size)
}

/// What emptying `directory` can free: the size of each entry the current
/// user owns in it.
fn owned_size(directory: &Path) -> io::Result<u64> {
    Ok(fs::read_dir(directory)?
        .flatten()
        .filter(|entry| {
            entry
                .metadata()
                .is_ok_and(|metadata| owned_by_current_user(&metadata))
        })
        .filter_map(|entry| size_of(&entry.path()).ok())
        .sum())
}

fn size_of(path: &Path) -> io::Result<u64> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(if metadata.is_dir() {
        crate::stats::analyse(path).total.bytes()
    } else {
        metadata.len()
    })
}
