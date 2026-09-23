//! Purpose: Command-line state: the editable line, its completion dropdown, and history recall.

use crate::history::RingIndex;
use crate::lineedit::LineEditor;

/// What a completion candidate is, which decides how accepting it ends.
///
/// A `Word` gets a trailing space; a `Path` gets one too unless it is a
/// directory (ending in `/`), where the user is about to descend into it.
/// Lives here rather than in `complete` so `models` stays dependency-free.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    Word,
    Path,
}

/// One row of the completion dropdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    /// Replacement text for the token being completed.
    pub text: String,
    /// Decides the trailing space when this row is accepted.
    pub kind: CandidateKind,
    /// Extra detail shown right-aligned — e.g. the path behind a bookmark slot.
    pub hint: Option<String>,
}

/// Open completion dropdown: the candidates and which one is highlighted.
///
/// Both fields are private because they share an invariant: the dropdown is
/// **never empty**, and `selected` always points at one of its rows. As public
/// fields, replacing `candidates` without resetting `selected` left the
/// highlight pointing past the end — safe only because the one code path that
/// refreshes a menu happens to rebuild the whole struct. [`Menu::new`] rejects
/// an empty candidate list instead, which is what makes [`Menu::selection`]
/// able to return a `&Candidate` rather than an `Option`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    candidates: Vec<Candidate>,
    selected: usize,
}

impl Menu {
    /// A dropdown over `candidates`, highlighting the best-ranked row.
    /// `None` for an empty list: there is no dropdown to show.
    #[must_use]
    pub fn new(candidates: Vec<Candidate>) -> Option<Self> {
        (!candidates.is_empty()).then_some(Self {
            candidates,
            selected: 0,
        })
    }

    #[must_use]
    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
    }

    #[must_use]
    pub const fn selected(&self) -> usize {
        self.selected
    }

    /// The highlighted candidate. Total: the list cannot be empty.
    #[must_use]
    #[expect(
        clippy::indexing_slicing,
        reason = "`new` refuses an empty list and `next`/`previous` keep `selected` below its length"
    )]
    pub fn selection(&self) -> &Candidate {
        &self.candidates[self.selected]
    }

    /// Highlight the next candidate, wrapping at the end.
    pub const fn next(&mut self) {
        self.selected = (self.selected + 1) % self.candidates.len();
    }

    /// Highlight the previous candidate, wrapping at the start.
    pub fn previous(&mut self) {
        self.selected = self
            .selected
            .checked_sub(1)
            .unwrap_or(self.candidates.len() - 1);
    }
}

/// How the command line is currently consulting the history ring.
///
/// An enum rather than an `Option<usize>` beside an `Option<String>` beside an
/// `Option<query>`, for the reason [`Overlay`](super::Overlay) is one: browsing with `Up`/`Down`
/// and reverse-searching with `Ctrl+R` are mutually exclusive, and `stash` — the
/// half-typed line that the first `Up` displaced — is meaningless outside
/// `Browsing`. Three `Option`s would make "browsing while searching" and
/// "browsing with no stash" representable states nothing knows how to render.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Recall {
    /// Not consulting the history; the buffer is whatever was typed.
    #[default]
    Idle,
    /// Walking back through the ring. `index` is the offset from the newest
    /// entry; `stash` is the line that was being typed when `Up` was first
    /// pressed, restored on walking forward past the newest entry.
    Browsing { index: RingIndex, stash: String },
    /// `Ctrl+R` reverse search. `hit` is the ring index currently shown, or
    /// `None` when nothing matches the query yet; `stash` is the line the
    /// search displaced, put back when the search is abandoned.
    Searching {
        query: String,
        hit: Option<RingIndex>,
        stash: String,
    },
}

/// State of the `:` command line: the editable line, any open completion
/// dropdown, the ghost suggestion, and the history-recall mode.
///
/// A struct rather than a bare `String` because the completion dropdown's
/// state belongs beside the buffer it completes: the two are edited by the
/// same keystrokes and must never drift apart. The line itself is a
/// [`LineEditor`] rather than a `String` so the caret travels with the text —
/// a separate cursor field could be left pointing past a shortened buffer.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandLine {
    pub line: LineEditor,
    /// `None` while the dropdown is closed.
    pub menu: Option<Menu>,
    /// History suggestion tail beyond the typed text (grey text after the
    /// cursor). `None` when nothing matches, while the menu is open, or during
    /// a reverse search — the menu and the ghost must never compete for the
    /// same accept gesture.
    pub ghost: Option<String>,
    /// Whether `Up`/`Down` or `Ctrl+R` is currently driving the buffer.
    pub recall: Recall,
}

impl CommandLine {
    /// The text typed so far.
    #[must_use]
    pub fn text(&self) -> &str {
        self.line.text()
    }
}

impl From<String> for CommandLine {
    fn from(buffer: String) -> Self {
        Self {
            line: LineEditor::from(buffer),
            menu: None,
            ghost: None,
            recall: Recall::Idle,
        }
    }
}

impl From<&str> for CommandLine {
    fn from(buffer: &str) -> Self {
        Self {
            line: LineEditor::from(buffer),
            menu: None,
            ghost: None,
            recall: Recall::Idle,
        }
    }
}
