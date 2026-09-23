//! Purpose: Bookmark slots 0-9, the paths saved in them, and the file that
//! keeps them from one run to the next.

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::history::Persistence;

/// Kept for backward compatibility. Prefer `BookmarkSlot::COUNT`.
pub const MAX_BOOKMARKS: usize = BookmarkSlot::COUNT;

/// The file under the data directory, beside the command and folder histories.
const FILE: &str = "bookmarks";

type Slots = [Option<PathBuf>; BookmarkSlot::COUNT];

/// The ten bookmark slots, indexed by [`BookmarkSlot`].
///
/// A fixed-size array rather than a `Vec`: `BookmarkSlot` already proves the
/// index is 0..=9, so with a matching length every lookup is total. That is
/// what removed two identical `if idx >= bookmarks.len()` guards from the
/// dispatcher, and the "defensive check used when the Vec length may not be
/// MAX+1" comment they were written against.
///
/// `persistence` follows the histories' rule: `default()` is session-only, so
/// constructing an `App` never touches the user's real file; only
/// [`Bookmarks::load`] at the production entry point names one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmarks {
    slots: Slots,
    persistence: Persistence,
}

impl Default for Bookmarks {
    fn default() -> Self {
        Self {
            slots: std::array::from_fn(|_| None),
            persistence: Persistence::SessionOnly,
        }
    }
}

impl Bookmarks {
    /// Read the saved slots and remember where they came from. A missing or
    /// unreadable file is ten empty slots; bookmarks must never break the
    /// file manager.
    #[must_use]
    pub fn load() -> Self {
        let persistence =
            crate::history::data_file(FILE).map_or(Persistence::SessionOnly, Persistence::File);
        let slots = persistence
            .file()
            .map_or_else(|| std::array::from_fn(|_| None), read_slots);
        Self { slots, persistence }
    }

    /// The path saved in `slot`, if any. Cannot fail: the slot proves itself.
    #[must_use]
    pub fn get(&self, slot: BookmarkSlot) -> Option<&Path> {
        self.slots[slot.as_usize()].as_deref()
    }

    pub fn set(&mut self, slot: BookmarkSlot, path: PathBuf) {
        self.slots[slot.as_usize()] = Some(path);
    }

    pub fn clear(&mut self, slot: BookmarkSlot) {
        self.slots[slot.as_usize()] = None;
    }

    /// Every slot in order, for the completer's `bookmark go` candidates.
    #[must_use]
    pub fn slots(&self) -> &[Option<PathBuf>] {
        &self.slots
    }

    /// `bookmark set`: save `path` in `slot` and write the file.
    ///
    /// The file is read again first, so a slot another running ffm has set
    /// since this one started is kept instead of being overwritten with this
    /// instance's stale copy.
    pub fn remember(&mut self, slot: BookmarkSlot, path: PathBuf) -> io::Result<()> {
        if let Some(file) = self.persistence.file() {
            self.slots = read_slots(file);
        }
        self.set(slot, path);
        self.save()
    }

    /// Write every saved slot, one `<slot> <path>` line each, the way the
    /// histories write theirs: owner-only, through an atomic rename. A path
    /// with a line break cannot be one line of the file, so it is left out.
    fn save(&self) -> io::Result<()> {
        let Some(file) = self.persistence.file() else {
            return Ok(());
        };
        if let Some(dir) = file.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut text = String::new();
        for (slot, path) in self.slots.iter().enumerate() {
            let Some(path) = path.as_deref().map(Path::to_string_lossy) else {
                continue;
            };
            if path.contains(['\n', '\r']) {
                continue;
            }
            text.push_str(&slot.to_string());
            text.push(' ');
            text.push_str(&path);
            text.push('\n');
        }
        crate::history::write_owner_only(file, &text)
    }
}

/// The slots saved in `file`. A line that is not a slot digit, a space and an
/// absolute path is skipped, so a damaged file costs only its damaged lines.
fn read_slots(file: &Path) -> Slots {
    let mut slots: Slots = std::array::from_fn(|_| None);
    let text = fs::read_to_string(file).unwrap_or_default();
    for (slot, path) in text.lines().filter_map(parse_line) {
        slots[slot.as_usize()] = Some(path);
    }
    slots
}

fn parse_line(line: &str) -> Option<(BookmarkSlot, PathBuf)> {
    let (digit, path) = line.split_once(' ')?;
    let slot = BookmarkSlot::from_usize(digit.parse().ok()?).ok()?;
    let path = PathBuf::from(path);
    path.is_absolute().then_some((slot, path))
}

const SLOT_OUT_OF_RANGE: &str = "bookmark slot must be 0-9";

/// Domain-specific type for a bookmark slot (0-9).
/// Replaces raw `usize` / magic numbers for bookmark operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BookmarkSlot(u8);

impl BookmarkSlot {
    /// Maximum valid slot (inclusive).
    pub const MAX: u8 = 9;
    /// Total number of bookmark slots available.
    pub const COUNT: usize = (Self::MAX as usize) + 1;

    /// Create a new slot. Returns error for values outside 0..=9.
    pub const fn new(n: u8) -> Result<Self, &'static str> {
        if n <= Self::MAX {
            Ok(Self(n))
        } else {
            Err(SLOT_OUT_OF_RANGE)
        }
    }

    /// Create from usize (common in parsing). Returns error if out of range.
    pub fn from_usize(n: usize) -> Result<Self, &'static str> {
        u8::try_from(n).map_or(Err(SLOT_OUT_OF_RANGE), Self::new)
    }

    pub const fn as_u8(self) -> u8 {
        self.0
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl std::fmt::Display for BookmarkSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests;
