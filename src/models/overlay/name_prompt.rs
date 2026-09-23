//! Purpose: The name dialog's state — Shift+F6 rename, F7 make folder, Alt+F6 link — and the names it hands back.

use super::{LinkType, NameField, NamePurpose};
use crate::models::FileName;

/// A dialog asking for a name, and the name being typed into it.
///
/// Shift+F6 uses one field of it and F7 uses all of them: Far's Make folder
/// dialog also asks what kind of link to make, what it should point at, and
/// whether the name field is a list. Both are the same dialog because the
/// typing, the editing and the cancelling are the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamePrompt {
    purpose: NamePurpose,
    /// What the entry is called now, for a rename. Empty for a new folder,
    /// which is not renaming anything.
    original: FileName,
    name: String,
    link: LinkType,
    /// What a link points at. Free text, and a path rather than a name: unlike
    /// the name field, separators are exactly what belongs here.
    link_target: String,
    /// Far's "Process multiple names": the name field is then a `;`-separated
    /// list, and every part of it is created.
    multiple: bool,
    field: NameField,
}

impl NamePrompt {
    /// A rename, opening on the current name: a rename is usually a small
    /// change to a name rather than a fresh one.
    #[must_use]
    pub fn rename(original: FileName) -> Self {
        Self {
            purpose: NamePurpose::Rename,
            name: original.as_str().to_owned(),
            original,
            link: LinkType::None,
            link_target: String::new(),
            multiple: false,
            field: NameField::Name,
        }
    }

    /// A new folder, opening empty.
    #[must_use]
    pub fn make_folder() -> Self {
        Self {
            purpose: NamePurpose::MakeFolder,
            original: FileName::new(String::new()),
            name: String::new(),
            link: LinkType::None,
            link_target: String::new(),
            multiple: false,
            field: NameField::Name,
        }
    }

    /// A link to `target`, opening on `name` and on a symbolic link, which is
    /// the kind that works for everything.
    ///
    /// Alt+F6 offers the other panel's directory joined with the target's
    /// own name: Far puts a link there unless told otherwise, and showing the
    /// whole path is what lets the user tell it otherwise.
    #[must_use]
    pub fn link_to(target: &std::path::Path, name: String) -> Self {
        Self {
            purpose: NamePurpose::Link,
            original: FileName::new(String::new()),
            name,
            link: LinkType::Symbolic,
            link_target: target.display().to_string(),
            multiple: false,
            field: NameField::Name,
        }
    }

    #[must_use]
    pub const fn purpose(&self) -> NamePurpose {
        self.purpose
    }

    #[must_use]
    pub fn original(&self) -> &str {
        self.original.as_str()
    }

    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    #[must_use]
    pub const fn link(&self) -> LinkType {
        self.link
    }

    #[must_use]
    pub fn link_target(&self) -> &str {
        self.link_target.trim()
    }

    #[must_use]
    pub const fn multiple(&self) -> bool {
        self.multiple
    }

    #[must_use]
    pub const fn field(&self) -> NameField {
        self.field
    }

    /// Whether this dialog offers that control at all.
    ///
    /// Shift+F6 is Far's plain Rename box: it asks for a name and nothing
    /// else. The link controls belong to F7, and the Target field only once
    /// there is a link for it to describe — greyed otherwise, so the dialog
    /// keeps its shape as the drop-down changes.
    #[must_use]
    pub const fn allows(&self, field: NameField) -> bool {
        match field {
            NameField::Name | NameField::Ok | NameField::Cancel => true,
            NameField::Link => self.purpose.offers_links(),
            NameField::Multiple => matches!(self.purpose, NamePurpose::MakeFolder),
            NameField::Target => self.purpose.offers_links() && self.link.needs_target(),
        }
    }

    /// Move to the next control the dialog actually offers, skipping the
    /// greyed ones: stopping on a control that does nothing would undo the
    /// point of greying it.
    pub fn advance(&mut self, forward: bool) {
        for _ in 0..NameField::ORDER.len() {
            self.field = if forward {
                self.field.next()
            } else {
                self.field.previous()
            };
            if self.allows(self.field) {
                return;
            }
        }
    }

    /// Put the cursor on `field`, if the dialog offers it. What a click means.
    pub const fn focus(&mut self, field: NameField) {
        if self.allows(field) {
            self.field = field;
        }
    }

    /// Toggle the checkbox under the cursor. Does nothing elsewhere, so the
    /// space bar stays a space in the two text fields.
    pub const fn toggle(&mut self) {
        if matches!(self.field, NameField::Multiple) && self.allows(NameField::Multiple) {
            self.multiple = !self.multiple;
        }
    }

    /// Left/Right on the drop-down. Elsewhere the arrows are not ours.
    pub fn cycle(&mut self, forward: bool) {
        if !matches!(self.field, NameField::Link) || !self.allows(NameField::Link) {
            return;
        }
        self.link = crate::cycle::step(self.purpose.link_choices(), self.link, forward);
        // The Target field may have just been greyed out from under the
        // cursor. It cannot be, in fact — the cursor is on the drop-down — but
        // the invariant is worth holding rather than reasoning about.
        if !self.allows(self.field) {
            self.field = NameField::Name;
        }
    }

    /// Which control the dialog draws on body row `row`.
    ///
    /// The row order lives here rather than only in the renderer, because the
    /// mouse needs it too: a click is only right if it reaches the control the
    /// user is pointing at.
    #[must_use]
    pub const fn field_at_row(&self, row: usize) -> Option<NameField> {
        // 0 is the caption, 2 is the rule under the name field.
        match (self.purpose, row) {
            (_, 1) => Some(NameField::Name),
            (purpose, 3) if purpose.offers_links() => Some(NameField::Link),
            (purpose, 4) if purpose.offers_links() => Some(NameField::Target),
            (NamePurpose::MakeFolder, 5) => Some(NameField::Multiple),
            _ => None,
        }
    }

    pub fn push(&mut self, ch: char) {
        match self.field {
            NameField::Name => self.name.push(ch),
            NameField::Target => self.link_target.push(ch),
            _ => {}
        }
    }

    pub fn backspace(&mut self) {
        match self.field {
            NameField::Name => self.name.pop(),
            NameField::Target => self.link_target.pop(),
            _ => None,
        };
    }

    /// A name this dialog may act on, or `None` when there is nothing to do:
    /// blank, or — for a rename — the name the entry already has. A path
    /// separator is refused too, because that would move the entry somewhere
    /// else rather than name it, and moving is what F6 is for.
    fn usable<'a>(&self, candidate: &'a str) -> Option<&'a str> {
        let trimmed = candidate.trim();
        if trimmed.is_empty() || trimmed.contains('/') || trimmed.contains('\\') {
            return None;
        }
        if self.purpose == NamePurpose::Rename && trimmed == self.original.as_str() {
            return None;
        }
        Some(trimmed)
    }

    /// The name to act on, for the dialogs that ask for exactly one.
    #[must_use]
    pub fn target(&self) -> Option<&str> {
        self.usable(&self.name)
    }

    /// Every name to create, in the order they were typed.
    ///
    /// One, unless "Process multiple names" is ticked — Far's way of making
    /// several folders from one dialog, where the field is a `;`-separated
    /// list. Unusable parts are dropped rather than failing the whole line: a
    /// trailing `;` is how the list is normally typed.
    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        if !self.multiple {
            return self.target().into_iter().collect();
        }
        self.name
            .split(';')
            .filter_map(|part| self.usable(part))
            .collect()
    }
}
