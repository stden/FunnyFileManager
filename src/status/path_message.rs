//! Purpose: `PathMessage` — a status message split around the path it quotes — and `Status::path_message`, which asks each outcome family for one.

use std::borrow::Cow;
use std::fmt;

use super::Status;

mod operations;
mod places;

/// A message split around the path it quotes: the words before it, the path
/// as the message spells it, and the words after it. `path` is empty for a
/// message that quotes none.
///
/// Three parts rather than one line so a row too narrow for the whole message
/// can shorten the *path* — from its head, keeping the file name — and leave
/// the reason whole: `Cannot edit …/unreadable.txt: Permission denied`. A
/// line already flattened to one string can only be cut at its end, and the
/// end is where the file name and the reason are.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathMessage<'a> {
    pub before: Cow<'a, str>,
    pub path: Cow<'a, str>,
    pub after: Cow<'a, str>,
}

impl<'a> PathMessage<'a> {
    /// A message quoting `path` between `before` and `after`; the words may
    /// be literals or formatted.
    pub fn quoting(
        before: impl Into<Cow<'a, str>>,
        path: impl Into<Cow<'a, str>>,
        after: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self {
            before: before.into(),
            path: path.into(),
            after: after.into(),
        }
    }
}

/// A message that quotes no path: all of it is `before`.
impl From<String> for PathMessage<'static> {
    fn from(text: String) -> Self {
        Self::quoting(text, "", "")
    }
}

impl fmt::Display for PathMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}{}", self.before, self.path, self.after)
    }
}

impl Status {
    /// This outcome's message split around the path it quotes, or `None` for
    /// one that never quotes a path (every outcome the families below leave
    /// out). An outcome that quotes one only sometimes — a copy of several
    /// entries counts them — has an empty `path` then. The one place these
    /// messages are worded: `Display` prints exactly these parts.
    #[must_use]
    pub fn path_message(&self) -> Option<PathMessage<'_>> {
        self.view_path_message()
            .or_else(|| self.edit_path_message())
            .or_else(|| self.file_path_message())
            .or_else(|| self.archive_path_message())
            .or_else(|| self.run_path_message())
            .or_else(|| self.failure_path_message())
            .or_else(|| self.attributes_path_message())
    }
}
