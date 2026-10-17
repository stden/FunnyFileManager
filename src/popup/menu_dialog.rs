//! Modal keyboard input for menu prompts and result lists.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, MenuDialog, Overlay};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    if matches!(code, KeyCode::Esc | KeyCode::F(10)) {
        if code == KeyCode::Esc && !app.settings.enabled(crate::menu::SettingsPage::Dialog) {
            return true;
        }
        app.close_overlay();
        return true;
    }
    if code == KeyCode::Enter {
        if let Err(error) = accept(app) {
            app.set_status(crate::status::Status::Error(error.to_string()));
        }
        return true;
    }
    let Overlay::MenuDialog(dialog) = &mut app.overlay else {
        return false;
    };
    match dialog.as_mut() {
        MenuDialog::Prompt { input, .. } => {
            super::text_field::edit(input, code, modifiers, &mut app.clipboard);
        }
        MenuDialog::Choices { rows, selected, .. } => navigate(selected, rows.len(), code),
    }
    true
}

fn accept(app: &mut App) -> std::io::Result<()> {
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        return Ok(());
    };
    match dialog.as_ref() {
        MenuDialog::Prompt { command, input, .. } => {
            let (command, text) = (*command, input.text().to_owned());
            crate::ops::menu::submit(app, command, &text)
        }
        MenuDialog::Choices { rows, selected, .. } => {
            if let Some(row) = rows.get(*selected) {
                crate::ops::menu::choose(app, row.target.clone())?;
            }
            Ok(())
        }
    }
}

/// Rows Page Up and Page Down move a choice list's cursor.
const MENU_PAGE: usize = 10;

/// Moves a list cursor `selected` over `length` rows by one navigation key,
/// never past either end. Shared by the F9 choice lists and the task list's
/// menus; the LSP popup and the vault list move their own cursors.
pub(super) fn navigate(selected: &mut usize, length: usize, code: KeyCode) {
    let last = length.saturating_sub(1);
    *selected = match code {
        KeyCode::Up => selected.saturating_sub(1),
        KeyCode::Down => selected.saturating_add(1).min(last),
        KeyCode::PageUp => selected.saturating_sub(MENU_PAGE),
        KeyCode::PageDown => selected.saturating_add(MENU_PAGE).min(last),
        KeyCode::Home => 0,
        KeyCode::End => last,
        _ => *selected,
    };
}

#[cfg(test)]
mod tests;
