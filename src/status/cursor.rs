//! Purpose: where a status message was drawn, and how long the info row keeps it — until the cursor moves on.

use std::path::PathBuf;

use crate::models::App;

/// Where the cursor stood when the message now on screen was drawn.
///
/// Far's info row carries a message only until the user moves on: the next
/// Up or Down puts the entry's name back. So a message has to remember which
/// cursor it belongs to — panel, directory and row, because any of the three
/// changing means the user is looking at something else.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatusCursor {
    pub(super) side: crate::models::PanelSide,
    pub(super) directory: PathBuf,
    pub(super) row: Option<usize>,
}

/// The info row's lifetime for one outcome, independent of its message text.
#[derive(Debug, Clone, Default)]
pub enum StatusDisplay {
    #[default]
    Pending,
    Drawn(StatusCursor),
    Expired,
}

impl StatusCursor {
    /// Is this still where the cursor is? Compared in place rather than
    /// rebuilt, so an unchanged frame costs no allocation.
    pub(super) fn is_current(&self, app: &App) -> bool {
        let panel = app.get_active_panel_ref();
        self.side == app.active_panel
            && self.directory == panel.path
            && self.row == panel.selected()
    }
}
