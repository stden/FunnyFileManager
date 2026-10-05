//! Purpose: The file attributes dialog (`Ctrl+A`): owner, permissions and times of the selection.
//!
//! State and rules only: the operations that fill it and act on it live in
//! `ops`, the keys that move through it in `popup`, the drawing in `ui`.
//!
//! Far's dialog has three-state checkboxes, and that is the whole difficulty.
//! A box shows `[x]` when every selected entry has the bit, `[ ]` when none
//! does, and `[?]` when they disagree — and only a box the user *moved* asks
//! for anything, so applying to ten files with different modes changes one
//! bit on each and leaves the other eight alone. Each box therefore remembers
//! what it showed when the dialog opened, and so does each time field.

use std::path::PathBuf;
use std::time::SystemTime;

mod controls;
mod flag;
mod flag_box;
mod time_field;

pub use controls::AttributesField;
pub use flag::{Flag, FlagGroup};
pub use flag_box::{Check, FlagBox};
pub use time_field::{format_time, InvalidTime, RequestedTimes, TimeField};

use flag_box::boxes;

/// What the dialog learned about one entry when it opened.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EntrySnapshot {
    pub path: PathBuf,
    /// The attribute word, as `Attributes::bits()` gives it.
    pub bits: u32,
    pub is_dir: bool,
    pub owner: Option<String>,
    pub group: Option<String>,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
}

/// What the selection disagrees on shows as this rather than as one name.
const VARIOUS: &str = "various";

/// The dialog's state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AttributesDialog {
    pub targets: Vec<PathBuf>,
    pub owner: String,
    pub group: String,
    /// One per platform flag, in [`Flag::GROUPS`] order.
    pub flags: Vec<FlagBox>,
    /// The two time fields; empty or untouched leaves that time alone.
    pub modified: TimeField,
    pub accessed: TimeField,
    /// Far's "Process subfolders": apply to what is inside a directory too.
    pub process_subfolders: bool,
    /// Whether the selection holds a directory, which is what makes the
    /// subfolders box apply and every flag box three-state.
    pub has_directories: bool,
    pub field: AttributesField,
}

impl AttributesDialog {
    /// Open on what was read about the selection.
    #[must_use]
    pub fn from_snapshots(entries: &[EntrySnapshot]) -> Self {
        let has_directories = entries.iter().any(|entry| entry.is_dir);
        let flags = boxes(entries, has_directories);
        // One file offers its own times back, which is Far's "Original";
        // several offer nothing, because a blank field means "leave it".
        let single = if let [only] = entries {
            Some(only)
        } else {
            None
        };
        Self {
            targets: entries.iter().map(|entry| entry.path.clone()).collect(),
            owner: shared_name(entries.iter().map(|entry| entry.owner.as_deref())),
            group: shared_name(entries.iter().map(|entry| entry.group.as_deref())),
            flags,
            modified: TimeField::offering(
                single
                    .and_then(|e| e.modified)
                    .map(format_time)
                    .unwrap_or_default(),
            ),
            accessed: TimeField::offering(
                single
                    .and_then(|e| e.accessed)
                    .map(format_time)
                    .unwrap_or_default(),
            ),
            process_subfolders: false,
            has_directories,
            field: AttributesField::Flag(0),
        }
    }

    /// The first line: the one path, or how many there are.
    #[must_use]
    pub fn headline(&self) -> String {
        match self.targets.as_slice() {
            [only] => only.display().to_string(),
            many => format!("{} objects", many.len()),
        }
    }

    /// `bits` with every moved box applied and every other bit kept.
    #[must_use]
    pub fn new_bits(&self, bits: u32) -> u32 {
        self.flags
            .iter()
            .fold(bits, |bits, flag_box| match flag_box.request() {
                Some(true) => bits | flag_box.flag.bit(),
                Some(false) => bits & !flag_box.flag.bit(),
                None => bits,
            })
    }

    /// Whether any box was moved at all.
    #[must_use]
    pub fn changes_bits(&self) -> bool {
        self.flags
            .iter()
            .any(|flag_box| flag_box.request().is_some())
    }

    /// The two time fields, parsed; a blank or untouched field asks for
    /// nothing.
    pub fn requested_times(&self) -> Result<RequestedTimes, InvalidTime> {
        Ok(RequestedTimes {
            modified: self.modified.request("Modification time")?,
            accessed: self.accessed.request("Access time")?,
        })
    }
}

/// One name when the selection agrees on it, [`VARIOUS`] when it does not,
/// and blank when nothing is known.
fn shared_name<'a>(names: impl Iterator<Item = Option<&'a str>>) -> String {
    let mut shared: Option<&str> = None;
    for name in names {
        match (shared, name) {
            (None, Some(name)) => shared = Some(name),
            (Some(seen), other) if other != Some(seen) => return VARIOUS.to_owned(),
            _ => {}
        }
    }
    shared.unwrap_or_default().to_owned()
}

#[cfg(test)]
mod tests;
