//! Post-order size accounting and entry descriptors for pattern deletion.

use ffm_mcp::tree_walk::directory_size;
use std::path::Path;

#[derive(Default)]
pub(super) struct DeleteRun {
    pub(super) deleted: Vec<serde_json::Value>,
    pub(super) errors: Vec<serde_json::Value>,
    pub(super) visited: usize,
}

#[derive(Clone, Copy, Default)]
pub(super) struct SubtreeSizes {
    pub(super) remaining: u64,
    pub(super) claimed: u64,
    pub(super) complete: bool,
}

impl SubtreeSizes {
    pub(super) const fn untouched(remaining: u64) -> Self {
        Self {
            remaining,
            claimed: 0,
            complete: true,
        }
    }
}

#[derive(Clone, Copy)]
pub(super) enum DeleteKind {
    Directory(Option<u64>),
    File(u64),
}

impl DeleteKind {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Directory(_) => "dir",
            Self::File(_) => "file",
        }
    }

    pub(super) fn size(self, path: &Path) -> u64 {
        match self {
            Self::Directory(Some(bytes)) => bytes,
            Self::Directory(None) => directory_size(path),
            Self::File(size) => size,
        }
    }
}
