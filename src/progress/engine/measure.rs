//! Purpose: Batched tree measurement for operations that count files and bytes
//! before or during cross-filesystem transfers, without following symlinks.

use std::fs;
use std::path::Path;

use super::super::Operation;
use super::report::entering;
use super::update;

const MEASURE_BATCH: u64 = 64;

#[cfg(test)]
thread_local! {
    pub(in crate::progress) static FORCE_MEASURE_ENTRY_ERROR: std::cell::Cell<bool> =
        const { std::cell::Cell::new(false) };
}

/// Add what is under `root` to the operation's total, without following links.
pub(super) fn measure(root: &Path, operation: Operation) {
    let Ok(metadata) = fs::symlink_metadata(root) else {
        return;
    };
    if !metadata.is_dir() {
        let bytes = file_bytes(metadata.file_type(), metadata.len());
        found_batch(Some(root), 1, bytes);
        return;
    }
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        entering(&folder);
        // Unreadable folders are the operation's to report; the count only
        // comes out short.
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        measure_folder(entries, operation, &mut folders);
    }
}

fn measure_folder(
    entries: fs::ReadDir,
    operation: Operation,
    folders: &mut Vec<std::path::PathBuf>,
) {
    let mut files = 0_u64;
    let mut bytes = 0_u64;
    for entry in entries.flatten() {
        let Ok((kind, size)) = measure_entry(&entry, operation) else {
            continue;
        };
        if kind.is_dir() {
            folders.push(entry.path());
            continue;
        }
        files += 1;
        bytes = bytes.saturating_add(file_bytes(kind, size));
        if files >= MEASURE_BATCH {
            found_batch(None, files, bytes);
            files = 0;
            bytes = 0;
        }
    }
    if files > 0 {
        found_batch(None, files, bytes);
    }
}

/// A delete only needs entry types, usually available without a metadata lookup.
fn measure_entry(
    entry: &fs::DirEntry,
    operation: Operation,
) -> std::io::Result<(fs::FileType, u64)> {
    #[cfg(test)]
    if FORCE_MEASURE_ENTRY_ERROR.get() {
        return Err(std::io::Error::other("injected measure_entry error"));
    }
    if operation == Operation::Delete {
        return Ok((entry.file_type()?, 0));
    }
    let metadata = entry.metadata()?;
    Ok((metadata.file_type(), metadata.len()))
}

#[expect(
    clippy::filetype_is_file,
    reason = "only regular files contribute bytes; links and special entries still count as files"
)]
fn file_bytes(kind: fs::FileType, size: u64) -> u64 {
    if kind.is_file() {
        size
    } else {
        0
    }
}

fn found_batch(path: Option<&Path>, files: u64, bytes: u64) {
    update(|shown| {
        if let Some(path) = path {
            shown.set_current(path);
        }
        if shown.operation != Operation::Move {
            shown.total.files = shown.total.files.saturating_add(files);
        }
        if shown.operation != Operation::Delete {
            shown.total.bytes = shown.total.bytes.saturating_add(bytes);
        }
    });
}
