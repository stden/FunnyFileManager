//! Purpose: what a click on a dialog or a menu means.
//!
//! Split out of the event loop because it is a different question. The loop
//! asks *what happened*; this asks *what the user meant by it* — which button
//! is under the pointer, which key that button stands for, and whether a press
//! on blank dialog is a grip rather than a command.
//!
//! Every answer is the key of the dialog protocol the control stands for,
//! or, where the screen has commands of its own — the viewer, the editor's
//! text, the task list's table, the WSL manager, the vault's unlock prompt —
//! that command. A click is turned into what already does the job, rather
//! than into a second way of doing it, so the mouse and the keyboard cannot
//! drift apart and a dialog gains mouse support by naming its buttons
//! rather than by growing a handler of its own.

use std::io::Write;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{backend::Backend, Terminal};

use crate::models::App;
use crate::mouse::{DialogTarget, Wheel};
use crate::popup;

mod button_keys;
mod menus;
mod rows;
mod tasks;
#[cfg(test)]
mod tests;

use button_keys::dialog_button_key;
// The tests module reaches these through `super::*`.
#[cfg(test)]
use button_keys::dialog_button_key_in_editor;
#[cfg(all(test, feature = "keepass"))]
use button_keys::kdbx_button_key;
#[cfg(feature = "keepass")]
use button_keys::vault_prompt_button;
use menus::{click_choice_menu, click_in_menu, click_sort_menu};
use rows::click_dialog_row;

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
    if let Some(wheel) = Wheel::of(mouse.kind) {
        // A screen with scroll commands of its own takes the wheel as one;
        // a dialog takes it as the Up or Down key of its protocol.
        if !popup::wheel(app, wheel, mouse.modifiers) {
            let code = match wheel {
                Wheel::Up => KeyCode::Up,
                Wheel::Down => KeyCode::Down,
            };
            super::dispatch_key(terminal, app, KeyEvent::new(code, mouse.modifiers));
        }
        return;
    }
    match mouse.kind {
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
        DialogTarget::Button(index) => press_button(terminal, app, index),
        DialogTarget::Row(index) => click_dialog_row(app, index),
        DialogTarget::Outside | DialogTarget::Popup => {}
    }
}

/// The `index`-th button: the vault's unlock prompt has commands of its own;
/// every other dialog's button is a key of its protocol.
fn press_button<B: Backend + Write>(terminal: &mut Terminal<B>, app: &mut App, index: usize) {
    #[cfg(feature = "keepass")]
    if let Some(command) = vault_prompt_button(app, index) {
        popup::kdbx::apply(app, command);
        return;
    }
    if let Some(key) = dialog_button_key(app, index) {
        super::dispatch_key(terminal, app, key);
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
        if let Some(command) = clicked {
            popup::wsl::apply(app, command);
        }
        return true;
    }
    if let crate::models::Overlay::Menu(menu) = &app.overlay {
        click_in_menu(app, *menu, column, row);
        // The same handoff the keyboard does: the chosen row runs once the
        // menu has left the screen.
        super::run_pending_row(terminal, app);
        return true;
    }

    false
}
