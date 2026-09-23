//! Mask prompt input and mark-selection acceptance.

use crate::models::{App, MaskPrompt, Overlay};
use crossterm::event::{KeyCode, KeyModifiers};

use crate::status::Status;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::MaskPrompt(prompt) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab => prompt.toggle_button(),
        // On a button, Left and Right walk the buttons, as Far's do; in the
        // field they move the caret — see `edit`.
        KeyCode::Left | KeyCode::Right if prompt.on_cancel() => prompt.toggle_button(),
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
        _ => edit(prompt, code),
    }
    true
}

/// The field is an edit control: Left goes back to the typo, not to Cancel.
///
/// TODO: the same key table as `describe::edit`. Share it when a third
/// dialog field needs one (rule of three).
fn edit(prompt: &mut MaskPrompt, code: KeyCode) {
    match code {
        KeyCode::Left => prompt.left(),
        KeyCode::Right => prompt.right(),
        KeyCode::Home => prompt.home(),
        KeyCode::End => prompt.end(),
        KeyCode::Backspace => prompt.backspace(),
        KeyCode::Delete => prompt.delete_forward(),
        _ => {}
    }
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

#[cfg(test)]
mod tests;
