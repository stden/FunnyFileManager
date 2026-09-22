//! Mask prompt input and mark-selection acceptance.

use crate::models::{App, Overlay};
use crossterm::event::{KeyCode, KeyModifiers};

use crate::status::Status;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::MaskPrompt(prompt) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab | KeyCode::Left | KeyCode::Right => prompt.toggle_button(),
        KeyCode::Backspace => prompt.backspace(),
        KeyCode::Enter => {
            let cancelled = prompt.on_cancel();
            let purpose = prompt.purpose();
            let mask = crate::mask::Mask::parse(prompt.mask());
            app.close_overlay();
            if !cancelled {
                apply_mask(app, purpose, &mask);
            }
        }
        KeyCode::Char(ch)
            if !modifiers.contains(KeyModifiers::CONTROL)
                && !modifiers.contains(KeyModifiers::ALT) =>
        {
            prompt.push(ch);
        }
        _ => {}
    }
    true
}

/// Gray + / Gray −: add or remove every row the mask names.
///
/// The count is reported rather than the mask, because "12 files selected" is
/// the answer to the question the user actually asked, and a mask that matched
/// nothing is otherwise indistinguishable from a key that did not register.
fn apply_mask(app: &mut App, purpose: crate::models::MaskPurpose, mask: &crate::mask::Mask) {
    let changed = app
        .get_active_panel()
        .mark_matching(mask, purpose.selects());
    let marked = app.get_active_panel().marked.len();
    app.set_status(Status::CommandOutput(format!(
        "{changed} changed, {marked} selected"
    )));
}
