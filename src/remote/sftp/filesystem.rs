//! Purpose: Streaming SFTP operations. Rename and new writes never replace a taken name.

use super::{at, metadata, SftpFs};
use crate::models::{DirEntry, EntryKind, RemotePath};
use crate::remote::{RemoteError, RemoteFs, RemoteMetadata, UnixMode, WriteMode};
use ssh2::{OpenFlags, OpenType, RenameFlags};
use std::{
    io::{Read, Write},
    path::Path,
};

impl RemoteFs for SftpFs {
    fn list(&mut self, dir: &RemotePath) -> Result<Vec<DirEntry>, RemoteError> {
        self.sftp
            .readdir(dir.as_str())
            .map_err(|error| at(&error, dir))?
            .into_iter()
            .map(|(path, stat)| self.entry(&path, &stat))
            .collect()
    }

    fn stat(&mut self, path: &RemotePath) -> Result<RemoteMetadata, RemoteError> {
        self.sftp
            .lstat(Path::new(path.as_str()))
            .map(|stat| metadata::metadata(&stat))
            .map_err(|error| at(&error, path))
    }

    fn download(&mut self, path: &RemotePath, output: &mut dyn Write) -> Result<u64, RemoteError> {
        let mut input = self
            .sftp
            .open(path.as_str())
            .map_err(|error| at(&error, path))?;
        let facts = input.stat().map_err(|error| at(&error, path))?;
        if facts.perm.is_some_and(|mode| mode & 0o170_000 != 0o100_000) {
            return Err(RemoteError::Unsupported("reading special files"));
        }
        let bytes = std::io::copy(&mut input, output)?;
        input.close().map_err(|error| at(&error, path))?;
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
        let mut output = self
            .sftp
            .open_mode(path.as_str(), flags, 0o600, OpenType::File)
            .map_err(|error| at(&error, path))?;
        let bytes = std::io::copy(input, &mut output)?;
        output.close().map_err(|error| at(&error, path))?;
        Ok(bytes)
    }

    fn mkdir(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        self.sftp
            .mkdir(Path::new(path.as_str()), 0o755)
            .map_err(|error| at(&error, path))
    }

    fn remove_file(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        self.sftp
            .unlink(Path::new(path.as_str()))
            .map_err(|error| at(&error, path))
    }

    fn rmdir(&mut self, path: &RemotePath) -> Result<(), RemoteError> {
        self.sftp
            .rmdir(Path::new(path.as_str()))
            .map_err(|error| at(&error, path))
    }

    fn rename(&mut self, from: &RemotePath, to: &RemotePath) -> Result<(), RemoteError> {
        match self.stat(to) {
            Ok(_) => return Err(RemoteError::AlreadyExists(to.clone())),
            Err(RemoteError::NotFound(_)) => {}
            Err(error) => return Err(error),
        }
        self.sftp
            .rename(
                Path::new(from.as_str()),
                Path::new(to.as_str()),
                Some(RenameFlags::empty()),
            )
            .map_err(|error| at(&error, from))
    }

    fn chmod(&mut self, path: &RemotePath, mode: UnixMode) -> Result<(), RemoteError> {
        let mut stat = metadata::empty_stat();
        stat.perm = Some(mode.bits());
        self.sftp
            .setstat(Path::new(path.as_str()), stat)
            .map_err(|error| at(&error, path))
    }

    fn replace_file(
        &mut self,
        temporary: &RemotePath,
        target: &RemotePath,
    ) -> Result<(), RemoteError> {
        let from = Path::new(temporary.as_str());
        let to = Path::new(target.as_str());
        let flags = RenameFlags::OVERWRITE | RenameFlags::ATOMIC | RenameFlags::NATIVE;
        match self.sftp.rename(from, to, Some(flags)) {
            Err(error) if error.code() == ssh2::ErrorCode::SFTP(8) => self
                .sftp
                .rename(from, to, Some(RenameFlags::OVERWRITE))
                .map_err(|error| at(&error, target)),
            result => result.map_err(|error| at(&error, target)),
        }
    }
}

impl SftpFs {
    fn entry(&self, path: &Path, stat: &ssh2::FileStat) -> Result<DirEntry, RemoteError> {
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or(RemoteError::Failed("SFTP returned a non-UTF-8 file name"))?;
        let mut entry = metadata::entry(name.to_owned(), stat);
        if entry.kind.is_symlink() {
            entry.kind = match self.sftp.stat(path) {
                Ok(target) if target.is_dir() => EntryKind::SymlinkToDirectory,
                Ok(_) => EntryKind::SymlinkToFile,
                Err(_) => EntryKind::BrokenSymlink,
            };
        }
        Ok(entry)
    }
}
