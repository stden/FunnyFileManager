//! Purpose: Mutating operations on the in-memory test host.

use super::{entry, FakeRemoteFs, Node, RemoteError, RemoteFs};
use crate::models::{DirEntry, RemotePath};
use crate::remote::{RemoteMetadata, UnixMode, WriteMode};
use std::io::{Read, Write};

impl RemoteFs for FakeRemoteFs {
    fn list(&mut self, dir: &RemotePath) -> Result<Vec<DirEntry>, RemoteError> {
        self.require_directory(dir)?;
        Ok(self
            .nodes
            .iter()
            .filter(|(path, _)| path.parent().as_ref() == Some(dir))
            .map(|(path, node)| entry(path, node))
            .collect())
    }

    fn stat(&mut self, path: &RemotePath) -> Result<RemoteMetadata, RemoteError> {
        let node = self
            .node(path)
            .ok_or_else(|| RemoteError::NotFound(path.clone()))?;
        let mut metadata = RemoteMetadata::from_entry(&entry(path, node));
        metadata.mode = Some(self.modes.get(path).copied().unwrap_or_else(|| {
            UnixMode::from_mode(if metadata.kind.is_dir() { 0o755 } else { 0o644 })
        }));
        Ok(metadata)
    }

    fn download(&mut self, path: &RemotePath, output: &mut dyn Write) -> Result<u64, RemoteError> {
        let Some(Node::File(size)) = self.node(path) else {
            return Err(RemoteError::NotFound(path.clone()));
        };
        if let Some(bytes) = self.contents.get(path) {
            output.write_all(bytes)?;
            return Ok(*size);
        }
        Ok(std::io::copy(&mut std::io::repeat(0).take(*size), output)?)
    }

    fn upload(
        &mut self,
        path: &RemotePath,
        input: &mut dyn Read,
        mode: WriteMode,
    ) -> Result<u64, RemoteError> {
        self.require_parent(path)?;
        if self.node(path).is_some() && mode == WriteMode::CreateNew {
            return Err(RemoteError::AlreadyExists(path.clone()));
        }
        if matches!(self.node(path), Some(Node::Dir)) {
            return Err(RemoteError::Failed(
                "cannot overwrite a directory with a file",
            ));
        }
        let mut bytes = Vec::new();
        let size = input.read_to_end(&mut bytes)?;
        let size =
            u64::try_from(size).map_err(|_protocol_error| RemoteError::Failed("file too large"))?;
        self.nodes.insert(path.clone(), Node::File(size));
        self.contents.insert(path.clone(), bytes);
        Ok(size)
    }

    fn mkdir(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        self.require_parent(path)?;
        if self.node(path).is_some() {
            return Err(RemoteError::AlreadyExists(path.clone()));
        }
        self.nodes.insert(path.clone(), Node::Dir);
        Ok(())
    }

    fn remove_file(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        if !matches!(self.node(path), Some(Node::File(_))) {
            return Err(RemoteError::NotFound(path.clone()));
        }
        self.remove_node(path);
        Ok(())
    }

    fn rmdir(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        self.require_directory(path)?;
        if path.is_root()
            || self
                .nodes
                .keys()
                .any(|child| child.parent().as_ref() == Some(path))
        {
            return Err(RemoteError::Failed("directory is not empty"));
        }
        self.remove_node(path);
        Ok(())
    }

    fn rename(&mut self, from: &RemotePath, to: &RemotePath) -> Result<(), RemoteError> {
        self.require_parent(to)?;
        if self.node(to).is_some() {
            return Err(RemoteError::AlreadyExists(to.clone()));
        }
        if from.is_root() || to.as_str().starts_with(&format!("{from}/")) {
            return Err(RemoteError::InvalidInput(
                "cannot move a directory into itself",
            ));
        }
        self.node(from)
            .ok_or_else(|| RemoteError::NotFound(from.clone()))?;
        let old_paths: Vec<_> = self
            .nodes
            .keys()
            .filter(|path| *path == from || path.as_str().starts_with(&format!("{from}/")))
            .cloned()
            .collect();
        for old in old_paths {
            let suffix = old.as_str().strip_prefix(from.as_str()).unwrap_or_default();
            let new = RemotePath::parse(&format!("{to}{suffix}")).map_err(|_protocol_error| {
                RemoteError::InvalidInput("invalid rename destination")
            })?;
            self.move_node(&old, new);
        }
        Ok(())
    }

    fn chmod(&mut self, path: &RemotePath, mode: UnixMode) -> Result<(), RemoteError> {
        self.node(path)
            .ok_or_else(|| RemoteError::NotFound(path.clone()))?;
        self.modes.insert(path.clone(), mode);
        Ok(())
    }

    fn replace_file(
        &mut self,
        temporary: &RemotePath,
        target: &RemotePath,
    ) -> Result<(), RemoteError> {
        if !matches!(self.node(temporary), Some(Node::File(_)))
            || matches!(self.node(target), Some(Node::Dir))
        {
            return Err(RemoteError::Failed("not a regular file"));
        }
        self.remove_node(target);
        self.rename(temporary, target)
    }
}

impl FakeRemoteFs {
    fn require_directory(&self, path: &RemotePath) -> Result<(), RemoteError> {
        match self.node(path) {
            Some(Node::Dir) => Ok(()),
            Some(Node::File(_)) => Err(RemoteError::NotADirectory(path.clone())),
            None => Err(RemoteError::NotFound(path.clone())),
        }
    }

    fn require_parent(&self, path: &RemotePath) -> Result<(), RemoteError> {
        self.require_directory(
            &path
                .parent()
                .ok_or(RemoteError::InvalidInput("cannot replace root"))?,
        )
    }

    fn move_node(&mut self, old: &RemotePath, new: RemotePath) {
        if let Some(node) = self.nodes.remove(old) {
            self.nodes.insert(new.clone(), node);
        }
        if let Some(bytes) = self.contents.remove(old) {
            self.contents.insert(new.clone(), bytes);
        }
        if let Some(mode) = self.modes.remove(old) {
            self.modes.insert(new, mode);
        }
    }

    fn remove_node(&mut self, path: &RemotePath) {
        self.nodes.remove(path);
        self.contents.remove(path);
        self.modes.remove(path);
    }
}
