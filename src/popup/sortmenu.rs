//! Purpose: Keys for the Sort by menu (Ctrl+F12), as Far's help lists them:
//! the arrows walk the rows, Enter, a row's letter or its `Ctrl+F` key choose
//! one, `+` `-` `*` set the order outright, and Esc / F10 leave the panel as
//! it was.
//!
//! Every choice closes the menu before it acts, because acting re-lists the
//! panel and the menu's ticks are a snapshot of the panel as it was.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, Overlay, SortOrder, SortRow};
use crate::ops::sortmenu;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::SortModes(menu) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Up => menu.up(),
        KeyCode::Down => menu.down(),
        KeyCode::Home => menu.first(),
        KeyCode::End => menu.last(),
        KeyCode::Enter => {
            let row = menu.row();
            choose(app, row);
        }
        KeyCode::F(number) if modifiers == KeyModifiers::CONTROL => {
            if let Some(row) = SortRow::by_function_key(number) {
                choose(app, row);
            }
        }
        KeyCode::Char(ch) => character(app, ch, modifiers),
        _ => {}
    }
    true
}

/// `+` `-` `*` set the order the way Far's help describes; any other letter
/// names a row — unless Ctrl or Alt holds it, since Alt+letter is quick
/// search's chord and must not pick a row the user did not mean.
fn character(app: &mut App, ch: char, modifiers: KeyModifiers) {
    match ch {
        '+' => set_order(app, SortOrder::Direct),
        '-' => set_order(app, SortOrder::Reversed),
        '*' => {
            app.close_overlay();
            sortmenu::toggle_order(app);
        }
        letter if !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => {
            if let Some(row) = SortRow::by_hotkey(letter) {
                choose(app, row);
            }
        }
        _ => {}
    }
}

fn choose(app: &mut App, row: SortRow) {
    app.close_overlay();
    sortmenu::choose(app, row);
}

fn set_order(app: &mut App, order: SortOrder) {
    app.close_overlay();
    sortmenu::order(app, order);
}

#[cfg(test)]
mod tests;
