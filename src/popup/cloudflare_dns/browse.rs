//! Purpose: Keys in the lists. The arrows move, Tab switches the list, Enter edits the record and
//! `a` and `r` ask for the host and the zones. Esc and F10 close the dialog.

use super::typed;
use crate::models::cloudflare_dns::{CloudflareDnsPanel, Focus};
use crate::ops::cloudflare_dns as ops;
use crossterm::event::{KeyCode, KeyModifiers};

/// Returns whether the dialog closes.
pub(super) fn process(
    panel: &mut CloudflareDnsPanel,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> bool {
    if matches!(code, KeyCode::Esc | KeyCode::F(10)) {
        return true;
    }
    if panel.busy.is_some() {
        return false;
    }
    match code {
        KeyCode::Up => ops::step(panel, false),
        KeyCode::Down => ops::step(panel, true),
        KeyCode::Tab | KeyCode::BackTab => panel.toggle_focus(),
        KeyCode::Enter => match panel.focus {
            Focus::Zones => panel.focus = Focus::Records,
            Focus::Records => ops::edit_selected(panel),
        },
        KeyCode::Char(ch) if typed(modifiers) => match ch.to_ascii_lowercase() {
            'a' => ops::add_host(panel),
            'r' => ops::reload(panel),
            _ => {}
        },
        _ => {}
    }
    false
}
