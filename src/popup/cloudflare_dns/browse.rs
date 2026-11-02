//! Purpose: Keys in the lists. The arrows move, Tab switches the list, Enter edits the record and
//! `a` and `r` ask for the host and the zones. Esc and F10 close the dialog.

use super::typed;
use crate::models::cloudflare_dns::Focus;
use crate::models::{App, Overlay};
use crate::ops::cloudflare_dns as ops;
use crossterm::event::{KeyCode, KeyModifiers};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    if matches!(code, KeyCode::Esc | KeyCode::F(10)) {
        app.close_overlay();
        return;
    }
    let Overlay::CloudflareDns(panel) = &mut app.overlay else {
        return;
    };
    if panel.busy.is_some() {
        return;
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
}
