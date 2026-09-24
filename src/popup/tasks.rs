//! Purpose: Keys for the task list (`Ctrl+W`), routed by what is open on it:
//! the table, a bar, a menu or a dialog.
//!
//! Every key is consumed, so none falls through to the panels behind the
//! list. A key clears the notice shown in place of the info row, and a key
//! that changes what is open re-centres the next dialog.

use std::mem::discriminant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::models::{App, Modal};

mod bars;
mod browse;
mod dialogs;
mod menus;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Some(list) = app.task_list_mut() else {
        return false;
    };
    list.clear_notice();
    let before = discriminant(list.modal());
    route(app, KeyEvent::new(code, modifiers));
    let after = app.task_list_mut().map(|list| discriminant(list.modal()));
    if after != Some(before) {
        app.dialog_mouse.reset();
    }
    true
}

/// Hands `key` to whatever is open on the list.
fn route(app: &mut App, key: KeyEvent) {
    let Some(list) = app.task_list_mut() else {
        return;
    };
    match list.modal() {
        Modal::Browse => browse::process(app, key),
        Modal::Filter(_) => bars::filter(app, key),
        Modal::Search(_) => bars::search(app, key),
        Modal::PidJump(_) => bars::pid_jump(app, key),
        Modal::Menu(_) => menus::process(app, key),
        Modal::Help(_) => dialogs::help(app, key),
        Modal::Info(_) | Modal::Error(_) => dialogs::dismiss(app, key),
        Modal::Mask(_) => dialogs::mask(app, key),
        Modal::Confirm(_) => dialogs::confirm(app, key),
        Modal::Priority(_) => dialogs::priority(app, key),
        Modal::Renice(_) => dialogs::renice(app, key),
    }
}

/// A terminal paste into the open bar, if one is.
pub(super) fn paste(app: &mut App, text: &str) {
    bars::paste(app, text);
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_bars;
#[cfg(test)]
mod tests_dialogs;
#[cfg(test)]
mod tests_escape;
#[cfg(test)]
mod tests_menus;
#[cfg(all(test, unix))]
mod tests_signal_menu;
