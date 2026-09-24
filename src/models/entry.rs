//! Purpose: What a panel row knows about the file it names — kind, sizes,
//! timestamps, attributes, owner and description — captured once per listing.
//!
//! Split out of `models.rs`, which sits under the 1000-line cap that
//! `tests/file_line_limit_test.rs` enforces. Everything here is a fact the
//! filesystem reported when the directory was read; nothing here queries the
//! disk while a row is drawn.

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
}

impl DirEntry {
    /// The name that reaches this entry on disk.
    #[must_use]
    pub fn disk_name(&self) -> &std::ffi::OsStr {
        self.on_disk
            .as_deref()
            .unwrap_or_else(|| std::ffi::OsStr::new(&self.name))
    }
}

/// One of an entry's timestamps — modified, created or accessed — as the
/// panel's Date and Time columns show it.
///
/// Wraps an `Option` because a listing has to survive a `stat` the filesystem
/// refuses to answer: one unreadable entry blanks its own two columns instead
/// of dropping the file out of the panel, which is what a plain `SystemTime`
/// field would have forced (`unwrap_or(UNIX_EPOCH)` lies — 01/01/70 is a real
/// date some archives genuinely carry). Ordered so the time-based sort modes
/// can compare rows directly; an unknown time sorts before every known one.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Modified(Option<std::time::SystemTime>);

impl Modified {
    /// No readable timestamp: both columns render empty.
    pub const UNKNOWN: Self = Self(None);

    #[must_use]
    pub const fn new(at: std::time::SystemTime) -> Self {
        Self(Some(at))
    }

    /// Read an entry's mtime, degrading to [`Self::UNKNOWN`] rather than
    /// failing the whole listing — `modified()` is unsupported on some
    /// filesystems and refused on others.
    #[must_use]
    pub fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        Self::from_result(metadata.modified())
    }

    /// The same degradation for `created()` and `accessed()`, which fail on
    /// even more filesystems than `modified()` does.
    #[must_use]
    pub fn from_result(at: std::io::Result<std::time::SystemTime>) -> Self {
        at.map_or(Self::UNKNOWN, Self::new)
    }

    fn local(self) -> Option<chrono::DateTime<chrono::Local>> {
        self.0.map(chrono::DateTime::<chrono::Local>::from)
    }

    /// `MM/DD/YY`, or empty when unknown. The caller pads to the column width:
    /// an empty string right-aligns to blanks, which is what an unreadable
    /// timestamp should look like.
    #[must_use]
    pub fn date(self) -> String {
        self.local()
            .map(|at| at.format("%m/%d/%y").to_string())
            .unwrap_or_default()
    }

    /// `HH:MM` in 24-hour form, or empty when unknown.
    #[must_use]
    pub fn time(self) -> String {
        self.local()
            .map(|at| at.format("%H:%M").to_string())
            .unwrap_or_default()
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

/// A file's size in bytes.
///
/// Named because a bare `u64` in this program is just as likely to be a line
/// count, a byte offset or a cell budget — and because the megabyte conversion
/// and the human-readable rendering belong with the number rather than in free
/// functions anyone can call with any integer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct FileSize(u64);

impl FileSize {
    #[must_use]
    pub const fn from_bytes(bytes: u64) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn bytes(self) -> u64 {
        self.0
    }

    /// The size in megabytes — the unit the status bar reports in.
    ///
    /// Whole mebibytes and the remainder each convert to `f64` exactly, so the
    /// result is exact; it saturates at `u32::MAX` MiB (4 PiB).
    #[must_use]
    pub fn megabytes(self) -> f64 {
        const MIB: u32 = 1 << 20;
        let Ok(whole) = u32::try_from(self.0 / u64::from(MIB)) else {
            return f64::from(u32::MAX);
        };
        // Below `MIB`, so it always fits; the fallback is never taken.
        let rest = u32::try_from(self.0 % u64::from(MIB)).unwrap_or(0);
        f64::from(whole) + f64::from(rest) / f64::from(MIB)
    }
}

impl std::fmt::Display for FileSize {
    /// Human-readable, as the panel's size column shows it: "1.50 KB",
    /// "3.20 MB", or a bare byte count below a kibibyte.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const UNITS: [(u64, &str); 3] = [(1 << 30, "GB"), (1 << 20, "MB"), (1 << 10, "KB")];

        let size = self.0;
        let Some(&(unit, name)) = UNITS.iter().find(|&&(unit, _)| size >= unit) else {
            return write!(f, "{size} B");
        };
        let hundredths = divide_rounded(u128::from(size) * 100, u128::from(unit));
        write!(f, "{}.{:02} {name}", hundredths / 100, hundredths % 100)
    }
}

/// `numerator / denominator` to the nearest integer, ties to even.
///
/// That is the rule `format!("{:.2}", x)` applies to an exact value, so a size
/// scaled with integers prints the digits the float formatting printed, without
/// `u64 as f64` rounding the byte count above 2^53. `denominator` is non-zero.
pub fn divide_rounded(numerator: u128, denominator: u128) -> u128 {
    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let to_next = denominator - remainder;
    let rounds_up = remainder > to_next || (remainder == to_next && quotient % 2 == 1);
    quotient + u128::from(rounds_up)
}

/// An entry's attribute word: the Unix mode (`st_mode`) or the Windows
/// attribute flags, whichever the listing read.
///
/// One newtype for both because the row stores exactly one `u32` either way
/// and only the rendering differs — `drwxr-xr-x` from mode bits, `RHSA`
/// letters from flags. Both renderers are public so the Windows one can be
/// tested on a Unix host; `Display` picks the platform's own.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Attributes(u32);

impl Attributes {
    const WINDOWS_LETTERS: [(u32, char); 4] = [(0x1, 'R'), (0x2, 'H'), (0x4, 'S'), (0x20, 'A')];
    const UNIX_TYPES: [(u32, char); 7] = [
        (0o140_000, 's'),
        (0o120_000, 'l'),
        (0o100_000, '-'),
        (0o060_000, 'b'),
        (0o040_000, 'd'),
        (0o020_000, 'c'),
        (0o010_000, 'p'),
    ];

    #[must_use]
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    /// The platform's attribute word for `metadata`: the mode on Unix, the
    /// attribute flags on Windows, and nothing anywhere else.
    #[must_use]
    pub fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            Self(metadata.mode())
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            Self(metadata.file_attributes())
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = metadata;
            Self::default()
        }
    }

    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    /// `ls -l` style: the file type, then owner, group and other permissions,
    /// with the setuid, setgid and sticky bits in their usual places.
    #[must_use]
    pub fn mode_string(self) -> String {
        let bits = self.0;
        let kind = Self::UNIX_TYPES
            .iter()
            .find(|(mask, _)| bits & 0o170_000 == *mask)
            .map_or('?', |(_, letter)| *letter);
        let mut out = String::with_capacity(10);
        out.push(kind);
        for (shift, special, special_letter) in
            [(6, 0o4000, 's'), (3, 0o2000, 's'), (0, 0o1000, 't')]
        {
            let triplet = (bits >> shift) & 0o7;
            out.push(if triplet & 0o4 != 0 { 'r' } else { '-' });
            out.push(if triplet & 0o2 != 0 { 'w' } else { '-' });
            out.push(execute_letter(
                triplet & 0o1 != 0,
                bits & special != 0,
                special_letter,
            ));
        }
        out
    }

    /// Far's attribute letters: `R`ead-only, `H`idden, `S`ystem, `A`rchive —
    /// those set, in that order.
    #[must_use]
    pub fn flag_letters(self) -> String {
        Self::WINDOWS_LETTERS
            .iter()
            .filter(|(flag, _)| self.0 & flag != 0)
            .map(|(_, letter)| *letter)
            .collect()
    }
}

/// The third character of a permission triplet: `x` when executable, or the
/// special bit's letter — uppercase when the special bit is set without it.
fn execute_letter(executable: bool, special: bool, letter: char) -> char {
    match (executable, special) {
        (true, true) => letter,
        (false, true) => letter.to_ascii_uppercase(),
        (true, false) => 'x',
        (false, false) => '-',
    }
}

impl std::fmt::Display for Attributes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if cfg!(windows) {
            f.write_str(&self.flag_letters())
        } else {
            f.write_str(&self.mode_string())
        }
    }
}

#[cfg(test)]
mod tests;
