//! Purpose: Keys on the record form. Tab and the arrows move between fields, typing edits the text
//! field under the cursor, Space switches the proxy, Enter asks for the confirmation, and Esc goes
//! back to the list.

use super::typed;
use crate::models::cloudflare_dns::{CloudflareDnsPanel, FormField, RecordForm, View};
use crate::ops::cloudflare_dns as ops;
use crossterm::event::{KeyCode, KeyModifiers};

pub(super) fn process(panel: &mut CloudflareDnsPanel, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Esc => ops::back(panel),
        KeyCode::Enter => ops::review(panel),
        _ => {
            if let View::Edit(form) = &mut panel.view {
                edit(form, code, modifiers);
            }
        }
    }
}

fn edit(form: &mut RecordForm, code: KeyCode, modifiers: KeyModifiers) {
    match code {
        KeyCode::Tab | KeyCode::Down => form.step(true),
        KeyCode::BackTab | KeyCode::Up => form.step(false),
        KeyCode::Backspace => form.backspace(),
        KeyCode::Char(' ') if form.field == FormField::Proxy => form.toggle_proxy(),
        KeyCode::Char(ch) if typed(modifiers) => form.type_char(ch),
        _ => {}
    }
}
