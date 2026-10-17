//! PATH drafts keep user and system values separate from the process environment.
use crate::lineedit::LineEditor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    User,
    System,
}

impl Scope {
    pub const fn label(self) -> &'static str {
        match self {
            Self::User => "current user",
            Self::System => "all users",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Validation {
    Directory,
    Missing,
    NotDirectory,
    Unresolved,
    Relative,
    Empty,
}

impl Validation {
    pub const fn label(&self) -> &'static str {
        match self {
            Self::Directory => "OK",
            Self::Missing => "MISSING",
            Self::NotDirectory => "NOT A DIRECTORY",
            Self::Unresolved => "UNKNOWN VARIABLE",
            Self::Relative => "NOT ABSOLUTE",
            Self::Empty => "EMPTY",
        }
    }
}

#[derive(Debug, Clone)]
pub struct PathEntry {
    pub text: String,
    pub validation: Validation,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub value: String,
    pub revision: String,
    pub location: String,
}

#[derive(Debug, Clone)]
pub enum Mode {
    Browse,
    Edit {
        index: Option<usize>,
        input: LineEditor,
    },
    ConfirmSave,
    ConfirmDiscard,
}

#[derive(Debug, Clone)]
pub struct PathEditor {
    pub scope: Scope,
    pub snapshot: Snapshot,
    pub entries: Vec<PathEntry>,
    pub selected: usize,
    pub mode: Mode,
    pub message: String,
}

impl PathEditor {
    pub(crate) fn new(scope: Scope, snapshot: Snapshot) -> Self {
        let entries = if snapshot.value.is_empty() {
            Vec::new()
        } else {
            snapshot
                .value
                .split(separator())
                .map(|text| PathEntry {
                    text: text.to_owned(),
                    validation: Validation::Unresolved,
                })
                .collect()
        };
        Self {
            scope,
            snapshot,
            entries,
            selected: 0,
            mode: Mode::Browse,
            message: String::new(),
        }
    }

    pub(crate) fn value(&self) -> String {
        self.entries
            .iter()
            .map(|entry| entry.text.as_str())
            .collect::<Vec<_>>()
            .join(&separator().to_string())
    }

    pub(crate) fn dirty(&self) -> bool {
        self.value() != self.snapshot.value
    }

    pub(crate) fn edit(&mut self, index: Option<usize>) {
        let value = index
            .and_then(|i| self.entries.get(i))
            .map_or("", |entry| entry.text.as_str());
        self.mode = Mode::Edit {
            index,
            input: LineEditor::new(value),
        };
    }

    pub(crate) fn remove(&mut self) {
        if self.selected < self.entries.len() {
            self.entries.remove(self.selected);
        }
        self.selected = self.selected.min(self.entries.len().saturating_sub(1));
    }

    pub(crate) fn move_selection(&mut self, down: bool, reorder: bool) {
        let next = if down {
            self.selected
                .saturating_add(1)
                .min(self.entries.len().saturating_sub(1))
        } else {
            self.selected.saturating_sub(1)
        };
        if reorder && next != self.selected {
            self.entries.swap(next, self.selected);
        }
        self.selected = next;
    }

    /// The line being edited, when the dialog is in edit mode.
    pub(crate) fn focused_line_mut(&mut self) -> Option<&mut LineEditor> {
        match &mut self.mode {
            Mode::Edit { input, .. } => Some(input),
            Mode::Browse | Mode::ConfirmSave | Mode::ConfirmDiscard => None,
        }
    }
}

pub(crate) const fn separator() -> char {
    if cfg!(windows) {
        ';'
    } else {
        ':'
    }
}
