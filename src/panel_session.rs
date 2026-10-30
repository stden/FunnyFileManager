//! The last displayed directories, persisted separately from optional setup.

use std::{
    io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

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

    /// Unchanged panels cause no disk access, including during idle polling.
    pub(crate) fn save(&mut self, left: &Path, right: &Path) -> io::Result<()> {
        let Some(path) = self.persistence.file() else {
            return Ok(());
        };
        if self
            .saved
            .as_ref()
            .is_some_and(|saved| saved.left == left && saved.right == right)
        {
            return Ok(());
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
        crate::file_write::replace_owner_only(path, "panel-session.json", &text)?;
        self.saved = Some(paths);
        Ok(())
    }
}

fn read(path: &Path) -> io::Result<PanelPaths> {
    let text = crate::fileload::read_small_file(path, "panel-session.json")?;
    serde_json::from_str(&text).map_err(io::Error::other)
}

#[cfg(test)]
mod tests;
