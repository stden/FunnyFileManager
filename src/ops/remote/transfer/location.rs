//! Purpose: Metadata, enumeration and permissions at either transfer endpoint.

use crate::models::remote_work::FileLocation;
use crate::models::{EntryKind, FileSize, Modified};
use crate::remote::{RemoteError, RemoteMetadata, UnixMode};
use std::{fs, path::Path};

pub(in crate::ops::remote) fn metadata(
    location: &FileLocation,
) -> Result<RemoteMetadata, RemoteError> {
    match location {
        FileLocation::Remote { link, path } => link.lock().stat(path),
        FileLocation::Local(path) => {
            let stat = fs::symlink_metadata(path)?;
            let kind = if stat.is_symlink() {
                EntryKind::SymlinkToFile
            } else if stat.is_dir() {
                EntryKind::Directory
            } else {
                EntryKind::File
            };
            Ok(RemoteMetadata {
                kind,
                size: FileSize::from_bytes(stat.len()),
                modified: Modified::from_metadata(&stat),
                mode: local_mode(&stat),
            })
        }
    }
}

pub(in crate::ops::remote) fn mkdir(location: &FileLocation) -> Result<(), RemoteError> {
    match location {
        FileLocation::Local(path) => Ok(fs::create_dir(path)?),
        FileLocation::Remote { link, path } => link.lock().mkdir(path),
    }
}

pub(in crate::ops::remote) fn chmod(
    location: &FileLocation,
    mode: UnixMode,
) -> Result<(), RemoteError> {
    if metadata(location)?.kind.is_symlink() {
        return Err(RemoteError::Unsupported(
            "changing symbolic-link permissions",
        ));
    }
    match location {
        FileLocation::Local(path) => set_local_mode(path, mode),
        FileLocation::Remote { link, path } => link.lock().chmod(path, mode),
    }
}

pub(super) fn children(location: &FileLocation) -> Result<Vec<String>, RemoteError> {
    match location {
        FileLocation::Remote { link, path } => link
            .lock()
            .list(path)?
            .into_iter()
            .map(|entry| {
                path.join(&entry.name)
                    .ok_or(RemoteError::InvalidInput("invalid remote file name"))?;
                Ok(entry.name)
            })
            .collect(),
        FileLocation::Local(path) => fs::read_dir(path)?
            .map(|entry| {
                entry?.file_name().into_string().map_err(|_protocol_error| {
                    RemoteError::InvalidInput("cannot transfer a non-UTF-8 name")
                })
            })
            .collect(),
    }
}

#[cfg(unix)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "the non-Unix variant returns None, so both share the Option signature"
)]
fn local_mode(metadata: &fs::Metadata) -> Option<UnixMode> {
    use std::os::unix::fs::PermissionsExt;
    Some(UnixMode::from_mode(metadata.permissions().mode()))
}
#[cfg(not(unix))]
fn local_mode(_metadata: &fs::Metadata) -> Option<UnixMode> {
    None
}

#[cfg(unix)]
fn set_local_mode(path: &Path, mode: UnixMode) -> Result<(), RemoteError> {
    use std::os::unix::fs::PermissionsExt;
    Ok(fs::set_permissions(
        path,
        fs::Permissions::from_mode(mode.bits()),
    )?)
}
#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "Matches the fallible Unix permission interface"
)]
fn set_local_mode(_path: &Path, _mode: UnixMode) -> Result<(), RemoteError> {
    Ok(())
}
