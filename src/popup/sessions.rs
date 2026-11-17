//! Purpose: Keys for a panel that has become `NetBox`'s stored-sessions list.
//!
//! The dispatcher asks here first while the active panel is the list; `true`
//! means the key was the list's. The list claims the keys the keymap's
//! `netbox_panel` holds: by default what moves through its rows, Enter, and
//! the keys Far's `NetBox` gives a session — `F3` to see it, `F4` to change
//! it, `Shift+F4` for a new one, `F5` to copy it, `F8` to delete it. Any
//! other key that would act on the hidden file list is refused with a word
//! through [`super::netbox`]; the rest falls through, so `Tab` still switches
//! panels and `Alt+F1` still opens the drive menu. This file applies the
//! command.

use crossterm::event::KeyEvent;

use super::netbox::{refuse_entry, refuse_entry_key, HIDDEN_ENTRIES};
use crate::action::netbox::{resolve, NetboxCommand, RowCommand};
use crate::action::Action;
use crate::models::{App, InputPrompt, InputPurpose, Overlay, PanelKind, Question};
use crate::models::{QuestionPurpose, SessionName};
use crate::ops;

/// What the list says to a key that would act on the hidden files.
const NOT_FILES: &str = "NetBox: this key works on files; press Enter on `..` to return to them";

/// Whether the sessions panel consumed `key`.
pub fn process(app: &mut App, key: KeyEvent) -> bool {
    if !matches!(app.get_active_panel_ref().kind, PanelKind::Sessions(_)) {
        return false;
    }
    match resolve(&app.keymap.netbox_panel, key) {
        Some(command) => apply(app, command),
        None => refuse_entry_key(app, key, NOT_FILES),
    }
}

/// [`process`] for an action that arrived without a key — a menu row, a
/// mouse gesture: the list's own reading of it, or the refusal its key
/// would meet.
pub fn answer(app: &mut App, action: Action) -> bool {
    match NetboxCommand::from_action(action) {
        Some(command) => apply(app, command),
        None => refuse_entry(app, action, HIDDEN_ENTRIES),
    }
}

/// Apply `command` to the sessions list; always `true`, every command is
/// the list's.
pub fn apply(app: &mut App, command: NetboxCommand) -> bool {
    let page = crate::ui::panel_page_size(app.viewport.rows());
    let PanelKind::Sessions(panel) = &mut app.get_active_panel().kind else {
        return false;
    };
    // Cloning the name ends the borrow of the panel, so the row's operation
    // can take the whole `App`.
    let selected = panel.selected_name().cloned();
    match (command, selected) {
        (NetboxCommand::Move(motion), _) => panel.move_cursor(motion.with_page(page)),
        (NetboxCommand::NewSession, _) => ops::sessions::create(app),
        (NetboxCommand::Row(row), Some(name)) => act_on_session(app, row, name),
        // `..` leaves; the other keys have no session to act on there.
        (NetboxCommand::Row(RowCommand::Open), None) => ops::sessions::back_to_files(app),
        (
            NetboxCommand::Row(
                RowCommand::ViewSession
                | RowCommand::EditSession
                | RowCommand::CopySession
                | RowCommand::DeleteSession,
            ),
            None,
        ) => {}
    }
    true
}

/// A row command on the session `name`.
fn act_on_session(app: &mut App, row: RowCommand, name: SessionName) {
    match row {
        RowCommand::Open => ops::remote::connect(app, &name),
        RowCommand::ViewSession => {
            let outcome = ops::sessions::view(app, &name);
            ops::sessions::report(app, outcome);
        }
        RowCommand::EditSession => {
            let outcome = ops::sessions::edit(app, &name);
            ops::sessions::report(app, outcome);
        }
        // The copy's name is offered as the original's: a copy is usually a
        // small change to it.
        RowCommand::CopySession => {
            let suggested = name.to_string();
            let prompt = InputPrompt::new(InputPurpose::DuplicateSession(name), &suggested);
            app.set_overlay(Overlay::InputPrompt(Box::new(prompt)));
        }
        // Asked, as F8 on a file is.
        RowCommand::DeleteSession => {
            let question = Question::new(QuestionPurpose::DeleteSession(name));
            app.set_overlay(Overlay::Question(question));
        }
    }
}

#[cfg(test)]
mod tests;
