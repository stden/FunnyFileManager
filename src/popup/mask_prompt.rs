//! Purpose: Mask prompt input and mark-selection acceptance.
//!
//! The dialog's keys are [`mask_key`], which only edits the prompt and says
//! what the key meant; the panel handler below acts on that answer. Any other
//! list that selects by a Far mask (the task list's Gray `+` and `-`) calls the
//! same function, so the dialog behaves identically wherever it opens.

use crate::mask::Mask;
use crate::models::{App, MaskPrompt, MaskPurpose, Overlay};
use crossterm::event::{KeyCode, KeyModifiers};

use crate::status::Status;

/// What one key did to an open mask dialog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum MaskKey {
    /// The key edited the field or moved the focus; the dialog stays open.
    Editing,
    /// Enter on OK: select or deselect by this mask, then close.
    Accepted(MaskPurpose, Mask),
    /// Esc, F10, or Enter on Cancel: close without selecting.
    Cancelled,
}

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Overlay::MaskPrompt(prompt) = &mut app.overlay else {
        return false;
    };
    match mask_key(prompt, code, modifiers) {
        MaskKey::Editing => {}
        MaskKey::Cancelled => app.close_overlay(),
        MaskKey::Accepted(purpose, mask) => {
            app.close_overlay();
            apply_mask(app, purpose, &mask);
        }
    }
    true
}

/// Applies one key to the mask dialog's field and buttons, and says whether
/// the dialog was accepted, cancelled or is still being edited.
pub(super) fn mask_key(prompt: &mut MaskPrompt, code: KeyCode, modifiers: KeyModifiers) -> MaskKey {
    match code {
        KeyCode::Esc | KeyCode::F(10) => return MaskKey::Cancelled,
        KeyCode::Enter if prompt.on_cancel() => return MaskKey::Cancelled,
        KeyCode::Enter => {
            return MaskKey::Accepted(prompt.purpose(), Mask::parse(prompt.mask()));
        }
        KeyCode::Tab => prompt.toggle_button(),
        // On a button, Left and Right walk the buttons, as Far's do; in the
        // field they move the caret — see `edit`.
        KeyCode::Left | KeyCode::Right if prompt.on_cancel() => prompt.toggle_button(),
        KeyCode::Char(ch)
            if !modifiers.contains(KeyModifiers::CONTROL)
                && !modifiers.contains(KeyModifiers::ALT) =>
        {
            prompt.push(ch);
        }
        _ => edit(prompt, code),
    }
    MaskKey::Editing
}

/// The field is an edit control: Left goes back to the typo, not to Cancel.
///
/// TODO: the same key table as `describe::edit` — two copies, so the Rule of
/// Three leaves them apart. A second list that selects by mask does not add a
/// third: it reuses this whole dialog through [`mask_key`]. Share the table
/// when a genuinely different dialog field needs one.
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
fn apply_mask(app: &mut App, purpose: MaskPurpose, mask: &Mask) {
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
#[cfg(test)]
mod tests_mask_key;
