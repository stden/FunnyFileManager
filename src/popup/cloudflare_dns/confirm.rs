//! Purpose: Keys on the confirmation. Enter applies the change and Esc returns to the form. While the
//! change is being saved, neither key does anything.

use crate::models::{App, Overlay};
use crate::ops::cloudflare_dns as ops;
use crossterm::event::KeyCode;

pub(super) fn process(app: &mut App, code: KeyCode, busy: bool) {
    let Overlay::CloudflareDns(panel) = &mut app.overlay else {
        return;
    };
    match code {
        KeyCode::Enter if !busy => ops::apply(panel),
        KeyCode::Esc if !busy => ops::back(panel),
        _ => {}
    }
}
