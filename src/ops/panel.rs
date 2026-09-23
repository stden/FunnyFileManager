//! Canonical directory loading and navigation effects.
//!
//! Inherent `Panel` methods preserve the public API; their implementation lives
//! here so the domain panel only applies complete snapshots and the renderer
//! never queries the filesystem.

use std::{
    io,
    path::{Path, PathBuf},
};

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

    /// Ctrl+R: re-read the directory — or, when it has been deleted, move to
    /// the nearest folder above it that still exists, as Far's panel does.
    /// Any other failure keeps the last complete snapshot, as
    /// [`Self::load_entries`] does.
    pub fn reload(&mut self) -> io::Result<()> {
        match self.load_entries() {
            Err(_) if !self.path.is_dir() => self.go_up(),
            result => result,
        }
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
                // Best effort: the destination's error is the one reported.
                drop(self.load_entries());
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
            self.go_up()?;
        } else {
            // Build the target first: the listing (and the entry we selected in
            // it) must stay valid until `navigate_to` has proven it readable.
            let target = self.path.join(&entry.name);
            self.navigate_to(target)?;
            self.select(self.entries.first().map(|_| 0));
        }
        Ok(())
    }

    /// `..`: one level up — or, when that folder has been deleted too, the
    /// nearest one above it that still exists — with the cursor on the way
    /// back to the folder that was left.
    fn go_up(&mut self) -> io::Result<()> {
        let left = self.path.clone();
        let parent = left.parent().unwrap_or(&left);
        let target = directory::nearest_existing(parent).unwrap_or(parent);
        self.navigate_to(target.to_path_buf())?;
        self.select_toward(&left);
        Ok(())
    }

    /// Put the cursor on the entry that leads back down to `left`, or on the
    /// first row when that entry is gone.
    fn select_toward(&mut self, left: &Path) {
        let step = left
            .strip_prefix(&self.path)
            .ok()
            .and_then(|rest| rest.components().next());
        let found = step.is_some_and(|step| self.select_named(&step.as_os_str().to_string_lossy()));
        if !found {
            self.select(self.entries.first().map(|_| 0));
        }
    }
}

#[cfg(test)]
mod tests;
