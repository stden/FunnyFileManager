//! Purpose: Why a copy, move or delete was refused, as far as can be told,
//! in words the user can act on — and whether an administrator could do it.

use std::{fs, io, path::Path};

use crate::models::Denial;

/// Windows' "The process cannot access the file because it is being used by
/// another process", and its lock-violation sibling.
const SHARING_VIOLATION: i32 = 32;
const LOCK_VIOLATION: i32 = 33;

/// Why `error` happened at `path`, when it is a refusal.
pub(super) fn classify(path: &Path, error: &io::Error) -> Option<Denial> {
    if cfg!(windows)
        && matches!(
            error.raw_os_error(),
            Some(SHARING_VIOLATION | LOCK_VIOLATION)
        )
    {
        return Some(Denial::InUse);
    }
    if error.kind() != io::ErrorKind::PermissionDenied {
        return None;
    }
    let metadata = fs::symlink_metadata(path).ok();
    if cfg!(windows)
        && metadata
            .as_ref()
            .is_some_and(|meta| meta.permissions().readonly())
    {
        return Some(Denial::ReadOnly);
    }
    Some(refused(path, metadata.as_ref()))
}

/// Whether running as an administrator could get past `denial`.
pub(super) const fn administrator_may_help(denial: &Denial) -> bool {
    match denial {
        Denial::NeedsAdministrator
        | Denial::FolderLocked(_)
        | Denial::OtherOwner
        | Denial::Rights => true,
        Denial::InUse | Denial::ReadOnly | Denial::NotEvenAdministrator => false,
    }
}

/// A refusal on Windows: the account's rights, which an administrator may
/// have unless FFM already is one.
#[cfg(windows)]
fn refused(_path: &Path, _metadata: Option<&fs::Metadata>) -> Denial {
    if crate::platform::is_elevated() {
        Denial::NotEvenAdministrator
    } else {
        Denial::NeedsAdministrator
    }
}

/// A refusal on Unix: removing or adding a name is up to the folder's mode,
/// changing a file up to its owner and its own mode.
#[cfg(unix)]
fn refused(path: &Path, metadata: Option<&fs::Metadata>) -> Denial {
    use std::os::unix::fs::MetadataExt;

    if let Some(folder) = path
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
    {
        if crate::platform::check_write_access(folder).is_err() {
            return Denial::FolderLocked(folder.to_path_buf());
        }
    }
    if metadata.is_some_and(|meta| meta.uid() != crate::platform::effective_uid()) {
        return Denial::OtherOwner;
    }
    Denial::Rights
}

#[cfg(not(any(unix, windows)))]
fn refused(_path: &Path, _metadata: Option<&fs::Metadata>) -> Denial {
    Denial::Rights
}
