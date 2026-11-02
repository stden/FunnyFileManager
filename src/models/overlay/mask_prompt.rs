//! Purpose: Gray + / Gray − mask dialog — which files join or leave the selection, given as a mask.

use crate::lineedit::LineEditor;

/// Which of Far's two group keys put the mask dialog up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskPurpose {
    /// Gray +
    Select,
    /// Gray −
    Unselect,
}

impl MaskPurpose {
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Select => "Select",
            Self::Unselect => "Deselect",
        }
    }

    /// Whether the matching rows join the selection or leave it.
    #[must_use]
    pub const fn selects(self) -> bool {
        matches!(self, Self::Select)
    }
}

/// Gray + / Gray −: which files, given as a mask.
///
/// The offered mask opens selected, as Far shows it: the first character
/// typed replaces it, and moving the caret keeps it as text of the user's
/// own. Appending to the offer turned `*.txt` and `semi*` into
/// `*.txtsemi*`, which matches nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaskPrompt {
    purpose: MaskPurpose,
    mask: LineEditor,
    /// The cursor is on Cancel rather than on OK.
    on_cancel: bool,
}

impl MaskPrompt {
    /// Open the dialog, offering `suggestion` as the mask.
    ///
    /// Far offers the extension of the file under the cursor, because the
    /// commonest thing anyone does with this key is "all the other ones like
    /// this". A field that opens on `*` makes them type it every time.
    #[must_use]
    pub fn new(purpose: MaskPurpose, suggestion: &str) -> Self {
        let mut mask = LineEditor::new(suggestion);
        mask.select_all();
        Self {
            purpose,
            mask,
            on_cancel: false,
        }
    }

    /// The mask Far would offer for `name`: its extension, or everything.
    #[must_use]
    pub fn suggest(name: Option<&str>) -> String {
        name.and_then(|name| name.rsplit_once('.'))
            // A leading dot starts a name, not an extension: `.gitignore` has
            // no extension, and offering `*.gitignore` would be nonsense.
            .filter(|(stem, extension)| !stem.is_empty() && !extension.is_empty())
            .map_or_else(|| "*".to_owned(), |(_, extension)| format!("*.{extension}"))
    }

    #[must_use]
    pub const fn purpose(&self) -> MaskPurpose {
        self.purpose
    }

    #[must_use]
    pub fn mask(&self) -> &str {
        self.mask.text()
    }

    /// The field, to draw.
    #[must_use]
    pub const fn line(&self) -> &LineEditor {
        &self.mask
    }

    /// The field, to edit.
    pub const fn line_mut(&mut self) -> &mut LineEditor {
        &mut self.mask
    }

    /// Whether the whole mask is still selected, as the offer opens — drawn
    /// selected, and replaced by the first character typed.
    #[must_use]
    pub fn is_offered(&self) -> bool {
        self.mask.selection().is_some_and(|range| {
            range.start.byte() == 0 && range.end.byte() == self.mask.text().len()
        })
    }

    #[must_use]
    pub const fn on_cancel(&self) -> bool {
        self.on_cancel
    }

    pub const fn focus_cancel(&mut self, cancel: bool) {
        self.on_cancel = cancel;
    }

    pub const fn toggle_button(&mut self) {
        self.on_cancel = !self.on_cancel;
    }

    pub fn push(&mut self, ch: char) {
        self.mask.insert_char(ch);
    }

    /// What stands to the right of the caret — the width `ui` backs the
    /// terminal cursor up by from the end of the field.
    #[must_use]
    pub fn after_caret(&self) -> &str {
        self.mask.after_cursor()
    }
}
