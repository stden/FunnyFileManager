//! Purpose: The enums a name dialog is made of — what it asks for, the link kinds it offers, and which control has the cursor.

#[cfg(doc)]
use super::NamePrompt;

/// What a [`NamePrompt`] is asking for, which is the only thing that differs
/// between the two dialogs — the typing, the editing and the cancelling are
/// the same, so they are written once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamePurpose {
    /// Shift+F6: the entry keeps its place and changes its name.
    Rename,
    /// F7: a new directory in the active panel.
    MakeFolder,
    /// Alt+F6: a link to the entry under the cursor.
    Link,
}

impl NamePurpose {
    /// The dialog's title.
    #[must_use]
    pub const fn title(self) -> &'static str {
        match self {
            Self::Rename => "Rename",
            Self::MakeFolder => "Make folder",
            Self::Link => "Link",
        }
    }

    /// Whether the dialog asks what kind of link to make. Rename does not:
    /// it is Far's plain box, one field and nothing else.
    #[must_use]
    pub const fn offers_links(self) -> bool {
        matches!(self, Self::MakeFolder | Self::Link)
    }

    /// The link kinds this dialog's drop-down steps through.
    ///
    /// Two lists rather than one: no kernel this runs on will hard-link a
    /// directory, so F7 must not offer it, while Alt+F6 on a file has no use
    /// for "none" — a link that is not a link is nothing at all.
    #[must_use]
    pub const fn link_choices(self) -> &'static [LinkType] {
        match self {
            Self::Link => &LinkType::LINK_CHOICES,
            _ => &LinkType::FOLDER_CHOICES,
        }
    }
}

/// What F7 makes — a directory, or a link that stands for one — and what
/// Alt+F6 makes: a link to the entry under the cursor.
///
/// Far's own list also offers junctions. A junction is a Windows idea, so it
/// would be a control that always fails, which is worse than not offering it
/// at all. Which of these a dialog offers is [`NamePurpose::link_choices`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LinkType {
    /// A plain directory.
    #[default]
    None,
    Symbolic,
    /// A second name for the same file. Files only: see
    /// [`NamePurpose::link_choices`].
    Hard,
}

impl LinkType {
    /// What F7 steps through, in order.
    pub const FOLDER_CHOICES: [Self; 2] = [Self::None, Self::Symbolic];
    /// What Alt+F6 steps through, in order.
    pub const LINK_CHOICES: [Self; 2] = [Self::Symbolic, Self::Hard];

    /// What the closed drop-down shows.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Symbolic => "symbolic link",
            Self::Hard => "hard link",
        }
    }

    /// Whether the Target field applies. A plain directory has no target, so
    /// the field is greyed rather than taking text that would be ignored.
    #[must_use]
    pub const fn needs_target(self) -> bool {
        matches!(self, Self::Symbolic | Self::Hard)
    }
}

/// Which control in the name dialog has the cursor.
///
/// An enum rather than an index, for the reason [`crate::copy::CopyField`] is
/// one: the tab order is stated once, and a control added in the middle cannot
/// silently renumber the rest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum NameField {
    #[default]
    Name,
    Link,
    Target,
    Multiple,
    Ok,
    Cancel,
}

impl NameField {
    /// The tab order, top to bottom and then along the buttons.
    pub(crate) const ORDER: [Self; 6] = [
        Self::Name,
        Self::Link,
        Self::Target,
        Self::Multiple,
        Self::Ok,
        Self::Cancel,
    ];

    #[must_use]
    pub fn next(self) -> Self {
        crate::cycle::step(&Self::ORDER, self, true)
    }

    #[must_use]
    pub fn previous(self) -> Self {
        crate::cycle::step(&Self::ORDER, self, false)
    }

    /// Whether typing lands here — which is also what decides whether the
    /// space bar is a space or a toggle.
    #[must_use]
    pub const fn is_text(self) -> bool {
        matches!(self, Self::Name | Self::Target)
    }
}
