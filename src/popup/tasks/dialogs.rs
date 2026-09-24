//! Purpose: The task list's dialogs: the help page, the information and
//! error dialogs, the Select/Deselect mask dialog, the kill confirmation,
//! the Priority form and the confirmation `[` asks for.
//!
//! A signal or a renice happens only through a button of a confirmation or
//! of the form, pressed by Enter or by the button's letter; every other key
//! only moves the focus, edits, or closes.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::tasks::{normalize, HELP};
use crate::models::{App, ConfirmButton, FormFocus, Modal, NiceStep, ReniceButton};
use crate::ops::tasks::control;
use crate::popup::mask_prompt::{mask_key, MaskKey};
use crate::popup::menu_dialog::edit;
use crate::scroll::ScrollTop;

/// The help page: the list keys scroll it; Esc, F1, F10, `q`, `h` and `?`
/// close it.
pub(super) fn help(app: &mut App, key: KeyEvent) {
    let viewport = app.viewport;
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let page = list.page_for(viewport);
    let Modal::Help(top) = list.modal_mut() else {
        return;
    };
    let bottom = HELP.len().saturating_sub(page);
    *top = match normalize(key).code {
        KeyCode::Up => top.up(1),
        KeyCode::Down => top.down(1, bottom),
        KeyCode::PageUp => top.up(page),
        KeyCode::PageDown => top.down(page, bottom),
        KeyCode::Home => ScrollTop::TOP,
        KeyCode::End => ScrollTop::TOP.down(bottom, bottom),
        KeyCode::Esc | KeyCode::F(1 | 10) | KeyCode::Char('q' | 'h' | '?') => {
            list.close_modal();
            return;
        }
        _ => *top,
    };
}

/// Process information and the Error dialog: Enter, Esc, F10 and F3 close.
pub(super) fn dismiss(app: &mut App, key: KeyEvent) {
    let Some(list) = app.task_list_mut() else {
        return;
    };
    if matches!(key.code, KeyCode::Enter | KeyCode::Esc | KeyCode::F(3 | 10)) {
        list.close_modal();
    }
}

/// Gray `+` and `-`: the panel's own mask dialog keys; accepting marks or
/// unmarks the shown processes whose name matches, and says how many.
pub(super) fn mask(app: &mut App, key: KeyEvent) {
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let Modal::Mask(prompt) = list.modal_mut() else {
        return;
    };
    match mask_key(prompt, key.code, key.modifiers) {
        MaskKey::Editing => {}
        MaskKey::Cancelled => list.close_modal(),
        MaskKey::Accepted(purpose, mask) => {
            list.close_modal();
            let marked = list.mark_by_mask(purpose, &mask);
            list.set_notice(marked);
        }
    }
}

/// The kill confirmation: Enter presses the focused button, `t` Terminate,
/// `s` Send, `k` Kill (with or without Alt, never with Ctrl); Esc, F10 and
/// `c` cancel; Tab, Left and Right move the focus.
pub(super) fn confirm(app: &mut App, key: KeyEvent) {
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let Modal::Confirm(confirm) = list.modal_mut() else {
        return;
    };
    let key = normalize(key);
    let button = match key.code {
        KeyCode::Enter => confirm.focused(),
        KeyCode::Esc | KeyCode::F(10) => ConfirmButton::Cancel,
        KeyCode::Char(letter) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
            let Some(button) = hotkey(letter) else {
                return;
            };
            button
        }
        KeyCode::Tab | KeyCode::Right => {
            confirm.focus_next();
            return;
        }
        KeyCode::BackTab | KeyCode::Left => {
            confirm.focus_previous();
            return;
        }
        _ => return,
    };
    control::press(app, button);
}

/// The button a letter presses in the kill confirmation, either case.
const fn hotkey(letter: char) -> Option<ConfirmButton> {
    match letter.to_ascii_lowercase() {
        't' => Some(ConfirmButton::Terminate),
        's' => Some(ConfirmButton::Send),
        'k' => Some(ConfirmButton::Kill),
        'c' => Some(ConfirmButton::Cancel),
        _ => None,
    }
}

/// The Priority form: the field edits and Up and Down step it; Tab and
/// Shift+Tab move between the field, Set and Cancel; Enter presses Set,
/// which does nothing while the value is not a nice value, or Cancel when
/// it is focused; Esc and F10 cancel.
pub(super) fn priority(app: &mut App, key: KeyEvent) {
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let Modal::Priority(form) = list.modal_mut() else {
        return;
    };
    match key.code {
        KeyCode::Esc | KeyCode::F(10) => list.close_modal(),
        KeyCode::Enter if form.focus() == FormFocus::Cancel => list.close_modal(),
        KeyCode::Enter => control::set_nice(app),
        KeyCode::Tab => form.focus_next(),
        KeyCode::BackTab => form.focus_previous(),
        KeyCode::Up => form.step(NiceStep::Raise),
        KeyCode::Down => form.step(NiceStep::Lower),
        code if form.focus() == FormFocus::Field => edit(form.value_mut(), code, key.modifiers),
        _ => {}
    }
}

/// The confirmation `[` asks for: Enter presses the focused button, Esc and
/// F10 cancel, and Tab, Left and Right move the focus.
pub(super) fn renice(app: &mut App, key: KeyEvent) {
    let Some(list) = app.task_list_mut() else {
        return;
    };
    let Modal::Renice(confirm) = list.modal_mut() else {
        return;
    };
    let button = match key.code {
        KeyCode::Enter => confirm.focused(),
        KeyCode::Esc | KeyCode::F(10) => ReniceButton::Cancel,
        KeyCode::Tab | KeyCode::BackTab | KeyCode::Left | KeyCode::Right => {
            confirm.focus_other();
            return;
        }
        _ => return,
    };
    control::answer_renice(app, button);
}
