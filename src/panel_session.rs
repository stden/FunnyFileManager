//! The last displayed directories, persisted separately from optional setup.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::file_write::SyncPolicy;
use crate::history::Persistence;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PanelPaths {
    pub left: PathBuf,
    pub right: PathBuf,
}

/// Constructors remain session-only until bootstrap supplies a store.
#[derive(Debug, Clone, Default)]
pub struct PanelSession {
    persistence: Persistence,
    saved: Option<PanelPaths>,
}

impl PanelSession {
    pub(crate) fn load(persistence: Persistence) -> Self {
        let saved = persistence.file().and_then(|path| match read(path) {
            Ok(paths) => Some(paths),
            Err(error) => {
                if error.kind() != io::ErrorKind::NotFound {
                    crate::diagnostics::log_error(&format!("loading panel session: {error}"));
                }
                None
            }
        });
        Self { persistence, saved }
    }

    pub(crate) fn paths(&self) -> Option<&PanelPaths> {
        self.saved.as_ref()
    }

    /// Save the pair of panel directories. A checkpoint writes nothing when the
    /// pair is unchanged, so idle polling never touches the disk. A flush of an
    /// unchanged pair fsyncs the file already there instead of rewriting it, so
    /// an idle instance cannot replace a newer session another FFM wrote.
    pub(crate) fn save(&mut self, left: &Path, right: &Path, policy: SyncPolicy) -> io::Result<()> {
        let Some(path) = self.persistence.file() else {
            return Ok(());
        };
        let unchanged = self
            .saved
            .as_ref()
            .is_some_and(|saved| saved.left == left && saved.right == right);
        if unchanged {
            return match policy {
                SyncPolicy::Skip => Ok(()),
                SyncPolicy::Always => sync_saved(path),
            };
        }
        let paths = PanelPaths {
            left: std::path::absolute(left)?,
            right: std::path::absolute(right)?,
        };
        let text = serde_json::to_string_pretty(&paths).map_err(io::Error::other)?;
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            crate::file_write::create_dirs_owner_only(parent)?;
        }
        crate::file_write::replace_owner_only(path, "panel-session.json", &text, policy)?;
        self.saved = Some(paths);
        Ok(())
    }
}

/// Fsync the session file already on disk. It is opened for writing without
/// truncation, because `sync_all` needs write access on Windows. A file that is
/// gone has nothing to sync, so the exit save of unchanged panels is not an
/// error when the user deleted the file while FFM ran.
fn sync_saved(path: &Path) -> io::Result<()> {
    match crate::fileload::open_regular(path, fs::OpenOptions::new().write(true)) {
        Ok(file) => file.sync_all(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn read(path: &Path) -> io::Result<PanelPaths> {
    let text = crate::fileload::read_small_file(path, "panel-session.json")?;
    serde_json::from_str(&text).map_err(io::Error::other)
}

#[cfg(test)]
mod tests;
