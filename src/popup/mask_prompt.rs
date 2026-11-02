//! Purpose: Mask prompt input and mark-selection acceptance.
//!
//! The dialog's keys are [`mask_key`], which only edits the prompt and says
//! what the key meant; the panel handler below acts on that answer. Any other
//! list that selects by a Far mask (the task list's Gray `+` and `-`) calls the
//! same function, so the dialog behaves identically wherever it opens.

use crate::mask::Mask;
use crate::models::{App, ClipboardSlot, MaskPrompt, MaskPurpose, Overlay};
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
    match mask_key(prompt, code, modifiers, &mut app.clipboard) {
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
pub(super) fn mask_key(
    prompt: &mut MaskPrompt,
    code: KeyCode,
    modifiers: KeyModifiers,
    clipboard: &mut ClipboardSlot,
) -> MaskKey {
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
        _ => {
            super::text_field::edit(prompt.line_mut(), code, modifiers, clipboard);
        }
    }
    MaskKey::Editing
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
