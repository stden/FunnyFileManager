//! Purpose: Keys on the confirmation. Enter applies the change and Esc returns to the form. While the
//! change is being saved, neither key does anything.

use crate::models::cloudflare_dns::CloudflareDnsPanel;
use crate::ops::cloudflare_dns as ops;
use crossterm::event::KeyCode;

pub(super) fn process(panel: &mut CloudflareDnsPanel, code: KeyCode, busy: bool) {
    match code {
        KeyCode::Enter if !busy => ops::apply(panel),
        KeyCode::Esc if !busy => ops::back(panel),
        _ => {}
    }
}
