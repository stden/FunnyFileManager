//! Canonical directory loading and navigation effects.
//!
//! Inherent `Panel` methods preserve the public API; their implementation lives
//! here so the domain panel only applies complete snapshots and the renderer
//! never queries the filesystem.

use std::{io, path::PathBuf};

use crate::{directory, models::Panel};

impl Panel {
    /// Load a filesystem directory into a new panel.
    pub fn new(path: PathBuf) -> io::Result<Self> {
        let mut panel = Self::from_entries(path, Vec::new());
        let listing = directory::read(&panel.path, &panel.description_names)?;
        panel.apply_listing(listing);
        panel.ensure_selection();
        Ok(panel)
    }

    /// Re-read the directory, preserving the last complete snapshot on error.
    pub fn load_entries(&mut self) -> io::Result<()> {
        let listing = directory::read(&self.path, &self.description_names)?;
        let mut updated = self.clone();
        updated.apply_listing(listing);
        let view = updated.view;
        super::panel_details::set_view(&mut updated, view)?;
        *self = updated;
        Ok(())
    }

    /// Navigate to a readable directory, dropping marks tied to the old path.
    ///
    /// Loading precedes the path change. If the destination cannot be read,
    /// refresh the original directory so entries that disappeared are removed,
    /// while preserving its path and cursor. If that refresh also fails, keep
    /// the last complete snapshot. Always report the destination's error.
    pub fn navigate_to(&mut self, path: PathBuf) -> io::Result<()> {
        let listing = match directory::read(&path, &self.description_names) {
            Ok(listing) => listing,
            Err(error) => {
                let previous_selection = self.selected();
                let _ = self.load_entries();
                self.select(previous_selection);
                self.ensure_selection();
                return Err(error);
            }
        };
        let mut updated = self.clone();
        updated.apply_navigation(path, listing);
        let view = updated.view;
        super::panel_details::set_view(&mut updated, view)?;
        *self = updated;
        Ok(())
    }

    pub fn enter(&mut self) -> io::Result<()> {
        let Some(i) = self.selected() else {
            return Ok(());
        };
        let Some(entry) = self.entries.get(i) else {
            return Ok(());
        };
        if !entry.kind.is_dir() {
            return Ok(());
        }

        if entry.name == ".." {
            // Remember the directory we're leaving so we can select it after going up
            let prev_dir_name = self
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string());
            let mut parent = self.path.clone();
            parent.pop();
            self.navigate_to(parent)?;
            // Select the directory we just came from
            if let Some(pos) =
                prev_dir_name.and_then(|name| self.entries.iter().position(|e| e.name == name))
            {
                self.select(Some(pos));
                return Ok(());
            }
            self.select(self.entries.first().map(|_| 0));
        } else {
            // Build the target first: the listing (and the entry we selected in
            // it) must stay valid until `navigate_to` has proven it readable.
            let target = self.path.join(&entry.name);
            self.navigate_to(target)?;
            self.select(self.entries.first().map(|_| 0));
        }
        Ok(())
    }
}
