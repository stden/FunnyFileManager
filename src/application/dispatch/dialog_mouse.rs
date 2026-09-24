//! Purpose: what a click on a dialog or a menu means.
//!
//! Split out of the event loop because it is a different question. The loop
//! asks *what happened*; this asks *what the user meant by it* — which button
//! is under the pointer, which key that button stands for, and whether a press
//! on blank dialog is a grip rather than a command.
//!
//! Every answer is a keystroke. A click is turned into the key that already
//! does the job, rather than into a second way of doing it, so the mouse and
//! the keyboard cannot drift apart and a dialog gains mouse support by naming
//! its buttons rather than by growing a handler of its own.

use std::io::Write;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::Backend, Terminal};

use crate::models::App;
use crate::mouse::DialogTarget;

mod button_keys;
mod menus;
mod tasks;
#[cfg(test)]
mod tests;

use button_keys::dialog_button_key;
// The tests module reaches these through `super::*`.
#[cfg(test)]
use button_keys::dialog_button_key_in_editor;
#[cfg(all(test, feature = "keepass"))]
use button_keys::kdbx_button_key;
use menus::{click_choice_menu, click_in_menu, click_sort_menu};

/// The pointer, while a dialog owns the screen.
///
/// Three things happen here: a press on a control acts on it, a press anywhere
/// else takes hold of the dialog, and a drag carries it. The dialog is the
/// only thing on screen that answers, which is what "modal" means — a click
/// on the panels behind it does nothing rather than something surprising.
pub(super) fn dispatch_dialog_mouse<B>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    mouse: crossterm::event::MouseEvent,
) where
    B: Backend + Write,
{
    use crossterm::event::{MouseButton, MouseEventKind};

    let (column, row) = (mouse.column, mouse.row);
    match mouse.kind {
        MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
            let code = if mouse.kind == MouseEventKind::ScrollUp {
                KeyCode::Up
            } else {
                KeyCode::Down
            };
            super::dispatch_key(terminal, app, KeyEvent::new(code, mouse.modifiers));
        }
        MouseEventKind::Down(MouseButton::Left) => {
            click_dialog(terminal, app, column, row);
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            if let Some((from_x, from_y)) = app.dialog_mouse.grabbed {
                // The offset moves by the pointer's travel, and the grip stays
                // put: tracking against the press point instead would make the
                // dialog jump by its own displacement on every event.
                app.dialog_mouse.offset.0 += i32::from(column) - i32::from(from_x);
                app.dialog_mouse.offset.1 += i32::from(row) - i32::from(from_y);
                app.dialog_mouse.grabbed = Some((column, row));
            }
        }
        MouseEventKind::Up(MouseButton::Left) => app.dialog_mouse.grabbed = None,
        _ => {}
    }
}

fn click_dialog<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    column: u16,
    row: u16,
) {
    if click_special_dialog(terminal, app, column, row) {
        return;
    }
    let target = app.dialog_mouse.hit(column, row);
    crate::diagnostics::log_action(&format!("Dialog click: {target:?}"));
    match target {
        DialogTarget::Chrome => {
            // Nothing to activate here, so the press is a grip: Far
            // lets a dialog be pushed aside by any blank part of it,
            // which is how you look at what is underneath.
            app.dialog_mouse.grabbed = Some((column, row));
        }
        DialogTarget::Button(index) => {
            if let Some(key) = dialog_button_key(app, index) {
                super::dispatch_key(terminal, app, key);
            }
        }
        DialogTarget::Row(index) => click_dialog_row(app, index),
        DialogTarget::Outside | DialogTarget::Popup => {}
    }
}

fn click_special_dialog<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    column: u16,
    row: u16,
) -> bool {
    if click_sort_menu(terminal, app, column, row)
        || click_choice_menu(terminal, app, column, row)
        || tasks::click_task_list(terminal, app, column, row)
    {
        return true;
    }
    if let crate::models::Overlay::Wsl(dialog) = &app.overlay {
        let clicked = terminal.size().ok().and_then(|size| {
            let screen = ratatui::layout::Rect::new(0, 0, size.width, size.height);
            crate::ui::wsl_button_at(screen, dialog, column, row)
        });
        if let Some(code) = clicked {
            super::dispatch_key(terminal, app, KeyEvent::new(code, KeyModifiers::NONE));
        }
        return true;
    }
    if let crate::models::Overlay::Menu(menu) = &app.overlay {
        click_in_menu(app, *menu, column, row);
        // The same handoff the keyboard does: a chosen row runs its command by
        // pressing that command's key, once the menu has left the screen.
        if let Some(shortcut) = app.pending_key.take() {
            let chosen = KeyEvent::new(shortcut.code, shortcut.modifiers);
            super::dispatch_key(terminal, app, chosen);
        }
        return true;
    }

    false
}

/// A click on a dialog's `index`-th body row.
///
/// Only the dialogs with controls in their body answer; elsewhere a row is a
/// sentence or a caption, and clicking a sentence should do nothing rather
/// than something the user cannot predict.
///
/// In both, a checkbox answers to the click that reached it and every other
/// control takes the cursor and waits — which is what clicking a text field
/// means everywhere else.
fn click_dialog_row(app: &mut App, index: usize) {
    use crate::models::{NameField, Overlay};

    match &mut app.overlay {
        Overlay::Editor(editor) if editor.encoding_choice.is_some() => {
            if let Some(encoding) = crate::encoding::TextEncoding::ALL.get(index) {
                editor.encoding_choice = Some(*encoding);
            }
        }
        Overlay::CopyPrompt(prompt) => click_copy_row(prompt, index),
        Overlay::LspEditor(editor) => click_lsp_form_row(editor, index),
        Overlay::NamePrompt(prompt) => {
            let Some(field) = prompt.field_at_row(index) else {
                return;
            };
            prompt.focus(field);
            if prompt.field() == NameField::Multiple {
                prompt.toggle();
            }
        }
        _ => {}
    }
}

/// The Language servers form, like the copy dialog: a field takes the cursor
/// and Enabled toggles as well. The list, the view and the exit question have
/// no clickable rows, and neither has an existing server's dimmed language.
fn click_lsp_form_row(editor: &mut crate::models::LspEditor, index: usize) {
    use crate::models::lsp_editor::FormField;

    let (Some(form), Some(field)) = (editor.form_mut(), FormField::at_row(index)) else {
        return;
    };
    if !field.allows(form.target()) {
        return;
    }
    form.focus_on(field);
    if field == FormField::Enabled {
        form.toggle_enabled();
    }
}

fn click_copy_row(prompt: &mut crate::copy::CopyPrompt, index: usize) {
    let Some(field) = crate::copy::field_at_row(index) else {
        return;
    };
    if !prompt.allows(field) {
        return;
    }
    prompt.field = field;
    if field.is_checkbox() {
        prompt.toggle();
    }
}
