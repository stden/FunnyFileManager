//! Purpose: The reports a copy, move, delete or search makes while it runs, and the
//! measuring that feeds the dialog's totals.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use super::super::{Operation, Phase, Snapshot};
use super::{update, with_screen, Run, Shown};

/// Start showing `operation` on `sources`. A copy or a delete measures them
/// first, reporting as it goes, so the dialog can say how far along it is.
pub fn begin(operation: Operation, sources: &[PathBuf]) -> Run {
    let started = with_screen(|screen| {
        if screen.shown.is_some() {
            return false;
        }
        screen.shown = Some(Shown::new(operation, sources, Instant::now()));
        true
    })
    .unwrap_or(false);
    if started && operation.scans_first() {
        for source in sources {
            measure(source, operation);
        }
        update(|shown| shown.phase = Phase::Working);
    }
    Run { owns: started }
}

/// The operation is at `path` now.
pub fn entering(path: &Path) {
    update(|shown| shown.set_current(path));
}

/// A copy wrote `bytes` more.
pub fn copied(bytes: u64) {
    update(|shown| {
        if shown.operation != Operation::Delete {
            shown.done.bytes = shown.done.bytes.saturating_add(bytes);
        }
    });
}

/// A copy finished one file or link.
pub fn file_copied() {
    update(|shown| {
        if shown.operation == Operation::Copy {
            shown.done.files = shown.done.files.saturating_add(1);
        }
    });
}

/// `path` was removed: one file less for a delete, and a move's source
/// going for a move across filesystems.
pub fn removed(path: &Path) {
    update(|shown| {
        shown.set_current(path);
        if shown.operation == Operation::Delete {
            shown.done.files = shown.done.files.saturating_add(1);
        }
    });
}

/// A move has to copy `source` across filesystems: its bytes join what the
/// dialog counts, so a big folder moved to another disk fills the bar as
/// it is copied.
pub fn copying_across(source: &Path) {
    if !is_shown() {
        return;
    }
    measure(source, Operation::Move);
}

/// One of the selected entries is done. A move counts them, since a rename
/// moves a whole folder in one step; a copy and a delete count every file.
pub fn source_done() {
    update(|shown| {
        if matches!(shown.operation, Operation::Move | Operation::GitPull) {
            shown.done.files = shown.done.files.saturating_add(1);
        }
    });
}

/// A search looked at one more entry.
pub fn examined() {
    update(|shown| shown.total.files = shown.total.files.saturating_add(1));
}

/// A search found one more match.
pub fn matched() {
    update(|shown| shown.done.files = shown.done.files.saturating_add(1));
}

/// Nothing changed, but the work goes on: the dialog's clock still moves
/// while one big file is read for a search's text.
pub fn tick() {
    update(|_| ());
}

/// Draw `snapshot`, which another process sent: the administrator helper
/// measures and throttles its own operation, and this one only shows it.
pub fn relay(snapshot: &Snapshot) {
    with_screen(|screen| {
        screen.painter.paint(snapshot);
        screen.painted = true;
    });
}

/// Whether an operation is being shown on this thread.
#[must_use]
pub fn is_shown() -> bool {
    with_screen(|screen| screen.shown.is_some()).unwrap_or(false)
}

/// Add what is under `root` to the operation's total, without following a
/// link: see [`found`].
fn measure(root: &Path, operation: Operation) {
    let mut folders = vec![root.to_path_buf()];
    let Ok(metadata) = fs::symlink_metadata(root) else {
        return;
    };
    if !metadata.is_dir() {
        found(Some(root), metadata.file_type(), metadata.len());
        return;
    }
    while let Some(folder) = folders.pop() {
        entering(&folder);
        // Unreadable folders are the operation's to report; the count only
        // comes out short.
        let Ok(entries) = fs::read_dir(&folder) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok((kind, size)) = measure_entry(&entry, operation) else {
                continue;
            };
            if kind.is_dir() {
                folders.push(entry.path());
            } else {
                found(None, kind, size);
            }
        }
    }
}

/// A delete only needs entry types, usually available without a metadata lookup.
fn measure_entry(
    entry: &fs::DirEntry,
    operation: Operation,
) -> std::io::Result<(fs::FileType, u64)> {
    if operation == Operation::Delete {
        return Ok((entry.file_type()?, 0));
    }
    let metadata = entry.metadata()?;
    Ok((metadata.file_type(), metadata.len()))
}

/// Count one file or link found by [`measure`]: as a file to do for a copy
/// or a delete, whose bar counts them, and by the bytes a copy or a move
/// across filesystems will read. A move's files are already counted by the
/// entries selected; see [`source_done`].
#[expect(
    clippy::filetype_is_file,
    reason = "only regular files contribute bytes; links and special entries still count as files"
)]
fn found(path: Option<&Path>, kind: fs::FileType, size: u64) {
    let size = if kind.is_file() { size } else { 0 };
    update(|shown| {
        if let Some(path) = path {
            shown.set_current(path);
        }
        if shown.operation != Operation::Move {
            shown.total.files = shown.total.files.saturating_add(1);
        }
        if shown.operation != Operation::Delete {
            shown.total.bytes = shown.total.bytes.saturating_add(size);
        }
    });
}
