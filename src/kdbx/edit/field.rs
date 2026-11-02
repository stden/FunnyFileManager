//! Purpose: the entry form's field tab order and its editing / confirm mode.

/// Which form field has the cursor.
///
/// An enum rather than an index, for the reason `crate::models::NameField` is
/// one: the tab order is stated once, and a field added in the middle cannot
/// silently renumber the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditField {
    Title,
    Username,
    Url,
    Notes,
    Password,
}

impl EditField {
    /// The tab order, top to bottom.
    const ORDER: [Self; 5] = [
        Self::Title,
        Self::Username,
        Self::Url,
        Self::Notes,
        Self::Password,
    ];

    #[must_use]
    pub fn next(self) -> Self {
        crate::cycle::step(&Self::ORDER, self, true)
    }

    #[must_use]
    pub fn prev(self) -> Self {
        crate::cycle::step(&Self::ORDER, self, false)
    }
}

/// Whether the form is taking edits or asking whether to discard them.
///
/// A mode instead of a bool named `confirming`: `Esc` on a dirty form must not
/// throw the draft away silently, and the renderer and the key handler have to
/// agree on which question is on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormMode {
    Editing,
    ConfirmSave,
}
