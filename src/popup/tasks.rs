//! Purpose: Keys for the task list (`Ctrl+W`), routed by what is open on it:
//! the table, a bar, a menu or a dialog.
//!
//! Every key is consumed, so none falls through to the panels behind the
//! list. A key clears the notice shown in place of the info row, and a key
//! that changes what is open re-centres the next dialog.

use std::mem::discriminant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::text_field::with_clipboard;
use crate::action::tasks::TaskCommand;
use crate::models::{App, ListMotion, Modal};
use crate::mouse::Wheel;

mod bars;
mod browse;
mod dialogs;
mod menus;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    transition(app, |app| route(app, KeyEvent::new(code, modifiers)))
}

/// The same browse command a key runs, without depending on its physical chord.
pub fn invoke(app: &mut App, command: TaskCommand) -> bool {
    if !app
        .task_list_mut()
        .is_some_and(|list| matches!(list.modal(), Modal::Browse))
    {
        return false;
    }
    transition(app, |app| browse::execute(app, command))
}

fn transition(app: &mut App, apply: impl FnOnce(&mut App)) -> bool {
    let Some(list) = app.task_list_mut() else {
        return false;
    };
    list.clear_notice();
    let before = discriminant(list.modal());
    apply(app);
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
        Modal::Filter(_) => with_clipboard(app, |app, clipboard| bars::filter(app, key, clipboard)),
        Modal::Search(_) => with_clipboard(app, |app, clipboard| bars::search(app, key, clipboard)),
        Modal::PidJump(_) => bars::pid_jump(app, key),
        Modal::Menu(_) => menus::process(app, key),
        Modal::Help(_) => dialogs::help(app, key),
        Modal::Info(_) | Modal::NetInfo | Modal::Error { .. } => dialogs::dismiss(app, key),
        Modal::Mask(_) => with_clipboard(app, |app, clipboard| dialogs::mask(app, key, clipboard)),
        Modal::Confirm(_) => dialogs::confirm(app, key),
        Modal::Priority(_) => {
            with_clipboard(app, |app, clipboard| dialogs::priority(app, key, clipboard));
        }
        Modal::Renice(_) => dialogs::renice(app, key),
    }
}

/// The wheel over the table: a cursor move, as Up and Down are. `false`
/// while a bar, a menu or a dialog is open on the list — it reads the key
/// itself.
pub(super) fn wheel(app: &mut App, wheel: Wheel) -> bool {
    let viewport = app.viewport;
    let Some(list) = app.task_list_mut() else {
        return false;
    };
    if !matches!(list.modal(), Modal::Browse) {
        return false;
    }
    list.clear_notice();
    let page = list.page_for(viewport);
    let motion = match wheel {
        Wheel::Up => ListMotion::Up,
        Wheel::Down => ListMotion::Down,
    };
    browse::apply(list, TaskCommand::Move(motion), page);
    true
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
