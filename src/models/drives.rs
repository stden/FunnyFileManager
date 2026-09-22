//! Purpose: the rows of Far's Alt+F1 / Alt+F2 "Change drive" dialog.
//!
//! Presentation only. The dialog's state — these rows and the selected index —
//! lives on [`Overlay::DriveSelection`](super::Overlay::DriveSelection); this
//! turns the drives the machine reports into the two sections Far draws — the
//! disks with their size, and below a rule whatever else can be opened as a
//! panel.

use std::path::Path;

use super::{App, DriveEntry};
use crate::diskfree::{capacity, drive_kind, Capacity, DriveKind};

/// The keys Far offers for the rows under the rule, in the order it uses them.
const PLUGIN_KEYS: [char; 10] = ['0', '1', '2', '3', '4', '5', '6', '7', '8', '9'];

/// What a row shows, which is also which section it belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RowKind {
    /// A filesystem: the letter, what kind of device it is, and how big.
    Disk {
        label: String,
        device: DriveKind,
        space: Option<Capacity>,
    },
    /// Everything Far lists below the rule as a plugin. Here: the S3 profiles.
    Plugin { name: String },
}

/// One line of the dialog, and what selecting it opens.
#[derive(Clone, Debug)]
pub struct DriveRow {
    /// The key that jumps straight to this row. `None` past the tenth plugin
    /// row, where the digits run out: an `Option` rather than a sentinel
    /// character, so a keyless row cannot accidentally answer to that
    /// character as a keypress.
    pub key: Option<char>,
    pub kind: RowKind,
    pub entry: DriveEntry,
}

impl DriveRow {
    /// Whether this row belongs in the boxed table at the top.
    #[must_use]
    pub const fn is_disk(&self) -> bool {
        matches!(self.kind, RowKind::Disk { .. })
    }

    /// Whether `key` selects this row. Case-insensitive, because a drive letter
    /// is shown upper case but nobody holds shift to reach it. A keyless row
    /// answers to nothing.
    #[must_use]
    pub fn answers_to(&self, key: char) -> bool {
        self.key.is_some_and(|own| own.eq_ignore_ascii_case(&key))
    }
}

/// The key each entry answers to, in list order.
///
/// Split from [`rows`] so a test can pin the hand-out order without reading
/// any filesystem's size.
fn keys(entries: &[DriveEntry]) -> Vec<Option<char>> {
    let mut plugin_keys = PLUGIN_KEYS.iter().copied();
    entries
        .iter()
        .map(|entry| match entry {
            DriveEntry::Local(path) => Some(drive_key(path)),
            // Past ten profiles the rows are still listed and still selectable
            // with the cursor; only the shortcut runs out.
            DriveEntry::S3Profile(_) => plugin_keys.next(),
        })
        .collect()
}

/// The entry a keypress selects among `rows`, or `None` when no row answers
/// to it. Pure over the rows the dialog stored when it opened, so a keypress
/// costs no drive scan.
#[must_use]
pub fn find_by_key(rows: &[DriveRow], key: char) -> Option<DriveEntry> {
    rows.iter()
        .find(|row| row.answers_to(key))
        .map(|row| row.entry.clone())
}

/// Every row of the dialog, disks first, in the order Far lists them.
///
/// Called once, when the dialog opens, and stored on the overlay: reading a
/// filesystem's size is a `statvfs` per drive, and the event loop redraws
/// after every key — repeated here per frame it would make the dialog cost
/// more to draw than the panels behind it.
#[must_use]
pub fn rows() -> Vec<DriveRow> {
    let entries = App::get_all_drives();
    keys(&entries)
        .into_iter()
        .zip(entries)
        .map(row_for)
        .collect()
}

/// One row from the entry behind it. Separate from [`rows`] so that a profile
/// row can be built in a test on a machine with no AWS configuration.
fn row_for((key, entry): (Option<char>, DriveEntry)) -> DriveRow {
    let kind = match &entry {
        DriveEntry::Local(path) => RowKind::Disk {
            label: drive_label(path),
            device: drive_kind(path),
            space: capacity(path),
        },
        DriveEntry::S3Profile(name) => RowKind::Plugin {
            name: format!("S3: {name}"),
        },
    };
    DriveRow { key, kind, entry }
}

/// The key that jumps to a drive: its letter on Windows, and on Unix the first
/// character of the mount point, which for the single root drive is `/`.
fn drive_key(path: &Path) -> char {
    drive_label(path).chars().next().unwrap_or('?')
}

/// How the drive names itself in the first column: `C:` on Windows, where the
/// trailing separator is noise, and the mount point itself on Unix.
fn drive_label(path: &Path) -> String {
    let text = path.display().to_string();
    match text.strip_suffix('\\').filter(|rest| rest.ends_with(':')) {
        Some(letter) => letter.to_owned(),
        None => text,
    }
}

#[cfg(test)]
mod tests;
