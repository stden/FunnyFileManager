//! Purpose: The F7 search dialog — its controls, the typed query, and the search it becomes.

use super::{Case, Direction, Needle, Search, WordMatch};

/// Which control the F7 dialog's cursor is on.
///
/// An enum rather than an index, for the same reason [`crate::models::
/// ExitChoice`] is one: an index has states naming no control at all, and
/// every reader of it then has to defend against them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SearchField {
    #[default]
    Query,
    Case,
    Words,
    /// Far gives the direction two buttons rather than a checkbox, so the
    /// choice and the command are one keystroke instead of two.
    FindPrevious,
    FindNext,
    Cancel,
}

impl SearchField {
    /// The controls in the order Tab walks them.
    pub(super) const ORDER: [Self; 6] = [
        Self::Query,
        Self::Case,
        Self::Words,
        Self::FindPrevious,
        Self::FindNext,
        Self::Cancel,
    ];

    #[must_use]
    pub fn next(self) -> Self {
        self.round(1)
    }

    #[must_use]
    pub fn previous(self) -> Self {
        self.round(Self::ORDER.len() - 1)
    }

    /// The control `steps` places further round the Tab ring. The index is
    /// taken modulo the ring's length, so the fallback is never used.
    fn round(self, steps: usize) -> Self {
        let at = Self::ORDER.iter().position(|f| *f == self).unwrap_or(0);
        Self::ORDER
            .get((at + steps) % Self::ORDER.len())
            .copied()
            .unwrap_or(self)
    }

    /// Whether this control is a checkbox, which Space toggles.
    #[must_use]
    pub const fn is_toggle(self) -> bool {
        matches!(self, Self::Case | Self::Words)
    }

    /// The direction this control commits to, if it is one of the two Find
    /// buttons. `None` for everything else, which leaves the direction alone.
    #[must_use]
    pub const fn direction(self) -> Option<Direction> {
        match self {
            Self::FindPrevious => Some(Direction::Backward),
            Self::FindNext => Some(Direction::Forward),
            _ => None,
        }
    }
}

/// The F7 dialog while it is open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchPrompt {
    /// What has been typed so far — a `String`, not a [`Needle`], because the
    /// dialog exists precisely in order to start out empty.
    pub query: String,
    pub case: Case,
    pub words: WordMatch,
    pub direction: Direction,
    pub field: SearchField,
}

impl SearchPrompt {
    /// Reopen the dialog on the previous search, as Far does — a repeat search
    /// with one option changed is the common case, and retyping the query to
    /// get there is the annoying one.
    #[must_use]
    pub fn from_last(last: Option<&Search>) -> Self {
        last.map_or_else(Self::default, |search| Self {
            query: search.needle.as_str().to_owned(),
            case: search.case,
            words: search.words,
            direction: search.direction,
            field: SearchField::default(),
        })
    }

    /// The search this describes, or `None` while the query is still empty.
    #[must_use]
    pub fn to_search(&self) -> Option<Search> {
        Some(Search {
            needle: Needle::new(&self.query)?,
            case: self.case,
            words: self.words,
            direction: self.direction,
        })
    }

    /// Flip whichever checkbox the cursor is on. Does nothing elsewhere, so
    /// Space stays a typeable character in the query field.
    pub fn toggle(&mut self) {
        match self.field {
            SearchField::Case => {
                self.case = match self.case {
                    Case::Ignore => Case::Match,
                    Case::Match => Case::Ignore,
                }
            }
            SearchField::Words => {
                self.words = match self.words {
                    WordMatch::Anywhere => WordMatch::WholeWords,
                    WordMatch::WholeWords => WordMatch::Anywhere,
                }
            }
            SearchField::Query
            | SearchField::FindPrevious
            | SearchField::FindNext
            | SearchField::Cancel => {}
        }
    }
}

/// The editor's search across both its lives: being described in the dialog,
/// and being repeated afterwards with Shift+F7.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SearchState {
    /// `Some` exactly while the F7 dialog is on screen.
    pub prompt: Option<SearchPrompt>,
    /// What Shift+F7 repeats. Outlives the dialog on purpose.
    pub last: Option<Search>,
    /// Set when a search found nothing, so the editor can say so — otherwise
    /// a failed search is indistinguishable from a key that did not register.
    pub not_found: Option<String>,
}
