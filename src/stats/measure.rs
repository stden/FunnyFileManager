//! Purpose: Measure each immediate entry of a folder for Space analysis — a
//! folder's whole tree through the same walk `analyse` uses, a file's length,
//! and nothing for a link or a special file, which are never followed.

use std::fs;
use std::io;
use std::path::Path;

use super::walk::{walk, Tally};
use super::Budget;

/// What an immediate entry is, as far as its size goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shape {
    Folder,
    File,
    Link,
    Special,
}

/// One immediate entry of a folder, measured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measured {
    pub name: String,
    pub shape: Shape,
    /// Bytes under the entry; for a folder, only what the budget let it walk.
    pub bytes: u64,
    /// Folders below the entry that could not be listed.
    pub unread: usize,
    /// Entries named `.git` below the entry, as far as the budget let it walk.
    pub git_entries: usize,
}

/// Every immediate entry of `directory`, all spending from one `budget`.
///
/// Fails only when `directory` itself cannot be listed.
pub fn measure_entries(directory: &Path, budget: &mut Budget) -> io::Result<Vec<Measured>> {
    let mut measured = Vec::new();
    for entry in fs::read_dir(directory)?.flatten() {
        if !budget.spend() {
            break;
        }
        // `metadata` on a `DirEntry` does not follow links, as in the walk.
        let Ok(meta) = entry.metadata() else { continue };
        let name = entry.file_name().to_string_lossy().into_owned();
        measured.push(measure_one(&entry.path(), name, &meta, budget));
    }
    Ok(measured)
}

fn measure_one(path: &Path, name: String, meta: &fs::Metadata, budget: &mut Budget) -> Measured {
    if meta.is_dir() {
        let mut tally = Tally::default();
        walk(path, &name, &mut tally, budget);
        return Measured {
            name,
            shape: Shape::Folder,
            bytes: tally.bytes(),
            unread: tally.unread,
            git_entries: tally.git_entries,
        };
    }
    let (shape, bytes) = if meta.is_file() {
        (Shape::File, meta.len())
    } else if meta.is_symlink() {
        (Shape::Link, 0)
    } else {
        (Shape::Special, 0)
    };
    Measured {
        name,
        shape,
        bytes,
        unread: 0,
        git_entries: 0,
    }
}
