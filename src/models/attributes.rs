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
//! what it showed when the dialog opened.

use std::path::PathBuf;
use std::time::SystemTime;

/// The three states Far draws a checkbox in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Check {
    /// `[x]`: every selected entry has the bit.
    Set,
    /// `[ ]`: none does.
    Clear,
    /// `[?]`: they disagree — leave each as it is.
    Mixed,
}

impl Check {
    /// The character inside the brackets.
    #[must_use]
    pub const fn mark(self) -> char {
        match self {
            Self::Set => 'x',
            Self::Clear => ' ',
            Self::Mixed => '?',
        }
    }

    fn of(with_bit: usize, total: usize) -> Self {
        if with_bit == 0 {
            Self::Clear
        } else if with_bit == total {
            Self::Set
        } else {
            Self::Mixed
        }
    }
}

/// What one checkbox stands for: a mode bit on Unix, an attribute flag on
/// Windows.
///
/// Both platforms' flags are named so the Windows rows can be tested on a
/// Unix host; [`Flag::GROUPS`] picks the platform's own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flag {
    OwnerRead,
    OwnerWrite,
    OwnerExecute,
    GroupRead,
    GroupWrite,
    GroupExecute,
    OtherRead,
    OtherWrite,
    OtherExecute,
    SetUid,
    SetGid,
    Sticky,
    ReadOnly,
    Hidden,
    System,
    Archive,
}

/// One row of checkboxes: its caption and the flags it holds, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagGroup {
    pub caption: &'static str,
    pub flags: &'static [Flag],
}

const UNIX_GROUPS: &[FlagGroup] = &[
    FlagGroup {
        caption: "Owner",
        flags: &[Flag::OwnerRead, Flag::OwnerWrite, Flag::OwnerExecute],
    },
    FlagGroup {
        caption: "Group",
        flags: &[Flag::GroupRead, Flag::GroupWrite, Flag::GroupExecute],
    },
    FlagGroup {
        caption: "Other",
        flags: &[Flag::OtherRead, Flag::OtherWrite, Flag::OtherExecute],
    },
    FlagGroup {
        caption: "Special",
        flags: &[Flag::SetUid, Flag::SetGid, Flag::Sticky],
    },
];

const WINDOWS_GROUPS: &[FlagGroup] = &[FlagGroup {
    caption: "Flags",
    flags: &[Flag::ReadOnly, Flag::Hidden, Flag::System, Flag::Archive],
}];

impl Flag {
    /// The platform's checkboxes, in the rows the dialog draws them.
    pub const GROUPS: &'static [FlagGroup] = if cfg!(windows) {
        WINDOWS_GROUPS
    } else {
        UNIX_GROUPS
    };

    /// The bit in the entry's attribute word — `st_mode` or the Windows
    /// attribute flags, the same word `Attributes` renders.
    ///
    /// The two platforms' low bits coincide numerically (Windows' `0x20`
    /// Archive is Unix's `0o040`), which is why some arms are shared: the
    /// same number, meaning a different thing on a different disk.
    #[must_use]
    pub const fn bit(self) -> u32 {
        match self {
            Self::OwnerRead => 0o400,
            Self::OwnerWrite => 0o200,
            Self::OwnerExecute => 0o100,
            Self::GroupRead | Self::Archive => 0o040,
            Self::GroupWrite => 0o020,
            Self::GroupExecute => 0o010,
            Self::OtherRead | Self::System => 0o004,
            Self::OtherWrite | Self::Hidden => 0o002,
            Self::OtherExecute | Self::ReadOnly => 0o001,
            Self::SetUid => 0o4000,
            Self::SetGid => 0o2000,
            Self::Sticky => 0o1000,
        }
    }

    /// The word beside the box.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::OwnerRead | Self::GroupRead | Self::OtherRead => "Read",
            Self::OwnerWrite | Self::GroupWrite | Self::OtherWrite => "Write",
            Self::OwnerExecute | Self::GroupExecute | Self::OtherExecute => "Execute",
            Self::SetUid => "Set UID",
            Self::SetGid => "Set GID",
            Self::Sticky => "Sticky",
            Self::ReadOnly => "Read-only",
            Self::Hidden => "Hidden",
            Self::System => "System",
            Self::Archive => "Archive",
        }
    }
}

/// One checkbox: what it showed when the dialog opened, and what it shows now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlagBox {
    pub flag: Flag,
    pub initial: Check,
    pub current: Check,
    /// Whether the cycle passes through `[?]`. Far makes every box three-state
    /// as soon as a folder is selected, because "leave it" is then a real
    /// answer for the files inside.
    three_state: bool,
}

impl FlagBox {
    /// Space: the next state round. A two-state box flips; a three-state one
    /// goes set → clear → leave → set, so `[?]` is reachable again after a
    /// slip.
    pub fn toggle(&mut self) {
        self.current = match self.current {
            Check::Set => Check::Clear,
            Check::Clear if self.three_state => Check::Mixed,
            Check::Clear | Check::Mixed => Check::Set,
        };
    }

    /// What this box asks for: set the bit, clear it, or nothing.
    ///
    /// `None` both for `[?]` and for a box left as it opened — Far changes
    /// only what the user moved.
    #[must_use]
    pub fn request(&self) -> Option<bool> {
        if self.current == self.initial {
            return None;
        }
        match self.current {
            Check::Set => Some(true),
            Check::Clear => Some(false),
            Check::Mixed => None,
        }
    }
}

/// Which control has the cursor. The ring is computed from the platform's
/// flag count, so the same enum serves both layouts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributesField {
    /// The `index`-th box, in [`Flag::GROUPS`] order.
    Flag(usize),
    Modified,
    Accessed,
    Subfolders,
    Set,
    Cancel,
}

impl Default for AttributesField {
    /// The cursor opens on the first box, as Far's does.
    fn default() -> Self {
        Self::Flag(0)
    }
}

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

/// The times the user asked for; `None` leaves that time as it is.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RequestedTimes {
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
}

/// A time field that does not read as a date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidTime {
    pub field: &'static str,
    pub text: String,
}

impl std::fmt::Display for InvalidTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {:?} is not a time ({TIME_FORMAT_HINT})",
            self.field, self.text
        )
    }
}

/// How the time fields read and are written: one format, so what the dialog
/// offers is exactly what it accepts back.
pub const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const TIME_FORMAT_HINT: &str = "YYYY-MM-DD HH:MM:SS";

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
    /// Text of the two time fields; empty leaves that time alone.
    pub modified: String,
    pub accessed: String,
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
        let single = (entries.len() == 1).then(|| &entries[0]);
        Self {
            targets: entries.iter().map(|entry| entry.path.clone()).collect(),
            owner: shared_name(entries.iter().map(|entry| entry.owner.as_deref())),
            group: shared_name(entries.iter().map(|entry| entry.group.as_deref())),
            flags,
            modified: single
                .and_then(|e| e.modified)
                .map(format_time)
                .unwrap_or_default(),
            accessed: single
                .and_then(|e| e.accessed)
                .map(format_time)
                .unwrap_or_default(),
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

    /// The tab ring, top to bottom: every box, the times, the subfolders box
    /// when it applies, then the buttons.
    fn ring(&self) -> Vec<AttributesField> {
        let mut ring: Vec<AttributesField> =
            (0..self.flags.len()).map(AttributesField::Flag).collect();
        ring.push(AttributesField::Modified);
        ring.push(AttributesField::Accessed);
        if self.has_directories {
            ring.push(AttributesField::Subfolders);
        }
        ring.push(AttributesField::Set);
        ring.push(AttributesField::Cancel);
        ring
    }

    /// Tab / Shift+Tab: the next or previous control, wrapping.
    pub fn advance(&mut self, forward: bool) {
        let ring = self.ring();
        self.field = crate::cycle::step(&ring, self.field, forward);
    }

    /// Space: flip the box under the cursor, if it is one.
    pub fn toggle(&mut self) {
        match self.field {
            AttributesField::Flag(index) => {
                if let Some(flag_box) = self.flags.get_mut(index) {
                    flag_box.toggle();
                }
            }
            AttributesField::Subfolders if self.has_directories => {
                self.process_subfolders = !self.process_subfolders;
            }
            _ => {}
        }
    }

    /// The time field under the cursor, if the cursor is on one.
    fn time_field_mut(&mut self) -> Option<&mut String> {
        match self.field {
            AttributesField::Modified => Some(&mut self.modified),
            AttributesField::Accessed => Some(&mut self.accessed),
            _ => None,
        }
    }

    /// A typed character, into the focused time field only.
    pub fn push(&mut self, ch: char) {
        if let Some(field) = self.time_field_mut() {
            field.push(ch);
        }
    }

    pub fn backspace(&mut self) {
        if let Some(field) = self.time_field_mut() {
            field.pop();
        }
    }

    /// A click on the button row: `0` is Set, anything else Cancel.
    pub fn focus_button(&mut self, index: usize) {
        self.field = if index == 0 {
            AttributesField::Set
        } else {
            AttributesField::Cancel
        };
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

    /// The two time fields, parsed; a blank field asks for nothing.
    pub fn requested_times(&self) -> Result<RequestedTimes, InvalidTime> {
        Ok(RequestedTimes {
            modified: parse_time("Modification time", &self.modified)?,
            accessed: parse_time("Access time", &self.accessed)?,
        })
    }
}

/// One box per platform flag, each showing what the selection agrees on.
fn boxes(entries: &[EntrySnapshot], has_directories: bool) -> Vec<FlagBox> {
    Flag::GROUPS
        .iter()
        .flat_map(|group| group.flags.iter().copied())
        .map(|flag| {
            let with_bit = entries
                .iter()
                .filter(|entry| entry.bits & flag.bit() != 0)
                .count();
            let initial = Check::of(with_bit, entries.len());
            FlagBox {
                flag,
                initial,
                current: initial,
                three_state: has_directories || initial == Check::Mixed,
            }
        })
        .collect()
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

/// A timestamp as the field shows it, in local time.
#[must_use]
pub fn format_time(at: SystemTime) -> String {
    chrono::DateTime::<chrono::Local>::from(at)
        .format(TIME_FORMAT)
        .to_string()
}

/// The field's text back into a timestamp; blank means "leave it".
fn parse_time(field: &'static str, text: &str) -> Result<Option<SystemTime>, InvalidTime> {
    let text = text.trim();
    if text.is_empty() {
        return Ok(None);
    }
    let refused = || InvalidTime {
        field,
        text: text.to_owned(),
    };
    let naive = chrono::NaiveDateTime::parse_from_str(text, TIME_FORMAT).map_err(|_| refused())?;
    let local = naive
        .and_local_timezone(chrono::Local)
        .single()
        .ok_or_else(refused)?;
    Ok(Some(SystemTime::from(local)))
}

#[cfg(test)]
mod tests;
