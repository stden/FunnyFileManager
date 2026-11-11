//! Interpret WSL dialog commands while keeping every key inside the overlay.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::wsl::{resolve, WslCommand};
use crate::models::{App, Overlay, WslActivity};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    if !matches!(app.overlay, Overlay::Wsl(_)) {
        return false;
    }
    let command = resolve(&app.keymap.wsl_manager, KeyEvent::new(code, modifiers));
    apply(app, command);
    true
}

/// Apply `command` to the open dialog — a key's, or one of its buttons'.
/// While a request runs only the moves and Close answer.
pub fn apply(app: &mut App, command: WslCommand) {
    let Overlay::Wsl(dialog) = &mut app.overlay else {
        return;
    };
    let busy = matches!(dialog.activity, WslActivity::Busy { .. });
    match command {
        WslCommand::Up => dialog.move_up(),
        WslCommand::Down => dialog.move_down(),
        WslCommand::First => dialog.first(),
        WslCommand::Last => dialog.last(),
        WslCommand::Close => app.close_overlay(),
        WslCommand::Open if !busy => crate::ops::wsl::open_selected(app),
        WslCommand::Refresh if !busy => crate::ops::wsl::refresh(app),
        WslCommand::Update if !busy => crate::ops::wsl::update(app),
        WslCommand::UpdatePackages if !busy => crate::ops::wsl::update_packages(app),
        WslCommand::Open
        | WslCommand::Refresh
        | WslCommand::Update
        | WslCommand::UpdatePackages
        | WslCommand::Ignore => {}
    }
}

#[cfg(test)]
mod tests;
