//! Purpose: what a click on a menu means.
//!
//! The sort-modes list and a choice list answer a click on a row by moving
//! their cursor there and pressing Enter; the menu bar opens the heading under
//! the pointer, runs the entry under it, or closes when the click lands away.

use std::io::Write;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::Backend, Terminal};

use super::super::dispatch_key;
use crate::menu;
use crate::models::App;

pub(super) fn click_sort_menu<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    column: u16,
    row: u16,
) -> bool {
    let crate::models::Overlay::SortModes(menu) = &mut app.overlay else {
        return false;
    };
    let offset = app.dialog_mouse.offset;
    let clicked = terminal.size().ok().and_then(|size| {
        crate::ui::sort_row_at(
            ratatui::layout::Rect::new(0, 0, size.width, size.height),
            menu,
            offset,
            column,
            row,
        )
    });
    // A press that lands on no row falls through to the frame, which is a
    // grip: the menu can be pushed aside like any other dialog.
    let Some(index) = clicked else {
        return false;
    };
    menu.cursor = index;
    dispatch_key(
        terminal,
        app,
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
    );
    true
}

pub(super) fn click_choice_menu<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    column: u16,
    row: u16,
) -> bool {
    let crate::models::Overlay::MenuDialog(dialog) = &mut app.overlay else {
        return false;
    };
    let crate::models::MenuDialog::Choices {
        title,
        rows,
        selected,
    } = dialog.as_mut()
    else {
        return false;
    };
    let clicked = terminal.size().ok().and_then(|size| {
        crate::ui::choice_row_at(
            ratatui::layout::Rect::new(0, 0, size.width, size.height),
            title,
            rows,
            *selected,
            column,
            row,
        )
    });
    if let Some(index) = clicked {
        *selected = index;
        dispatch_key(terminal, app, KeyCode::Enter.into());
    }
    true
}

/// A click while the menu is open: on the bar, in the drop-down, or away.
pub(super) fn click_in_menu(app: &mut App, menu: menu::Menu, column: u16, row: u16) {
    let screen = app.dialog_mouse.screen;
    if row == screen.y {
        if let Some(heading) = crate::ui::heading_at(screen, column) {
            app.set_menu(menu::Menu::open(heading));
        }
        return;
    }
    if let Some(chosen) = crate::ui::menu_row_at(&menu, screen, column, row) {
        let mut menu = menu;
        menu.row = chosen;
        if let Some(entry) = menu.entry().copied() {
            crate::ops::menu::activate(app, entry);
        }
        return;
    }
    // Anywhere else closes it, which is what clicking away from an open menu
    // means everywhere else too.
    app.close_overlay();
}
