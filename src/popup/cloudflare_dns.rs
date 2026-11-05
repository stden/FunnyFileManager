//! Purpose: Keys for the Cloudflare DNS dialog. The lists move with the arrows, the form is typed
//! into, and a write waits for its own Enter. Esc steps back, and then closes the dialog.

mod browse;
mod confirm;
mod form;
#[cfg(test)]
mod tests;

use crate::models::cloudflare_dns::ViewKind;
use crate::models::{App, Overlay};
use crossterm::event::{KeyCode, KeyModifiers};

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::CloudflareDns(panel) = &mut app.overlay else {
        return false;
    };
    let kind = panel.view.kind();
    let busy = panel.busy.is_some();
    let close = match kind {
        ViewKind::Browse => browse::process(panel, code, modifiers),
        ViewKind::Edit => {
            form::process(panel, code, modifiers);
            false
        }
        ViewKind::Confirm => {
            confirm::process(panel, code, busy);
            false
        }
    };
    if close {
        app.close_overlay();
    }
    true
}

/// A key that types a character: no Ctrl or Alt chord.
fn typed(modifiers: KeyModifiers) -> bool {
    !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
}
