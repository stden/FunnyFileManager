//! Purpose: How a panel shows its listing — the view mode, what kind of panel
//! it is, which panels are on screen, and the sort order.
//!
//! Split out of `models.rs`, which sits a few lines under the 1000-line cap
//! `tests/file_line_limit_test.rs` enforces. The boundary is not arbitrary:
//! everything here answers "how does this panel look", and `models.rs` keeps
//! what the program *is*.

use super::{FolderTree, PanelSide, QuickView, RemotePanel, SessionsPanel};

mod sort_mode;

pub use sort_mode::SortMode;

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

#[cfg(test)]
mod tests;
