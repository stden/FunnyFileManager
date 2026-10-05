//! Purpose: Keys for a panel that has become `NetBox`'s stored-sessions list.
//!
//! The dispatcher asks here first while the active panel is the list; `true`
//! means the key was the list's. The list claims what moves through its rows,
//! Enter, and the keys Far's `NetBox` gives a session — `F3` to see it, `F4`
//! to change it, `Shift+F4` for a new one, `F5` to copy it, `F8` to delete it.
//! Any other key that would act on the hidden file list is refused with a
//! word through [`super::netbox`]; the rest falls through, so `Tab` still
//! switches panels and `Alt+F1` still opens the drive menu.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::netbox::{is_bare, refuse_entry_key, row_motion};
use crate::models::{App, InputPrompt, InputPurpose, Overlay, PanelKind, Question};
use crate::models::{QuestionPurpose, SessionName};
use crate::ops;

/// What the list says to a key that would act on the hidden files.
const NOT_FILES: &str = "NetBox: this key works on files; press Enter on `..` to return to them";

/// A key the list gives the row under the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowKey {
    /// Enter: connect, or leave on `..`.
    Open,
    /// F3
    View,
    /// F4
    Edit,
    /// F5
    Copy,
    /// F8
    Delete,
}

const ROW_KEYS: [(KeyCode, RowKey); 5] = [
    (KeyCode::Enter, RowKey::Open),
    (KeyCode::F(3), RowKey::View),
    (KeyCode::F(4), RowKey::Edit),
    (KeyCode::F(5), RowKey::Copy),
    (KeyCode::F(8), RowKey::Delete),
];

/// Whether the sessions panel consumed `key`.
pub fn process(app: &mut App, key: KeyEvent) -> bool {
    let page = crate::ui::panel_page_size(app.viewport.rows());
    let PanelKind::Sessions(panel) = &mut app.get_active_panel().kind else {
        return false;
    };
    if let Some(motion) = row_motion(key, page) {
        panel.move_cursor(motion);
        return true;
    }
    // Cloning the name ends the borrow of the panel, so the key's operation
    // can take the whole `App`.
    let selected = panel.selected_name().cloned();
    if key.code == KeyCode::F(4) && key.modifiers == KeyModifiers::SHIFT {
        ops::sessions::create(app);
        return true;
    }
    let Some(row_key) = ROW_KEYS
        .into_iter()
        .find_map(|(code, row_key)| is_bare(key, code).then_some(row_key))
    else {
        return refuse_entry_key(app, key, NOT_FILES);
    };
    match (row_key, selected) {
        (row_key, Some(name)) => act_on_session(app, row_key, name),
        // `..` leaves; the other keys have no session to act on there.
        (RowKey::Open, None) => ops::sessions::back_to_files(app),
        (RowKey::View | RowKey::Edit | RowKey::Copy | RowKey::Delete, None) => {}
    }
    true
}

/// A row key on the session `name`.
fn act_on_session(app: &mut App, row_key: RowKey, name: SessionName) {
    match row_key {
        RowKey::Open => ops::remote::connect(app, &name),
        RowKey::View => {
            let outcome = ops::sessions::view(app, &name);
            ops::sessions::report(app, outcome);
        }
        RowKey::Edit => {
            let outcome = ops::sessions::edit(app, &name);
            ops::sessions::report(app, outcome);
        }
        // The copy's name is offered as the original's: a copy is usually a
        // small change to it.
        RowKey::Copy => {
            let suggested = name.to_string();
            let prompt = InputPrompt::new(InputPurpose::DuplicateSession(name), &suggested);
            app.set_overlay(Overlay::InputPrompt(Box::new(prompt)));
        }
        // Asked, as F8 on a file is.
        RowKey::Delete => {
            let question = Question::new(QuestionPurpose::DeleteSession(name));
            app.set_overlay(Overlay::Question(question));
        }
    }
}

#[cfg(test)]
mod tests;
