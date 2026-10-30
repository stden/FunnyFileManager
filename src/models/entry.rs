//! Purpose: What a panel row knows about the file it names — kind, sizes,
//! timestamps, attributes, owner and description — captured once per listing.
//!
//! Split out of `models.rs`, which sits under the 1000-line cap that
//! `tests/file_line_limit_test.rs` enforces. Everything here is a fact the
//! filesystem reported when the directory was read; nothing here queries the
//! disk while a row is drawn.

mod attributes;
mod file_size;
mod modified;

pub use attributes::Attributes;
pub use file_size::{divide_rounded, FileSize};
pub use modified::Modified;

/// A single entry in the file list (file or directory)
///
/// `is_dir` describes what the entry *behaves* like — a symlink to a directory
/// is entered, copied and deleted as a directory would be — while `is_symlink`
/// records that the entry is a link, which the raw `is_dir` flag would
/// otherwise lose. Both are needed: a broken link is neither a directory nor a
/// plain file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub kind: EntryKind,
    pub size: FileSize,
    pub modified: Modified,
    /// Whether the file can be run, which Far colours green.
    ///
    /// Resolved when the directory is listed, not when the row is drawn: on
    /// Unix the answer is in the mode bits the listing already read, and
    /// re-`stat`ing every visible row on every keypress to recover it would be
    /// paying twice for a fact we had.
    pub executable: bool,
    /// Hard-link count, Far's "File links" column. Always 1 where the
    /// filesystem does not count them.
    pub links: u32,
    /// The owning user's name, Far's "File owners" column. `None` where the
    /// platform has no cheap answer, or the uid has no passwd entry.
    pub owner: Option<String>,
    /// Bytes the file occupies on disk, as opposed to its length: sparse files
    /// take less, small files take a whole block. Far's "Detailed" mode shows
    /// both side by side.
    pub allocated: FileSize,
    /// When the entry was created, or [`Modified::UNKNOWN`] where the
    /// filesystem does not record it.
    pub created: Modified,
    /// When the entry was last read.
    pub accessed: Modified,
    /// Mode bits or attribute flags, rendered per platform.
    pub attributes: Attributes,
    /// The line the directory's description file gives this entry, if any.
    pub description: Option<String>,
    /// The name on disk, kept only when it is not UTF-8 and [`Self::name`] is
    /// therefore a lossy rendering of it (`caf\xE9.txt` shown as `caf�.txt`).
    /// Paths are built from [`Self::disk_name`], never from the display name.
    pub on_disk: Option<std::ffi::OsString>,
    /// Recursive byte total of a folder when measured by `F3`, or
    /// [`DirectorySize::Unmeasured`] until then.
    pub directory_size: DirectorySize,
}

/// Whether a directory's recursive byte total has been measured (`F3`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum DirectorySize {
    #[default]
    Unmeasured,
    Measured(FileSize),
}

impl DirectorySize {
    #[must_use]
    pub const fn is_measured(self) -> bool {
        matches!(self, Self::Measured(_))
    }

    #[must_use]
    pub const fn measured(self) -> Option<FileSize> {
        match self {
            Self::Unmeasured => None,
            Self::Measured(size) => Some(size),
        }
    }
}

impl DirEntry {
    /// The name that reaches this entry on disk.
    #[must_use]
    pub fn disk_name(&self) -> &std::ffi::OsStr {
        self.on_disk
            .as_deref()
            .unwrap_or_else(|| std::ffi::OsStr::new(&self.name))
    }

    /// Record a folder's measured recursive total size from `F3`.
    pub const fn set_measured_size(&mut self, size: FileSize) {
        self.size = size;
        self.allocated = size;
        self.directory_size = DirectorySize::Measured(size);
    }
}

/// What a panel row *is*, as far as every operation on it is concerned.
///
/// This replaced a pair of `bool`s whose reading needed two paragraphs of
/// prose: `is_dir` described the symlink's *target* while `is_symlink`
/// recorded the link, and a broken link had to keep `is_dir == false`. Four
/// combinations, three explanations, and one shipped bug — a symlinked
/// directory rendered as a tiny file that could not be opened but whose whole
/// target tree got copied by F5. The states are named now.
///
/// The old encoding also could not tell a symlink-to-a-file from a dangling
/// one; both were `(false, true)`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum EntryKind {
    #[default]
    File,
    Directory,
    /// A symlink whose target is a file.
    SymlinkToFile,
    /// A symlink whose target is a directory: entered, copied and deleted as a
    /// directory would be.
    SymlinkToDirectory,
    /// A symlink with nothing at the other end. Not a directory — there is
    /// nothing to enter — and not a file either.
    BrokenSymlink,
}

impl EntryKind {
    /// Whether this behaves as a directory: `Enter` descends into it, and file
    /// operations recurse through it.
    #[must_use]
    pub const fn is_dir(self) -> bool {
        matches!(self, Self::Directory | Self::SymlinkToDirectory)
    }

    /// Whether a link is involved, whatever it points at.
    #[must_use]
    pub const fn is_symlink(self) -> bool {
        matches!(
            self,
            Self::SymlinkToFile | Self::SymlinkToDirectory | Self::BrokenSymlink
        )
    }
}

#[cfg(test)]
mod tests;
