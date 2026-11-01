//! Purpose: Preserve remote Linux attributes independently of the client's OS.

use crate::models::{Attributes, DirEntry, EntryKind, FileSize, Modified};
use crate::remote::{RemoteMetadata, UnixMode};
use std::time::{Duration, UNIX_EPOCH};

pub(super) fn metadata(stat: &ssh2::FileStat) -> RemoteMetadata {
    RemoteMetadata {
        kind: kind(stat.perm.unwrap_or(0)),
        size: FileSize::from_bytes(stat.size.unwrap_or(0)),
        modified: stat
            .mtime
            .and_then(|seconds| UNIX_EPOCH.checked_add(Duration::from_secs(seconds)))
            .map_or(Modified::UNKNOWN, Modified::new),
        mode: stat.perm.map(UnixMode::from_mode),
    }
}

pub(super) fn entry(name: String, stat: &ssh2::FileStat) -> DirEntry {
    let metadata = metadata(stat);
    DirEntry {
        name,
        kind: metadata.kind,
        size: metadata.size,
        modified: metadata.modified,
        attributes: Attributes::from_bits(stat.perm.unwrap_or(0)),
        executable: stat.perm.is_some_and(|mode| mode & 0o111 != 0),
        owner: stat.uid.map(|uid| uid.to_string()),
        ..DirEntry::default()
    }
}

fn kind(mode: u32) -> EntryKind {
    match mode & 0o170_000 {
        0o040_000 => EntryKind::Directory,
        0o120_000 => EntryKind::SymlinkToFile,
        _ => EntryKind::File,
    }
}

pub(super) fn empty_stat() -> ssh2::FileStat {
    ssh2::FileStat {
        size: None,
        uid: None,
        gid: None,
        perm: None,
        atime: None,
        mtime: None,
    }
}
