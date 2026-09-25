//! Purpose: Writing the settings file — atomically, owner-only, through a
//! symbolic link — and refusing a file a rename would damage.

use std::{io, path::Path};

use serde_norway::Mapping;

use super::{file_name, HEADER};

/// Write `document` to `path` under the header, the way every file only its
/// owner may read is written (see [`crate::file_write::replace_owner_only`]):
/// atomically, through a symbolic link, and never over a dangling link, a
/// hard-linked file, another user's file or a read-only one.
pub(super) fn write(path: &Path, document: &Mapping) -> io::Result<()> {
    let body = serde_norway::to_string(document).map_err(io::Error::other)?;
    crate::file_write::replace_owner_only(path, &file_name(path), &format!("{HEADER}{body}"))
}
