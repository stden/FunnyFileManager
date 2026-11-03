//! Purpose: Streaming SFTP operations. Rename and new writes never replace a taken name.

use super::{at, metadata, SftpFs};
use crate::remote::{RemoteError, RemoteFs, RemoteMetadata, UnixMode, WriteMode};
use crate::values::entry::{DirEntry, EntryKind};
use crate::values::remote_path::RemotePath;
use ssh2::{OpenFlags, OpenType, RenameFlags};
use std::{
    io::{Read, Write},
    path::{Path, PathBuf},
};

impl<S: Send + 'static, F: Read + Write + 'static> RemoteFs for SftpFs<S, F> {
    fn list(&mut self, dir: &RemotePath) -> Result<Vec<DirEntry>, RemoteError> {
        (self.ops.readdir)(&self.sftp, PathBuf::from(dir.as_str()))
            .map_err(|error| at(&error, dir))?
            .into_iter()
            .map(|(path, stat)| self.entry(&path, &stat))
            .collect()
    }

    fn stat(&mut self, path: &RemotePath) -> Result<RemoteMetadata, RemoteError> {
        (self.ops.lstat)(&self.sftp, Path::new(path.as_str()))
            .map(|stat| metadata::metadata(&stat))
            .map_err(|error| at(&error, path))
    }

    fn download(&mut self, path: &RemotePath, output: &mut dyn Write) -> Result<u64, RemoteError> {
        let mut input = (self.ops.open)(&self.sftp, PathBuf::from(path.as_str()))
            .map_err(|error| at(&error, path))?;
        let facts = (self.ops.file_stat)(&mut input).map_err(|error| at(&error, path))?;
        if facts.perm.is_some_and(|mode| mode & 0o170_000 != 0o100_000) {
            return Err(RemoteError::Unsupported("reading special files"));
        }
        let bytes = std::io::copy(&mut input, output)?;
        (self.ops.file_close)(&mut input).map_err(|error| at(&error, path))?;
        Ok(bytes)
    }

    fn upload(
        &mut self,
        path: &RemotePath,
        input: &mut dyn Read,
        mode: WriteMode,
    ) -> Result<u64, RemoteError> {
        let flags = OpenFlags::WRITE
            | OpenFlags::CREATE
            | match mode {
                WriteMode::CreateNew => OpenFlags::EXCLUSIVE,
                WriteMode::Overwrite => OpenFlags::TRUNCATE,
            };
        let mut output = (self.ops.open_mode)(
            &self.sftp,
            PathBuf::from(path.as_str()),
            flags,
            0o600,
            OpenType::File,
        )
        .map_err(|error| at(&error, path))?;
        let bytes = std::io::copy(input, &mut output)?;
        (self.ops.file_close)(&mut output).map_err(|error| at(&error, path))?;
        Ok(bytes)
    }

    fn mkdir(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        (self.ops.mkdir)(&self.sftp, Path::new(path.as_str()), 0o755)
            .map_err(|error| at(&error, path))
    }

    fn remove_file(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        (self.ops.unlink)(&self.sftp, Path::new(path.as_str())).map_err(|error| at(&error, path))
    }

    fn rmdir(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        (self.ops.rmdir)(&self.sftp, Path::new(path.as_str())).map_err(|error| at(&error, path))
    }

    fn rename(&mut self, from: &RemotePath, to: &RemotePath) -> Result<(), RemoteError> {
        match self.stat(to) {
            Ok(_) => return Err(RemoteError::AlreadyExists(to.clone())),
            Err(RemoteError::NotFound(_)) => {}
            Err(error) => return Err(error),
        }
        (self.ops.rename)(
            &self.sftp,
            Path::new(from.as_str()),
            Path::new(to.as_str()),
            Some(RenameFlags::empty()),
        )
        .map_err(|error| at(&error, from))
    }

    fn chmod(&mut self, path: &RemotePath, mode: UnixMode) -> Result<(), RemoteError> {
        let mut stat = metadata::empty_stat();
        stat.perm = Some(mode.bits());
        (self.ops.setstat)(&self.sftp, Path::new(path.as_str()), stat)
            .map_err(|error| at(&error, path))
    }

    fn replace_file(
        &mut self,
        temporary: &RemotePath,
        target: &RemotePath,
    ) -> Result<(), RemoteError> {
        let from = Path::new(temporary.as_str());
        let to = Path::new(target.as_str());
        replace_with(target, |flags| {
            (self.ops.rename)(&self.sftp, from, to, Some(flags))
        })
    }
}

fn replace_with(
    path: &RemotePath,
    mut rename: impl FnMut(RenameFlags) -> Result<(), ssh2::Error>,
) -> Result<(), RemoteError> {
    let flags = RenameFlags::OVERWRITE | RenameFlags::ATOMIC | RenameFlags::NATIVE;
    match rename(flags) {
        Err(error) if error.code() == ssh2::ErrorCode::SFTP(8) => {
            rename(RenameFlags::OVERWRITE).map_err(|error| at(&error, path))
        }
        result => result.map_err(|error| at(&error, path)),
    }
}

#[cfg(test)]
pub(super) mod tests;
#[cfg(test)]
pub(super) mod tests_fake;

impl<S: 'static, F: 'static> SftpFs<S, F> {
    fn entry(&self, path: &Path, stat: &ssh2::FileStat) -> Result<DirEntry, RemoteError> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(RemoteError::Failed("SFTP returned a non-UTF-8 file name"))?;
        let mut entry = metadata::entry(name.to_owned(), stat);
        if entry.kind.is_symlink() {
            entry.kind = match (self.ops.stat)(&self.sftp, path) {
                Ok(target) if target.is_dir() => EntryKind::SymlinkToDirectory,
                Ok(_) => EntryKind::SymlinkToFile,
                Err(_) => EntryKind::BrokenSymlink,
            };
        }
        Ok(entry)
    }
}
