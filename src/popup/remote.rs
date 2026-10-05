//! Purpose: Keys for a panel that browses a remote host through a session.
//!
//! The dispatcher asks here first while the active panel is remote; `true`
//! means the key was the panel's. The panel claims what moves through its
//! rows and Enter; any other key that would act on the file list hidden
//! beneath it is refused with a word through [`super::netbox`], and the rest
//! falls through.

use crossterm::event::{KeyCode, KeyEvent};

use super::netbox::{is_bare, refuse_entry_key, row_motion};
use crate::models::{App, PanelKind};
use crate::ops;

/// What the panel says to a key that would act on the hidden files.
const NOT_FILES: &str =
    "NetBox: this key works on local files; Enter on `..` at the host's root leaves it";

/// Whether the remote panel consumed `key`.
pub fn process(app: &mut App, key: KeyEvent) -> bool {
    let page = crate::ui::panel_page_size(app.viewport.rows());
    let PanelKind::Remote(panel) = &mut app.get_active_panel().kind else {
        return false;
    };
    if let Some(motion) = row_motion(key, page) {
        panel.move_cursor(motion);
        return true;
    }
    if is_bare(key, KeyCode::Enter) {
        ops::remote::enter(app);
        return true;
    }
    refuse_entry_key(app, key, NOT_FILES)
}

#[cfg(test)]
mod tests;
