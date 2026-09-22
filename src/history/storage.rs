//! Purpose: Where the history file lives — the persistence target, its XDG path and the owner-only write.

use std::path::PathBuf;

/// Where this history writes, if anywhere.
///
/// An enum rather than `Option<PathBuf>` because `None` here does not mean
/// "the path is missing" — it means *this history deliberately does not
/// persist*, which is the state every test and every freshly-constructed `App`
/// is in. `save` is then not a failed write, it is a no-op by design, and the
/// name says so.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Persistence {
    /// Nothing reaches disk; the ring dies with the process.
    #[default]
    SessionOnly,
    /// Durable lines are written to this file.
    File(PathBuf),
}

impl Persistence {
    /// The file to write to, or `None` while session-only.
    #[must_use]
    pub fn file(&self) -> Option<&std::path::Path> {
        match self {
            Self::SessionOnly => None,
            Self::File(path) => Some(path),
        }
    }
}

/// `$XDG_DATA_HOME/ffm/history`, defaulting to `~/.local/share/ffm/history`.
pub(super) fn history_file_path() -> Option<PathBuf> {
    data_file("history")
}

/// `$XDG_DATA_HOME/ffm/<name>`, defaulting to `~/.local/share/ffm/<name>` —
/// the one data directory every history (command, view, folder) shares.
pub fn data_file(name: &str) -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        // XDG spec: a relative (or empty) $XDG_DATA_HOME is invalid and must be
        // ignored, so history never lands in a cwd-relative directory.
        .filter(|p| p.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))?;
    Some(base.join("ffm").join(name))
}

/// Write `text` to `path` via a co-located temp file + atomic rename.
///
/// Truncating the live history file in place loses entries if another FFM
/// process writes concurrently or if we crash mid-write. A sibling temp +
/// `persist` keeps readers seeing the previous complete file until the rename.
pub fn write_owner_only(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;

    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
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
