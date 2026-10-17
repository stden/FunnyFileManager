//! Purpose: Keys for a panel that browses a remote host through a session.
//!
//! The dispatcher asks here first while the active panel is remote; `true`
//! means the key was the panel's. Of the keys the keymap's `netbox_panel`
//! holds, the panel claims what moves through its rows and Open; any other
//! key that would act on the file list hidden beneath it is refused with a
//! word through [`super::netbox`], and the rest falls through. This file
//! applies the command.

use crossterm::event::KeyEvent;

use super::netbox::{refuse_entry, refuse_entry_key, HIDDEN_ENTRIES};
use crate::action::netbox::{resolve, NetboxCommand, RowCommand};
use crate::action::Action;
use crate::models::{App, PanelKind};
use crate::ops;

/// What the panel says to a key that would act on the hidden files.
const NOT_FILES: &str =
    "NetBox: this key works on local files; Enter on `..` at the host's root leaves it";

/// Whether the remote panel consumed `key`.
pub fn process(app: &mut App, key: KeyEvent) -> bool {
    if !matches!(app.get_active_panel_ref().kind, PanelKind::Remote(_)) {
        return false;
    }
    resolve(&app.keymap.netbox_panel, key).is_some_and(|command| apply(app, command))
        || refuse_entry_key(app, key, NOT_FILES)
}

/// [`process`] for an action that arrived without a key — a menu row, a
/// mouse gesture: the panel's own reading of it, or the refusal its key
/// would meet.
pub fn answer(app: &mut App, action: Action) -> bool {
    NetboxCommand::from_action(action).is_some_and(|command| apply(app, command))
        || refuse_entry(app, action, HIDDEN_ENTRIES)
}

/// Whether the panel took `command`: the moves and Enter are its. The
/// session commands are the sessions list's; on a host they would reach for
/// the files hidden beneath the panel, so they are handed back to be refused.
pub fn apply(app: &mut App, command: NetboxCommand) -> bool {
    let page = crate::ui::panel_page_size(app.viewport.rows());
    let PanelKind::Remote(panel) = &mut app.get_active_panel().kind else {
        return false;
    };
    match command {
        NetboxCommand::Move(motion) => panel.move_cursor(motion.with_page(page)),
        NetboxCommand::Row(RowCommand::Open) => ops::remote::enter(app),
        NetboxCommand::NewSession
        | NetboxCommand::Row(
            RowCommand::ViewSession
            | RowCommand::EditSession
            | RowCommand::CopySession
            | RowCommand::DeleteSession,
        ) => return false,
    }
    true
}

#[cfg(test)]
mod tests;
