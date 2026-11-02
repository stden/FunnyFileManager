//! Purpose: The incremental `Alt+<letter>` quick-search query and its rules.
//!
//! Pure: no `App`, no panel, no I/O. The caller owns the matching — it is the
//! panel that knows which names exist — and this owns *what the query is* after
//! each keystroke, which is the part that had grown five branches inside the
//! event loop's action dispatcher.

/// The quick-search query, or its absence.
///
/// An enum rather than an `Option<String>` because "searching" is a mode with
/// rules of its own, and those rules used to live inline in `apply_action`:
/// the restart-on-miss below is the whole reason a bare `Option` was not
/// enough. `Active` never holds an empty string — [`Self::backspace`] returns
/// to `Idle` instead — so "searching for nothing" is not representable.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum QuickSearch {
    /// No search in progress.
    #[default]
    Idle,
    /// A non-empty query the panel is being matched against.
    Active(String),
}

impl QuickSearch {
    /// The query, or `None` while idle.
    #[must_use]
    pub fn query(&self) -> Option<&str> {
        match self {
            Self::Idle => None,
            Self::Active(query) => Some(query),
        }
    }

    #[must_use]
    pub const fn is_active(&self) -> bool {
        matches!(self, Self::Active(_))
    }

    /// Append `ch` and return the query to match against.
    pub fn push(&mut self, ch: char) -> &str {
        match self {
            Self::Idle => *self = Self::Active(ch.to_string()),
            Self::Active(query) => query.push(ch),
        }
        self.query().unwrap_or_default()
    }

    /// Start over from `ch` alone, and return that query.
    ///
    /// What `Alt+<letter>` does when the extended query matches nothing: the
    /// user has almost certainly moved on to a different file rather than typed
    /// a name that does not exist, so the new character starts a fresh search.
    pub fn restart_with(&mut self, ch: char) -> &str {
        *self = Self::Active(ch.to_string());
        self.query().unwrap_or_default()
    }

    /// Whether a failed match should restart from the last character.
    ///
    /// Only for a query of **more than one character**: restarting a one-char
    /// query with the same character would search for it again and again.
    /// Counted in `char`s, not bytes — a single Cyrillic or CJK letter is two
    /// or three bytes, and a byte-length test called it a long query.
    #[must_use]
    pub fn should_restart(&self) -> bool {
        self.query().is_some_and(|query| query.chars().count() > 1)
    }

    /// Drop the last character, returning to [`Self::Idle`] when none is left.
    /// Returns the remaining query, or `None` once the search has ended.
    pub fn backspace(&mut self) -> Option<&str> {
        let Self::Active(query) = self else {
            return None;
        };
        query.pop();
        if query.is_empty() {
            *self = Self::Idle;
            return None;
        }
        self.query()
    }

    /// End the search.
    pub fn clear(&mut self) {
        *self = Self::Idle;
    }
}

#[cfg(test)]
mod tests;
