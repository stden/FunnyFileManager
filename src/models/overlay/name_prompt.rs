//! Purpose: The name dialog's state — Shift+F6 rename, F7 make folder, Alt+F6 link — and the names it hands back.

use super::{LinkType, NameField, NamePurpose};
use crate::models::FileName;

mod controls;

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

    /// A name this dialog may act on, or `None` when there is nothing to do:
    /// blank, or — for a rename — the name the entry already has. A path
    /// separator is refused in a rename or a link name, because that would
    /// put the entry somewhere else rather than name it, and moving is what
    /// F6 is for. F7 takes a path, as Far's does: `p1/p2/p3` makes all three.
    fn usable<'a>(&self, candidate: &'a str) -> Option<&'a str> {
        let trimmed = candidate.trim();
        let is_path = trimmed.contains('/') || trimmed.contains('\\');
        if trimmed.is_empty() || (is_path && self.purpose != NamePurpose::MakeFolder) {
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
