//! Purpose: The task list's three menus — Sort by, Send signal and Show
//! processes of — moved through with the shared list keys, chosen with
//! Enter, and left with Esc or F10.
//!
//! Choosing a signal only opens its confirmation, for the targets the menu
//! froze when it opened.

use crossterm::event::{KeyCode, KeyEvent};

use crate::action::tasks::normalize;
use crate::models::{
    App, KillConfirm, KillOffer, Modal, Signal, SortKey, TaskList, TaskMenu, UserScope,
};
use crate::popup::menu_dialog::navigate;

/// F6, Ctrl+F12, `<` and `>`: Sort by, on the key in force.
pub(super) fn open_sort(list: &mut TaskList) {
    let menu = TaskMenu::sort(list.options().sort);
    list.set_modal(Modal::Menu(menu));
}

/// `u`: Show processes of, over everyone who owns a process now.
pub(super) fn open_user(list: &mut TaskList) {
    let menu = TaskMenu::user(list.owners(), &list.options().scope);
    list.set_modal(Modal::Menu(menu));
}

pub(super) fn process(app: &mut App, key: KeyEvent) {
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let Modal::Menu(menu) = list.modal_mut() else {
        return;
    };
    match normalize(key).code {
        KeyCode::Esc | KeyCode::F(10) => list.close_modal(),
        KeyCode::Enter => {
            let chosen = menu.clone();
            list.close_modal();
            choose(list, chosen);
        }
        KeyCode::Char('I') if matches!(menu, TaskMenu::Sort(_)) => {
            list.close_modal();
            list.invert_order();
        }
        code => {
            let mut row = menu.cursor().index();
            navigate(&mut row, menu.row_count(), code);
            menu.select(row);
        }
    }
}

/// Acts on the row under `menu`'s cursor; the menu is already closed.
fn choose(list: &mut TaskList, menu: TaskMenu) {
    let row = menu.cursor().index();
    match menu {
        TaskMenu::Sort(_) => match SortKey::ALL.get(row) {
            Some(&key) => list.choose_sort(key),
            None => list.invert_order(),
        },
        TaskMenu::Signal { targets, .. } => {
            if let Some(&signal) = Signal::OFFERED.get(row) {
                let confirm = KillConfirm::new(targets, KillOffer::Send(signal));
                list.set_modal(Modal::Confirm(confirm));
            }
        }
        TaskMenu::User { owners, .. } => {
            let owner = row.checked_sub(1).and_then(|at| owners.get(at));
            let scope = owner.cloned().map_or(UserScope::All, UserScope::Only);
            list.set_scope(scope);
        }
    }
}
