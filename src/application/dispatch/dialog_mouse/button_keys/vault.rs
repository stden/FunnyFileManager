//! Purpose: the vault's buttons — the unlock prompt's Unlock, Reveal and Cancel as the vault's own commands; OK or Cancel on its dialogs as protocol keys.

use crossterm::event::{KeyCode, KeyEvent};

use crate::action::vault::VaultCommand;
use crate::models::{App, Overlay};

/// The command the `index`-th button of the unlock prompt stands for:
/// Unlock, Reveal, Cancel. `None` when the vault is not on its prompt.
pub(in crate::application::dispatch::dialog_mouse) fn vault_prompt_button(
    app: &App,
    index: usize,
) -> Option<VaultCommand> {
    let Overlay::Kdbx(session) = &app.overlay else {
        return None;
    };
    session.0.prompt()?;
    match index {
        0 => Some(VaultCommand::Activate),
        1 => Some(VaultCommand::Reveal),
        2 => Some(VaultCommand::Close),
        _ => None,
    }
}

/// The key a button of the unlocked vault's own dialogs stands for: OK is
/// Enter, every other button Esc. The list has no buttons, and neither has
/// the unlock prompt here — its buttons are commands.
pub(in crate::application::dispatch::dialog_mouse) fn kdbx_button_key(
    session: &crate::kdbx::KdbxSession,
    index: usize,
) -> Option<KeyEvent> {
    match session.vault()?.page {
        crate::kdbx::vault::Page::Password(_) | crate::kdbx::vault::Page::DeleteConfirm(_) => Some(
            (if index == 0 {
                KeyCode::Enter
            } else {
                KeyCode::Esc
            })
            .into(),
        ),
        crate::kdbx::vault::Page::Browse(_) => None,
    }
}
