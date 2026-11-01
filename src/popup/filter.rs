//! Purpose: Edit and apply the file panel filter.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{App, FilterField, Overlay};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    if !matches!(app.overlay, Overlay::Filter(_)) {
        return false;
    }
    if mnemonic(app, code, modifiers) {
        return true;
    }
    match code {
        KeyCode::Esc if !app.settings.enabled(crate::menu::SettingsPage::Dialog) => {}
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab | KeyCode::Down => advance(app, true),
        KeyCode::BackTab | KeyCode::Up => advance(app, false),
        KeyCode::Enter if app.filter_input.field == FilterField::Cancel => app.close_overlay(),
        KeyCode::Enter | KeyCode::F(2) => crate::ops::filter::apply(app),
        KeyCode::Char(' ') if app.filter_input.field == FilterField::Hidden => toggle_hidden(app),
        KeyCode::Char(' ') if app.filter_input.field == FilterField::Apply => {
            crate::ops::filter::apply(app);
        }
        KeyCode::Char(' ') if app.filter_input.field == FilterField::Cancel => app.close_overlay(),
        _ => edit(app, code, modifiers),
    }
    true
}

fn advance(app: &mut App, forward: bool) {
    app.filter_input.field = app
        .filter_input
        .field
        .advance(forward, app.filter_input.additional.is_some());
}

fn edit(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    let line = match app.filter_input.field {
        FilterField::Include => &mut app.filter_input.include,
        FilterField::Additional => {
            let Some(line) = &mut app.filter_input.additional else {
                return;
            };
            line
        }
        FilterField::Exclude => &mut app.filter_input.exclude,
        _ => return,
    };
    super::text_field::edit(line, code, modifiers, &mut app.clipboard);
    sync(app);
}

/// Bracketed paste is inserted only into the selected text field.
pub(super) fn paste(app: &mut App, text: &str) {
    if let Some(line) = app.focused_text_field() {
        line.insert_str(&super::text_field::one_line(text));
        sync(app);
    }
}

fn sync(app: &mut App) {
    if let Overlay::Filter(dialog) = &mut app.overlay {
        app.filter_input
            .include
            .text()
            .clone_into(&mut dialog.include);
        app.filter_input
            .exclude
            .text()
            .clone_into(&mut dialog.exclude);
    }
    app.filter_input.error = None;
}

fn toggle_hidden(app: &mut App) {
    if let Overlay::Filter(dialog) = &mut app.overlay {
        dialog.hide_dotfiles = !dialog.hide_dotfiles;
    }
}

fn mnemonic(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    if modifiers != KeyModifiers::ALT && modifiers != (KeyModifiers::ALT | KeyModifiers::SHIFT) {
        return false;
    }
    match code {
        KeyCode::Char('i' | 'I') => app.filter_input.field = FilterField::Include,
        KeyCode::Char('e' | 'E') => app.filter_input.field = FilterField::Exclude,
        KeyCode::Char('x' | 'X') if app.filter_input.additional.is_some() => {
            app.filter_input.field = FilterField::Additional;
        }
        KeyCode::Char('d' | 'D') => {
            app.filter_input.field = FilterField::Hidden;
            toggle_hidden(app);
        }
        KeyCode::Char('a' | 'A') => crate::ops::filter::apply(app),
        KeyCode::Char('c' | 'C') => app.close_overlay(),
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_dual;
#[cfg(test)]
mod tests_flow;
#[cfg(test)]
mod tests_mnemonics;
#[cfg(test)]
mod tests_more;
