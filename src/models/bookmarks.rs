//! Purpose: Bookmark slots 0-9 and the paths saved in them.

use std::path::{Path, PathBuf};

/// Kept for backward compatibility. Prefer `BookmarkSlot::COUNT`.
pub const MAX_BOOKMARKS: usize = BookmarkSlot::COUNT;

/// The ten bookmark slots, indexed by [`BookmarkSlot`].
///
/// A fixed-size array rather than a `Vec`: `BookmarkSlot` already proves the
/// index is 0..=9, so with a matching length every lookup is total. That is
/// what removed two identical `if idx >= bookmarks.len()` guards from the
/// dispatcher, and the "defensive check used when the Vec length may not be
/// MAX+1" comment they were written against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bookmarks([Option<PathBuf>; BookmarkSlot::COUNT]);

impl Default for Bookmarks {
    fn default() -> Self {
        Self(std::array::from_fn(|_| None))
    }
}

impl Bookmarks {
    /// The path saved in `slot`, if any. Cannot fail: the slot proves itself.
    #[must_use]
    pub fn get(&self, slot: BookmarkSlot) -> Option<&Path> {
        self.0[slot.as_usize()].as_deref()
    }

    pub fn set(&mut self, slot: BookmarkSlot, path: PathBuf) {
        self.0[slot.as_usize()] = Some(path);
    }

    pub fn clear(&mut self, slot: BookmarkSlot) {
        self.0[slot.as_usize()] = None;
    }

    /// Every slot in order, for the completer's `bookmark go` candidates.
    #[must_use]
    pub fn slots(&self) -> &[Option<PathBuf>] {
        &self.0
    }
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
