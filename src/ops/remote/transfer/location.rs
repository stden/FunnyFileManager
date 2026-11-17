//! Purpose: Metadata, enumeration and permissions at either transfer endpoint.

use crate::models::remote_work::FileLocation;
use crate::models::{DirEntry, EntryKind, FileSize, Modified, RemotePath};
use crate::remote::{RemoteError, RemoteMetadata, UnixMode};
use std::{fs, path::Path};

pub(in crate::ops::remote) fn metadata(
    location: &FileLocation,
) -> Result<RemoteMetadata, RemoteError> {
    match location {
        FileLocation::Remote { link, path } => link.lock().stat(path),
        FileLocation::Local(path) => Ok(local_metadata(&fs::symlink_metadata(path)?)),
    }
}

fn local_metadata(stat: &fs::Metadata) -> RemoteMetadata {
    let kind = if is_local_link(stat) {
        EntryKind::SymlinkToFile
    } else if stat.is_dir() {
        EntryKind::Directory
    } else {
        EntryKind::File
    };
    RemoteMetadata {
        kind,
        size: FileSize::from_bytes(stat.len()),
        modified: Modified::from_metadata(stat),
        mode: local_mode(stat),
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
    set_created_mode(location, mode)
}

pub(super) fn set_created_mode(location: &FileLocation, mode: UnixMode) -> Result<(), RemoteError> {
    match location {
        FileLocation::Local(path) => set_local_mode(path, mode),
        FileLocation::Remote { link, path } => link.lock().chmod(path, mode),
    }
}

pub(super) fn children(
    location: &FileLocation,
) -> Result<Vec<(String, RemoteMetadata)>, RemoteError> {
    match location {
        FileLocation::Remote { link, path } => link
            .lock()
            .list(path)?
            .into_iter()
            .map(|entry| remote_child_entry(path, entry))
            .collect(),
        FileLocation::Local(path) => fs::read_dir(path)?.map(local_child_entry).collect(),
    }
}

fn remote_child_entry(
    parent: &RemotePath,
    entry: DirEntry,
) -> Result<(String, RemoteMetadata), RemoteError> {
    parent
        .join(&entry.name)
        .ok_or(RemoteError::InvalidInput("invalid remote file name"))?;
    let mode = (entry.attributes.bits() != 0).then(|| UnixMode::from_mode(entry.attributes.bits()));
    let facts = RemoteMetadata {
        kind: entry.kind,
        size: entry.size,
        modified: entry.modified,
        mode,
    };
    Ok((entry.name, facts))
}

fn local_child_entry(
    entry: std::io::Result<fs::DirEntry>,
) -> Result<(String, RemoteMetadata), RemoteError> {
    let entry = entry?;
    let name = utf8_name(entry.file_name())?;
    let facts = local_metadata(&entry.metadata()?);
    Ok((name, facts))
}

pub(in crate::ops::remote) fn utf8_name(name: std::ffi::OsString) -> Result<String, RemoteError> {
    name.into_string()
        .map_err(|_protocol_error| RemoteError::InvalidInput("cannot transfer a non-UTF-8 name"))
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
pub(in crate::ops::remote::transfer) fn set_local_mode(
    path: &Path,
    mode: UnixMode,
) -> Result<(), RemoteError> {
    use std::os::unix::fs::PermissionsExt;
    let perms = fs::Permissions::from_mode(mode.bits());
    Ok(fs::set_permissions(path, perms)?)
}
#[cfg(not(unix))]
#[expect(
    clippy::unnecessary_wraps,
    reason = "Matches the fallible Unix permission interface"
)]
pub(in crate::ops::remote::transfer) fn set_local_mode(
    _path: &Path,
    _mode: UnixMode,
) -> Result<(), RemoteError> {
    Ok(())
}

#[cfg(windows)]
fn is_local_link(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
    metadata.is_symlink() || (metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0)
}

#[cfg(not(windows))]
fn is_local_link(metadata: &fs::Metadata) -> bool {
    metadata.is_symlink()
}
