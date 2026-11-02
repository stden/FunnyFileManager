//! Purpose: Validate a temporary root and share age/ownership checks between preview and deletion.
use crate::models::temp_cleanup::{FileIdentity, TempCutoff, TempError, TempRoot, TempSource};
use std::{
    fs,
    path::{Component, Path},
};
mod system;
pub(super) fn root(source: &TempSource) -> Result<TempRoot, TempError> {
    let (path, home) = match source {
        TempSource::Unavailable => return Err(TempError::Unavailable),
        TempSource::System => return root(&system::source()?),
        TempSource::Directory { path, home } => (path.clone(), home.clone()),
    };
    if !path.is_absolute() || path.parent().is_none() || !directory(&path) {
        return Err(TempError::UnsafeRoot);
    }
    let path = fs::canonicalize(path).map_err(|_error| TempError::Scan)?;
    let home = home.and_then(|home| fs::canonicalize(home).ok());
    if path.parent().is_none() || home.as_ref() == Some(&path) {
        return Err(TempError::UnsafeRoot);
    }
    let identity = identity(&path).map_err(|_error| TempError::Scan)?;
    Ok(TempRoot { path, identity })
}
pub(super) fn identity(path: &Path) -> std::io::Result<FileIdentity> {
    file_id::get_file_id(path).map(FileIdentity)
}
pub(super) fn safe_parent(root: &TempRoot, path: &Path) -> bool {
    if !directory(root.path()) || identity(root.path()).ok() != Some(root.identity) {
        return false;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    let Ok(relative) = parent.strip_prefix(root.path()) else {
        return false;
    };
    let mut current = root.path.clone();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return false;
        };
        current.push(part);
        if !directory(&current) {
            return false;
        }
        if !fs::symlink_metadata(&current).is_ok_and(|metadata| owned(&metadata)) {
            return false;
        }
    }
    true
}
pub(super) fn directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir() && !linked(&metadata))
}
pub(super) fn eligible(metadata: &fs::Metadata, cutoff: TempCutoff) -> bool {
    metadata.is_file()
        && !linked(metadata)
        && owned(metadata)
        && metadata
            .modified()
            .is_ok_and(|modified| modified < cutoff.0)
}
#[cfg(unix)]
pub(super) fn owned(metadata: &fs::Metadata) -> bool {
    use std::os::unix::fs::MetadataExt;
    metadata.uid() == rustix::process::geteuid().as_raw()
}
#[cfg(not(unix))]
pub(super) const fn owned(_metadata: &fs::Metadata) -> bool {
    true
}

#[cfg(windows)]
pub(super) fn linked(metadata: &fs::Metadata) -> bool {
    use std::os::windows::fs::MetadataExt;
    metadata.file_attributes() & 0x400 != 0
}
#[cfg(not(windows))]
pub(super) fn linked(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

#[cfg(unix)]
pub(super) fn on_root_device(metadata: &fs::Metadata, root: &TempRoot) -> bool {
    use std::os::unix::fs::MetadataExt;
    matches!(root.identity.0, file_id::FileId::Inode { device_id, .. } if metadata.dev() == device_id)
}
#[cfg(not(unix))]
pub(super) const fn on_root_device(_metadata: &fs::Metadata, _root: &TempRoot) -> bool {
    true
}
