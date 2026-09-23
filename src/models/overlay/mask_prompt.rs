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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaskPrompt {
    purpose: MaskPurpose,
    mask: MaskField,
    /// The cursor is on Cancel rather than on OK.
    on_cancel: bool,
}

/// The mask field: the offered mask, pre-selected as Far shows it, or the
/// user's own text once any key has edited it.
///
/// Two states rather than a flag beside the text, because they answer a typed
/// character differently: it replaces an offer and joins an edit. Appending
/// to the offer turned `*.txt` and `semi*` into `*.txtsemi*`, which matches
/// nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
enum MaskField {
    Offered(LineEditor),
    Edited(LineEditor),
}

impl MaskField {
    const fn line(&self) -> &LineEditor {
        match self {
            Self::Offered(line) | Self::Edited(line) => line,
        }
    }

    /// The text to edit where it stands; an offer becomes the user's own.
    fn edit(&mut self) -> &mut LineEditor {
        if let Self::Offered(line) = self {
            *self = Self::Edited(std::mem::take(line));
        }
        match self {
            Self::Offered(line) | Self::Edited(line) => line,
        }
    }

    fn type_char(&mut self, ch: char) {
        if let Self::Offered(_) = self {
            *self = Self::Edited(LineEditor::default());
        }
        self.edit().insert_char(ch);
    }
}

impl MaskPrompt {
    /// Open the dialog, offering `suggestion` as the mask.
    ///
    /// Far offers the extension of the file under the cursor, because the
    /// commonest thing anyone does with this key is "all the other ones like
    /// this". A field that opens on `*` makes them type it every time.
    #[must_use]
    pub fn new(purpose: MaskPurpose, suggestion: &str) -> Self {
        Self {
            purpose,
            mask: MaskField::Offered(LineEditor::new(suggestion)),
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
        self.mask.line().text()
    }

    /// Whether the field still holds the offered mask untouched — drawn
    /// selected, and replaced by the first character typed.
    #[must_use]
    pub const fn is_offered(&self) -> bool {
        matches!(self.mask, MaskField::Offered(_))
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
        self.mask.type_char(ch);
    }

    pub fn backspace(&mut self) {
        self.mask.edit().backspace();
    }

    pub fn delete_forward(&mut self) {
        self.mask.edit().delete_forward();
    }

    /// The caret moves through the text, as in any Far edit control; moving
    /// it keeps the offer as text of the user's own.
    pub fn left(&mut self) {
        self.mask.edit().left();
    }

    pub fn right(&mut self) {
        self.mask.edit().right();
    }

    pub fn home(&mut self) {
        self.mask.edit().home();
    }

    pub fn end(&mut self) {
        self.mask.edit().end();
    }

    /// What stands to the right of the caret — the width `ui` backs the
    /// terminal cursor up by from the end of the field.
    #[must_use]
    pub fn after_caret(&self) -> &str {
        self.mask.line().after_cursor()
    }
}
