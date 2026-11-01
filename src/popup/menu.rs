//! Menu navigation and deferred shortcut dispatch.

use crate::models::{App, Overlay};
use crossterm::event::{KeyCode, KeyModifiers};

#[cfg(test)]
mod tests;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::Menu(menu) = &mut app.overlay else {
        return false;
    };
    let mut menu = *menu;
    if modifiers == KeyModifiers::ALT || modifiers == (KeyModifiers::ALT | KeyModifiers::SHIFT) {
        if let KeyCode::Char(ch) = code {
            if let Some(heading) = crate::menu::Menu::heading_for_hotkey(ch) {
                app.set_menu(crate::menu::Menu::open(heading));
            }
        }
        return true;
    }
    match code {
        // F9 both opens and closes the bar. Far's does, and a key that
        // only opens leaves the user hunting for the one that shuts it.
        KeyCode::Esc | KeyCode::F(9 | 10) => app.close_overlay(),
        KeyCode::Left | KeyCode::Right => {
            menu.go(code == KeyCode::Right);
            app.set_menu(menu);
        }
        KeyCode::Up | KeyCode::Down => move_row(app, menu, code == KeyCode::Down),
        // The menu closes before the command runs, so a command that opens
        // a dialog of its own is not fighting the drop-down for the screen.
        KeyCode::Enter => crate::ops::menu::activate_row(app, menu),
        // A letter picks a row here, or a heading if this menu has no
        // row for it — Far lets the bar be walked by its letters
        // whichever drop-down happens to be open.
        KeyCode::Char(ch) if !modifiers.contains(KeyModifiers::CONTROL) => {
            activate_letter(app, menu, ch);
        }
        _ => {}
    }
    true
}

fn move_row(app: &mut App, mut menu: crate::menu::Menu, down: bool) {
    let previous = menu.row;
    menu.step(down);
    if !app.settings.enabled(crate::menu::SettingsPage::Menu)
        && ((down && menu.row < previous) || (!down && menu.row > previous))
    {
        menu.row = previous;
    }
    app.set_menu(menu);
}

fn activate_letter(app: &mut App, mut menu: crate::menu::Menu, ch: char) {
    if let Some(row) = menu.row_for_hotkey(ch) {
        menu.row = row;
        crate::ops::menu::activate_row(app, menu);
    } else if let Some(heading) = crate::menu::Menu::heading_for_hotkey(ch) {
        app.set_menu(crate::menu::Menu::open(heading));
    }
}
