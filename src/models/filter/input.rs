//! Cursor and focus for the open filter dialog.

use crate::lineedit::LineEditor;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FilterField {
    #[default]
    Include,
    Additional,
    Exclude,
    Hidden,
    Apply,
    Cancel,
}

impl FilterField {
    pub(crate) const fn advance(self, forward: bool, additional: bool) -> Self {
        match (self, forward, additional) {
            (Self::Include, true, true) | (Self::Exclude, false, true) => Self::Additional,
            (Self::Additional, true, _)
            | (Self::Include, true, false)
            | (Self::Hidden, false, _) => Self::Exclude,
            (Self::Additional, false, _)
            | (Self::Exclude, false, false)
            | (Self::Cancel, true, _) => Self::Include,
            (Self::Exclude, true, _) | (Self::Apply, false, _) => Self::Hidden,
            (Self::Hidden, true, _) | (Self::Cancel, false, _) => Self::Apply,
            (Self::Apply, true, _) | (Self::Include, false, _) => Self::Cancel,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct FilterInput {
    pub field: FilterField,
    pub include: LineEditor,
    pub additional: Option<LineEditor>,
    pub exclude: LineEditor,
    /// Existing `PanelFilter` includes and the older options mask remain independent.
    pub edit_panel_include: bool,
    pub error: Option<String>,
}
