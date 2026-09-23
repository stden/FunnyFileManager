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
    fn record(&mut self, result: io::Result<u64>) {
        match result {
            Ok(bytes) => self.freed = FileSize::from_bytes(self.freed.bytes() + bytes),
            Err(error) => {
                self.failures += 1;
                crate::diagnostics::log_error(&format!("Cleanup removal failed: {error}"));
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
            Err(error) => self.record(Err(error)),
        }
    }
}

pub(super) fn cleanup(candidates: &[CleanupCandidate]) -> CleanupOutcome {
    let mut outcome = CleanupOutcome::default();
    for candidate in candidates {
        match candidate.kind {
            CleanupKind::Path => outcome.record(remove_path(&candidate.path)),
            CleanupKind::HeapDumps => outcome.remove_heap_dumps(&candidate.path),
        }
    }
    outcome
}

pub(super) fn measure(candidate: &CleanupCandidate) -> io::Result<Option<FileSize>> {
    let measured = match candidate.kind {
        CleanupKind::Path => size_of(&candidate.path).map(Some),
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

fn size_of(path: &Path) -> io::Result<u64> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(if metadata.is_dir() {
        crate::stats::analyse(path).total.bytes()
    } else {
        metadata.len()
    })
}
