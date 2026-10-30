//! Purpose: What a click on the task list (`Ctrl+W`) means: a row takes
//! the cursor, a column title sorts by it, a key-bar cell presses its key, a
//! menu line is chosen, and a dialog's button presses the key that button
//! stands for.

use std::io::Write;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{backend::Backend, layout::Rect, Terminal};

use super::super::dispatch_key;
use crate::models::tasks::TaskTab;
use crate::models::{App, ConfirmButton, FormFocus, Modal, Overlay, ReniceButton, TaskList};
use crate::ui::{task_click_at, TaskClick};

/// A click on the task list's table, title row, key bar or open menu.
pub(super) fn click_task_list<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    column: u16,
    row: u16,
) -> bool {
    let Overlay::Tasks(list) = &app.overlay else {
        return false;
    };
    let Ok(size) = terminal.size() else {
        return false;
    };
    let screen = Rect::new(0, 0, size.width, size.height);
    let Some(click) = task_click_at(screen, list, column, row) else {
        return false;
    };
    crate::diagnostics::log_action(&format!("Task list click: {click:?}"));
    apply(terminal, app, click);
    true
}

/// Does what `click` names: the key it presses, or the move it makes.
fn apply<B: Backend + Write>(terminal: &mut Terminal<B>, app: &mut App, click: TaskClick) {
    let viewport = app.viewport;
    match click {
        TaskClick::Key(number) => dispatch_key(terminal, app, KeyCode::F(number).into()),
        TaskClick::MenuLine(index) => {
            if let Some(Modal::Menu(menu)) = app.task_list_mut().map(TaskList::modal_mut) {
                menu.select(index);
            }
            dispatch_key(terminal, app, KeyCode::Enter.into());
        }
        TaskClick::Row(index) => {
            if let Some(list) = app.task_list_mut() {
                let page = list.page_for(viewport);
                if list.tab() == TaskTab::Network {
                    list.select_net_index(index, page);
                } else {
                    list.select_index(index, page);
                }
            }
        }
        TaskClick::Title(key) => {
            if let Some(list) = app.task_list_mut() {
                list.choose_sort(key);
            }
        }
    }
}

/// The key the `index`-th button of the dialog open on `list` stands for.
pub(super) fn tasks_button_key(list: &mut TaskList, index: usize) -> Option<KeyEvent> {
    let code = match list.modal_mut() {
        Modal::Confirm(confirm) => confirm_key(*confirm.buttons().get(index)?),
        Modal::Priority(form) => {
            form.focus_on(button_at(index, [FormFocus::Set, FormFocus::Cancel])?);
            KeyCode::Enter
        }
        Modal::Renice(confirm) => {
            confirm.focus_on(button_at(
                index,
                [ReniceButton::Renice, ReniceButton::Cancel],
            )?);
            KeyCode::Enter
        }
        Modal::Mask(prompt) => {
            prompt.focus_cancel(index != 0);
            KeyCode::Enter
        }
        Modal::Info(_) | Modal::NetInfo | Modal::Error { .. } => KeyCode::Esc,
        Modal::Browse
        | Modal::Help(_)
        | Modal::Filter(_)
        | Modal::Search(_)
        | Modal::PidJump(_)
        | Modal::Menu(_) => return None,
    };
    Some(code.into())
}

const fn confirm_key(button: ConfirmButton) -> KeyCode {
    match button {
        ConfirmButton::Terminate => KeyCode::Char('t'),
        ConfirmButton::Kill => KeyCode::Char('k'),
        ConfirmButton::Send => KeyCode::Char('s'),
        ConfirmButton::Cancel => KeyCode::Esc,
    }
}

fn button_at<T: Copy>(index: usize, buttons: [T; 2]) -> Option<T> {
    buttons.get(index).copied()
}
