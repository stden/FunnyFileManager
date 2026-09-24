//! Purpose: Writing the settings file — atomically, owner-only, through a
//! symbolic link — and refusing a file a rename would damage.

use std::{fmt::Display, io, path::Path};

use serde_norway::Mapping;

use super::{file_name, named, HEADER};

/// Write `document` to `path` under the header, atomically and readable by
/// the owner only: a crash mid-write leaves the previous file intact.
///
/// The write lands on the file a symbolic link names, so a settings file kept
/// in a dotfiles repository stays a link. A link that leads nowhere, a file
/// with other hard links and another user's file are refused: renaming a new
/// file over them would turn the link into a plain file, leave the other
/// names holding the old text, or hand the file to this user.
pub(super) fn write(path: &Path, document: &Mapping) -> io::Result<()> {
    let body = serde_norway::to_string(document).map_err(io::Error::other)?;
    let target = crate::file_write::real_target(path)
        .map_err(|link| named(file_name(path), &link.into_io_error()))?;
    refuse_takeover(&target, path)?;
    crate::file_write::ensure_writable(&target)?;
    crate::history::write_owner_only(&target, &format!("{HEADER}{body}"))
        .map_err(|error| named(file_name(path), &error))
}

/// Why renaming a new file over the settings would damage them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Takeover {
    /// The other names would keep the old text.
    HardLinked,
    /// The file would become this user's, readable by this user only.
    NotOwned,
}

impl Display for Takeover {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::HardLinked => "has other hard links; FFM will not replace it",
            Self::NotOwned => "belongs to another user; FFM will not replace it",
        })
    }
}

/// Refuse to replace `target`, the file `path` names, when a rename would
/// damage it (see [`Takeover`]).
fn refuse_takeover(target: &Path, path: &Path) -> io::Result<()> {
    let takeover = if crate::file_write::has_other_names(target) {
        Takeover::HardLinked
    } else if crate::file_write::owned_by_us(target) {
        return Ok(());
    } else {
        Takeover::NotOwned
    };
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!("{} {takeover}", file_name(path)),
    ))
}
