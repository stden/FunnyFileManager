//! Purpose: Replacing a file only its owner may read — the settings, the
//! stored sessions — through a symbolic link, and never a file a rename
//! would damage; and making the owner-only folders they live in.

use std::{fmt, io, path::Path};

use super::{has_other_names, owned_by_us, real_target, refuse_unwritable};

/// Why renaming a new file over the target would damage it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Takeover {
    /// The other names would keep the old text.
    HardLinked,
    /// The file would become this user's, readable by this user only.
    NotOwned,
}

impl fmt::Display for Takeover {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::HardLinked => "has other hard links; FFM will not replace it",
            Self::NotOwned => "belongs to another user; FFM will not replace it",
        })
    }
}

/// Write `text` to the file `path` names, atomically and readable by the
/// owner only: a crash mid-write leaves the previous file intact. `name` is
/// what every error calls the file.
///
/// The write lands on the file a symbolic link names, so a file kept in a
/// dotfiles repository stays a link. A link that leads nowhere, a file with
/// other hard links, another user's file and a read-only one are refused:
/// renaming a new file over them would turn the link into a plain file,
/// leave the other names holding the old text, hand the file to this user,
/// or ignore its permissions.
pub fn replace_owner_only(path: &Path, name: &str, text: &str) -> io::Result<()> {
    let target = real_target(path).map_err(|link| named(name, &link.into_io_error()))?;
    refuse_takeover(&target, name)?;
    // By its name, as every other refusal here: the status line is half the
    // screen, and a full path in front of the reason pushed it off the end.
    refuse_unwritable(&target).map_err(|error| named(name, &error))?;
    write_owner_only(&target, text).map_err(|error| named(name, &error))
}

/// Refuse to replace `target` when a rename would damage it (see
/// [`Takeover`]).
fn refuse_takeover(target: &Path, name: &str) -> io::Result<()> {
    let takeover = if has_other_names(target) {
        Takeover::HardLinked
    } else if owned_by_us(target) {
        return Ok(());
    } else {
        Takeover::NotOwned
    };
    Err(io::Error::new(
        io::ErrorKind::PermissionDenied,
        format!("{name} {takeover}"),
    ))
}

/// Write `text` to `path` via a co-located temp file + atomic rename.
///
/// Truncating the live history file in place loses entries if another FFM
/// process writes concurrently or if we crash mid-write. A sibling temp +
/// `persist` keeps readers seeing the previous complete file until the rename.
pub fn write_owner_only(path: &Path, text: &str) -> io::Result<()> {
    use std::io::Write;

    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(dir)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // Owner-only from the first byte: history holds the user's paths.
        tmp.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    tmp.write_all(text.as_bytes())?;
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| e.error)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        // `persist` keeps the temp file's mode on most platforms; re-tighten
        // in case a pre-existing destination's metadata was reused.
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Create `path` and every missing folder above it readable by the owner
/// only (0700), as the XDG spec asks of the directories it names. A folder
/// that already exists keeps its mode: how it is shared is its owner's choice.
pub fn create_dirs_owner_only(path: &Path) -> io::Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

/// `error`, of the same kind, after `name`.
fn named(name: &str, error: &io::Error) -> io::Error {
    io::Error::new(error.kind(), format!("{name}: {error}"))
}

#[cfg(test)]
mod tests;
