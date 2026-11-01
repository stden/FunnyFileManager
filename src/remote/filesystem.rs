//! Purpose: Streaming filesystem contract shared by transports and panel tests.

use super::{RemoteError, RemoteMetadata, UnixMode, WriteMode};
use crate::models::{DirEntry, RemotePath};
use std::io::{Read, Write};

pub trait RemoteFs: Send {
    fn list(&mut self, dir: &RemotePath) -> Result<Vec<DirEntry>, RemoteError>;

    fn stat(&mut self, path: &RemotePath) -> Result<RemoteMetadata, RemoteError> {
        let parent = path.parent().ok_or(RemoteError::Unsupported("stat root"))?;
        let name = path.as_str().rsplit('/').next().unwrap_or_default();
        self.list(&parent)?
            .iter()
            .find(|entry| entry.name == name)
            .map(RemoteMetadata::from_entry)
            .ok_or_else(|| RemoteError::NotFound(path.clone()))
    }

    fn download(
        &mut self,
        _path: &RemotePath,
        _output: &mut dyn Write,
    ) -> Result<u64, RemoteError> {
        Err(RemoteError::Unsupported("download"))
    }

    fn upload(
        &mut self,
        _path: &RemotePath,
        _input: &mut dyn Read,
        _mode: WriteMode,
    ) -> Result<u64, RemoteError> {
        Err(RemoteError::Unsupported("upload"))
    }

    fn mkdir(&mut self, _path: &RemotePath) -> Result<(), RemoteError> {
        Err(RemoteError::Unsupported("mkdir"))
    }

    fn remove_file(&mut self, _path: &RemotePath) -> Result<(), RemoteError> {
        Err(RemoteError::Unsupported("remove file"))
    }

    fn rmdir(&mut self, _path: &RemotePath) -> Result<(), RemoteError> {
        Err(RemoteError::Unsupported("remove directory"))
    }

    fn rename(&mut self, _from: &RemotePath, _to: &RemotePath) -> Result<(), RemoteError> {
        Err(RemoteError::Unsupported("rename"))
    }

    fn chmod(&mut self, _path: &RemotePath, _mode: UnixMode) -> Result<(), RemoteError> {
        Err(RemoteError::Unsupported("chmod"))
    }

    fn replace_file(
        &mut self,
        _temporary: &RemotePath,
        _target: &RemotePath,
    ) -> Result<(), RemoteError> {
        Err(RemoteError::Unsupported("replacing a file by rename"))
    }
}
