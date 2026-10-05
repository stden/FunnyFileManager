//! Purpose: the keys behind the vault's buttons — unlock, reveal and cancel on the master-password prompt, confirm or cancel on its dialogs.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(in crate::application::dispatch::dialog_mouse) fn kdbx_button_key(
    session: &crate::kdbx::KdbxSession,
    index: usize,
) -> Option<KeyEvent> {
    if session.prompt().is_some() {
        return match index {
            0 => Some(KeyCode::Enter.into()),
            1 => Some(KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL)),
            2 => Some(KeyCode::Esc.into()),
            _ => None,
        };
    }
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
