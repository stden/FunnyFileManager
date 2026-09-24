//! Purpose: The task list's three text bars on the bottom row — the live
//! filter (`F4`), the search (`/`, Alt+letter) and the PID jump (digits) —
//! and the terminal pastes they take.
//!
//! The editing keys are the shared one-line ones of every prompt. The
//! filter rebuilds the view on every edit; the search and the jump only move
//! the cursor.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::tasks::{resolve, TaskCommand};
use crate::lineedit::LineEditor;
use crate::models::{
    App, ListMotion, Modal, Pid, PidQuery, SearchBar, SearchFrom, TaskList, TextBar,
};
use crate::popup::menu_dialog::edit;

use super::browse;

/// The most characters one paste puts into a bar.
const PASTE_LIMIT: usize = 256;

/// `F4` and `\`: the filter bar, holding the filter in force.
pub(super) fn open_filter(list: &mut TaskList) {
    let bar = TextBar::new(list.options().filter.line());
    list.set_modal(Modal::Filter(bar));
}

/// `/` opens the search bar empty; Alt+character opens it with `typed`
/// already searched for.
pub(super) fn open_search(list: &mut TaskList, typed: Option<char>, page: usize) {
    let mut bar = SearchBar::default();
    if let Some(ch) = typed {
        bar.line_mut().insert_char(ch);
    }
    let query = bar.line().text().to_owned();
    list.set_modal(Modal::Search(bar));
    if typed.is_some() {
        find(list, &query, SearchFrom::Cursor, page);
    }
}

/// A digit: the PID jump, with the cursor on that PID if it is shown.
pub(super) fn open_pid_jump(list: &mut TaskList, digit: char, page: usize) {
    let mut query = PidQuery::default();
    query.push(digit);
    let pid = query.pid();
    list.set_modal(Modal::PidJump(query));
    jump(list, pid, page);
}

/// The filter bar: every edit filters afresh; the list keys still move the
/// cursor; Enter keeps the filter and Esc clears it, both closing the bar.
pub(super) fn filter(app: &mut App, key: KeyEvent) {
    let viewport = app.viewport;
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let page = list.page_for(viewport);
    if let Some(motion) = list_motion(key) {
        list.move_cursor(motion, page);
        return;
    }
    match key.code {
        KeyCode::Enter => list.close_modal(),
        KeyCode::Esc => {
            list.set_filter("");
            list.close_modal();
        }
        code => {
            let Modal::Filter(bar) = list.modal_mut() else {
                return;
            };
            if let Some(text) = edited(bar.line_mut(), code, key.modifiers) {
                list.set_filter(&text);
            }
        }
    }
}

/// The search bar: every edit searches from the cursor down, `F3` finds the
/// next match of what is typed, Enter and Esc leave the cursor where it is,
/// and a list key closes the bar and then moves as it would without it.
pub(super) fn search(app: &mut App, key: KeyEvent) {
    let viewport = app.viewport;
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let page = list.page_for(viewport);
    if list_motion(key).is_some() {
        list.close_modal();
        browse::process(app, key);
        return;
    }
    let Modal::Search(bar) = list.modal_mut() else {
        return;
    };
    match key.code {
        KeyCode::Enter | KeyCode::Esc => list.close_modal(),
        KeyCode::F(3) if !bar.line().text().is_empty() => {
            let query = bar.line().text().to_owned();
            find(list, &query, SearchFrom::AfterCursor, page);
        }
        code => {
            if let Some(query) = edited(bar.line_mut(), code, key.modifiers) {
                find(list, &query, SearchFrom::Cursor, page);
            }
        }
    }
}

/// The PID jump: a digit appends, Backspace removes one, Enter and Esc end
/// the jump, and any other key ends it and then does what it does.
pub(super) fn pid_jump(app: &mut App, key: KeyEvent) {
    let viewport = app.viewport;
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let page = list.page_for(viewport);
    let Modal::PidJump(query) = list.modal_mut() else {
        return;
    };
    match (key.code, resolve(key)) {
        (_, TaskCommand::PidJump(digit)) => query.push(digit),
        (KeyCode::Backspace, _) => query.pop(),
        (KeyCode::Enter | KeyCode::Esc, _) => {
            list.close_modal();
            return;
        }
        _ => {
            list.close_modal();
            browse::process(app, key);
            return;
        }
    }
    let pid = query.pid();
    jump(list, pid, page);
}

/// A terminal paste into the open bar, its control characters dropped and
/// cut at [`PASTE_LIMIT`]; with no bar open it goes nowhere.
pub(super) fn paste(app: &mut App, text: &str) {
    let viewport = app.viewport;
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let page = list.page_for(viewport);
    let clean: String = text
        .chars()
        .filter(|ch| !ch.is_control())
        .take(PASTE_LIMIT)
        .collect();
    match list.modal_mut() {
        Modal::Filter(bar) => {
            bar.line_mut().insert_str(&clean);
            let text = bar.line().text().to_owned();
            list.set_filter(&text);
        }
        Modal::Search(bar) => {
            bar.line_mut().insert_str(&clean);
            let query = bar.line().text().to_owned();
            find(list, &query, SearchFrom::Cursor, page);
        }
        Modal::PidJump(query) => {
            clean.chars().for_each(|digit| query.push(digit));
            let pid = query.pid();
            jump(list, pid, page);
        }
        _ => {}
    }
}

/// Moves the cursor to `query`'s next match and marks the search bar found
/// or not.
fn find(list: &mut TaskList, query: &str, from: SearchFrom, page: usize) {
    let found = list.find(query, from, page);
    if let Modal::Search(bar) = list.modal_mut() {
        bar.set_found(found);
    }
}

/// Puts the cursor on the PID typed so far, when there is one and that
/// process is shown.
fn jump(list: &mut TaskList, pid: Option<Pid>, page: usize) {
    if let Some(pid) = pid {
        list.jump_to_pid(pid, page);
    }
}

/// Applies an editing key to `line`, and gives its text when the key
/// changed it; caret moves and other keys give `None`.
fn edited(line: &mut LineEditor, code: KeyCode, modifiers: KeyModifiers) -> Option<String> {
    let before = line.text().to_owned();
    edit(line, code, modifiers);
    (line.text() != before).then(|| line.text().to_owned())
}

/// The list keys that still move the cursor while a bar is open, as the
/// keymap reads them; Home and End move the bar's caret instead.
const fn list_motion(key: KeyEvent) -> Option<ListMotion> {
    match resolve(key) {
        TaskCommand::Move(
            motion
            @ (ListMotion::Up | ListMotion::Down | ListMotion::PageUp | ListMotion::PageDown),
        ) => Some(motion),
        _ => None,
    }
}
