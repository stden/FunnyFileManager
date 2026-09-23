//! Purpose: The file associations file ("File associations") — where it lives.
//!
//! Sections of a mask and the commands for `Enter`, `F3`, `F4`, `Alt+F3` and
//! `Alt+F4`. Parsing the file and consulting it before the executable rule
//! arrive with the feature; the location is fixed here, beside the settings,
//! so the dialog and `Enter` read the same file.

use std::path::PathBuf;

/// The file's name under the `ffm` config directory.
pub const FILE_NAME: &str = "associations.ini";

/// `<config>/ffm/associations.ini`, or `None` when the environment names no home.
#[must_use]
pub fn path() -> Option<PathBuf> {
    crate::config::dir().map(|dir| dir.join(FILE_NAME))
}

#[cfg(test)]
mod tests;
