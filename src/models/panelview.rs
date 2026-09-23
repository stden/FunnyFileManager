//! Purpose: How a panel shows its listing — the view mode, what kind of panel
//! it is, which panels are on screen, and the sort order.
//!
//! Split out of `models.rs`, which sits a few lines under the 1000-line cap
//! `tests/file_line_limit_test.rs` enforces. The boundary is not arbitrary:
//! everything here answers "how does this panel look", and `models.rs` keeps
//! what the program *is*.

use super::{FolderTree, PanelSide, QuickView, RemotePanel, SessionsPanel};

/// What a panel is, beyond a list of files.
///
/// Far can turn either panel into a folder tree or a preview of what the other
/// panel's cursor is on. The alternatives carry their own state so that
/// switching back to the file list drops it — a tree that lived in a separate
/// field would survive, stale, behind a file panel that no longer shows it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum PanelKind {
    /// The directory listing.
    #[default]
    Files,
    /// `Ctrl+T`: the folder tree, rooted at the filesystem root.
    Tree(Box<FolderTree>),
    /// `Ctrl+Q`: the file under the other panel's cursor.
    QuickView(Box<QuickView>),
    /// Alt+F1 → `NetBox`: the stored sessions.
    Sessions(Box<SessionsPanel>),
    /// Enter on a session: a directory on the remote host.
    Remote(Box<RemotePanel>),
}

impl PanelKind {
    #[must_use]
    pub const fn is_tree(&self) -> bool {
        matches!(self, Self::Tree(_))
    }

    #[must_use]
    pub const fn is_quick_view(&self) -> bool {
        matches!(self, Self::QuickView(_))
    }

    /// Whether the panel shows a `NetBox` surface — the sessions or a host —
    /// rather than local files.
    #[must_use]
    pub const fn is_netbox(&self) -> bool {
        matches!(self, Self::Sessions(_) | Self::Remote(_))
    }

    /// A click on the `row`th row a `NetBox` panel draws: its cursor moves
    /// there. The row's index, or `None` below the last row and for every
    /// other kind of panel.
    pub fn select_visible_row(&mut self, row: usize) -> Option<usize> {
        match self {
            Self::Sessions(sessions) => sessions.select_visible(row),
            Self::Remote(remote) => remote.select_visible(row),
            Self::Files | Self::Tree(_) | Self::QuickView(_) => None,
        }
    }
}

/// Which panels are on screen — Far's `Ctrl+F1` and `Ctrl+F2`.
///
/// Three states rather than two `bool`s: with two flags "neither shown" is a
/// representable state, and it is one this program refuses — the command
/// line with nothing above it is `Ctrl+O`'s job, not a panel setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanelsShown {
    #[default]
    Both,
    LeftOnly,
    RightOnly,
}

impl PanelsShown {
    #[must_use]
    pub const fn shows(self, side: PanelSide) -> bool {
        matches!(
            (self, side),
            (Self::Both, _)
                | (Self::LeftOnly, PanelSide::Left)
                | (Self::RightOnly, PanelSide::Right)
        )
    }

    /// The panels actually on screen when `active` is in a view mode that
    /// is `fullscreen`: that panel takes the whole width, as if the other
    /// were hidden. The renderer and the mouse both read the answer, so a
    /// click can never land on a panel that was not drawn.
    #[must_use]
    pub const fn with_fullscreen(self, active: PanelSide, fullscreen: bool) -> Self {
        match (fullscreen, active) {
            (false, _) => self,
            (true, PanelSide::Left) => Self::LeftOnly,
            (true, PanelSide::Right) => Self::RightOnly,
        }
    }

    /// The state after `Ctrl+F1` / `Ctrl+F2` on `side`.
    ///
    /// Hiding the only panel left is refused: the state comes back unchanged
    /// and `ops::panels::toggle` tells the user so.
    #[must_use]
    pub const fn toggled(self, side: PanelSide) -> Self {
        match (self, side) {
            (Self::Both, PanelSide::Left) => Self::RightOnly,
            (Self::Both, PanelSide::Right) => Self::LeftOnly,
            (Self::LeftOnly, PanelSide::Right) | (Self::RightOnly, PanelSide::Left) => Self::Both,
            (only, _) => only,
        }
    }
}

/// How the panel orders its rows — Far's `Ctrl+F3` … `Ctrl+F11`.
///
/// `Unsorted` is the directory's own order, which on most filesystems means
/// creation order and on none of them means anything the user chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum SortMode {
    #[default]
    Name,
    Extension,
    Size,
    ModificationTime,
    Unsorted,
    CreationTime,
    AccessTime,
    Descriptions,
    Owner,
}

impl SortMode {
    /// In the order of Far's Sort modes menu.
    pub const ALL: [Self; 9] = [
        Self::Name,
        Self::Extension,
        Self::ModificationTime,
        Self::Size,
        Self::Unsorted,
        Self::CreationTime,
        Self::AccessTime,
        Self::Descriptions,
        Self::Owner,
    ];

    /// The legacy three-stop cycle (name, extension, size) kept for its
    /// callers; the Far-only modes walk the rest of the menu back to `Name`.
    #[must_use]
    pub const fn next(self) -> Self {
        match self {
            Self::Name => Self::Extension,
            Self::Extension => Self::Size,
            Self::Size | Self::Owner => Self::Name,
            Self::ModificationTime => Self::Unsorted,
            Self::Unsorted => Self::CreationTime,
            Self::CreationTime => Self::AccessTime,
            Self::AccessTime => Self::Descriptions,
            Self::Descriptions => Self::Owner,
        }
    }

    /// The name the status line and the sort menu show.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Name => "Name",
            Self::Extension => "Ext",
            Self::Size => "Size",
            Self::ModificationTime => "Modification time",
            Self::Unsorted => "Unsorted",
            Self::CreationTime => "Creation time",
            Self::AccessTime => "Access time",
            Self::Descriptions => "Descriptions",
            Self::Owner => "Owner",
        }
    }

    /// The letter Far puts in the panel header's corner — the only place the
    /// panel says how it is ordered.
    #[must_use]
    pub const fn letter(self) -> char {
        match self {
            Self::Name => 'n',
            Self::Extension => 'e',
            Self::Size => 's',
            Self::ModificationTime => 'm',
            Self::Unsorted => 'u',
            Self::CreationTime => 'c',
            Self::AccessTime => 'a',
            Self::Descriptions => 'd',
            Self::Owner => 'o',
        }
    }

    /// The `:sort` argument for the mode, and its accepted spellings.
    const WORDS: [(Self, &'static [&'static str]); 9] = [
        (Self::Name, &["name"]),
        (Self::Extension, &["ext", "extension"]),
        (Self::Size, &["size"]),
        (Self::ModificationTime, &["mtime", "modified"]),
        (Self::Unsorted, &["unsorted"]),
        (Self::CreationTime, &["ctime", "created"]),
        (Self::AccessTime, &["atime", "accessed"]),
        (Self::Descriptions, &["desc", "descriptions"]),
        (Self::Owner, &["owner"]),
    ];

    /// The mode a `:sort` word names, or `None` for a word that is not one.
    #[must_use]
    pub fn parse(word: &str) -> Option<Self> {
        Self::WORDS
            .iter()
            .find(|(_, words)| words.contains(&word))
            .map(|(mode, _)| *mode)
    }

    /// Every spelling `:sort` accepts, for the usage line.
    pub const USAGE: &'static str =
        "sort expects name|ext|size|mtime|unsorted|ctime|atime|desc|owner";
}

#[cfg(test)]
mod tests;
