//! Purpose: An entry's timestamp as the panel's Date and Time columns show it,
//! degrading to blank when the filesystem will not say.

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
