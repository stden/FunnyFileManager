//! Purpose: What a click on the task list lands on — a table row, a column
//! title, a key-bar cell or a menu line — answered from the same layout the
//! renderer draws with, so a click lands on what the user saw.
//!
//! A click only ever names a key or a transition that already exists: the
//! most it can do is press F8 or F9, which open a confirmation, so no click
//! on the list can send a signal by itself.

use ratatui::layout::{Position, Rect};

use crate::models::{Modal, SortKey, TaskList};
use crate::mouse::fkey_at;

use super::super::fkey_bar::FKEY_COUNT;
use super::layout;
use super::menus::MenuView;

/// What a click on the task list lands on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskClick {
    /// A table row, by its place in the shown rows.
    Row(usize),
    /// A column title: sort by its key.
    Title(SortKey),
    /// A key-bar cell: press `F<n>`.
    Key(u8),
    /// A line of the open menu.
    MenuLine(usize),
}

/// What a click at (`column`, `row`) lands on, when `list` is drawn on
/// `screen`: in Browse a row, a column title or a key-bar cell; in a menu,
/// a line. Anything else — a dialog, a bar, the help page — answers
/// through its own buttons or not at all.
pub fn task_click_at(screen: Rect, list: &TaskList, column: u16, row: u16) -> Option<TaskClick> {
    match list.modal() {
        Modal::Browse => browse_click(screen, list, Position::new(column, row)),
        Modal::Menu(menu) => MenuView::of(list, menu)
            .row_at(screen, column, row)
            .map(TaskClick::MenuLine),
        _ => None,
    }
}

fn browse_click(screen: Rect, list: &TaskList, at: Position) -> Option<TaskClick> {
    let layout = layout(screen, list);
    let (titles, body) = (layout.rows.titles, layout.rows.body);
    if layout.bottom.contains(at) {
        let x = at.x.saturating_sub(screen.x);
        return fkey_at(screen.width, FKEY_COUNT, x).map(TaskClick::Key);
    }
    if titles.contains(at) {
        let x = usize::from(at.x.saturating_sub(titles.x));
        let clicked = layout.columns.at(x)?;
        return Some(TaskClick::Title(clicked.sort_key(list.command_view())));
    }
    if body.contains(at) {
        let offset = usize::from(at.y.saturating_sub(body.y));
        let index = layout.window.start.saturating_add(offset);
        return (index < layout.window.end).then_some(TaskClick::Row(index));
    }
    None
}
