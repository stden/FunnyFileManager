//! Purpose: An in-memory remote filesystem for tests — the trait the remote
//! panel is written against, over a map instead of a host.
//!
//! Kept deliberately literal — files and directories, no links, no
//! permissions — because what the remote panel's tests check is the panel.

use std::collections::BTreeMap;

use crate::models::{DirEntry, EntryKind, FileSize, RemotePath};

use super::{RemoteError, RemoteFs};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Node {
    Dir,
    File(u64),
}

/// Every path and what is at it; the root always exists.
#[derive(Debug, Clone, Default)]
pub struct FakeRemoteFs {
    nodes: BTreeMap<RemotePath, Node>,
}

impl FakeRemoteFs {
    /// A host holding a file of `size` bytes at every path in `files`, with
    /// the directories above each. A path that does not parse is left out.
    #[must_use]
    pub fn with_files(files: &[(&str, u64)]) -> Self {
        let mut fs = Self::default();
        for (text, size) in files {
            if let Ok(path) = RemotePath::parse(text) {
                fs.add_directories_above(&path);
                fs.nodes.insert(path, Node::File(*size));
            }
        }
        fs
    }

    fn add_directories_above(&mut self, path: &RemotePath) {
        let mut dir = path.parent();
        while let Some(above) = dir.filter(|above| !above.is_root()) {
            dir = above.parent();
            self.nodes.insert(above, Node::Dir);
        }
    }

    fn node(&self, path: &RemotePath) -> Option<&Node> {
        if path.is_root() {
            return Some(&Node::Dir);
        }
        self.nodes.get(path)
    }
}

impl RemoteFs for FakeRemoteFs {
    fn list(&mut self, dir: &RemotePath) -> Result<Vec<DirEntry>, RemoteError> {
        match self.node(dir) {
            None => return Err(RemoteError::NotFound(dir.clone())),
            Some(Node::File(_)) => return Err(RemoteError::NotADirectory(dir.clone())),
            Some(Node::Dir) => {}
        }
        Ok(self
            .nodes
            .iter()
            .filter(|(path, _)| path.parent().as_ref() == Some(dir))
            .map(|(path, node)| entry(path, node))
            .collect())
    }
}

/// The listing row for `node` at `path`.
fn entry(path: &RemotePath, node: &Node) -> DirEntry {
    let name = path
        .as_str()
        .rsplit('/')
        .next()
        .unwrap_or_default()
        .to_owned();
    let (kind, size) = match node {
        Node::Dir => (EntryKind::Directory, FileSize::default()),
        Node::File(size) => (EntryKind::File, FileSize::from_bytes(*size)),
    };
    DirEntry {
        name,
        kind,
        size,
        ..DirEntry::default()
    }
}

#[cfg(test)]
mod tests;
