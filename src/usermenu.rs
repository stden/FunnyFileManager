//! Purpose: The user menu file (`F2`, "Edit user menu") — where it lives.
//!
//! Far keeps its user menu in `FarMenu.ini`; this program keeps its own beside
//! the settings, so that everything the user configured is in one directory
//! they can back up. Reading the file, running a chosen line and creating the
//! commented default arrive with the feature; the location is fixed here so
//! the dialog, the editor and the docs cannot disagree about it.

use std::path::PathBuf;

/// The file's name under the `ffm` config directory.
pub const FILE_NAME: &str = "menu.ini";

/// `<config>/ffm/menu.ini`, or `None` when the environment names no home.
#[must_use]
pub fn path() -> Option<PathBuf> {
    crate::config::dir().map(|dir| dir.join(FILE_NAME))
}

#[cfg(test)]
mod tests;
