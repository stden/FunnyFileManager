//! Translate main-view input into application transitions and canonical effects.
//!
//! Keys and mouse gestures share one modal-first path. Routine operation failures
//! update status; terminal failures remain the event loop's responsibility.

use std::io::{self, Write};
use std::path::PathBuf;

use crossterm::event::KeyEvent;
use ratatui::{backend::Backend, Terminal};

use crate::action::{resolve, Action};
use crate::diagnostics::{action_label, log_action};
use crate::models::{App, PanelKind, ViewMode};
use crate::popup::process_popup_input;

mod actions;
mod console;
mod dialog_mouse;
mod mouse;
mod navigation;
mod panel_mouse;
mod prompts;

use actions::apply_panel_action;
#[cfg(test)]
use actions::apply_plain_action;
use console::console_action;
pub(super) use mouse::dispatch_mouse;

/// Preserve the active drive or UNC share when navigating to its root.
fn filesystem_root(path: &std::path::Path) -> PathBuf {
    path.ancestors().last().unwrap_or(path).to_path_buf()
}

/// Run one fallible user operation and surface any failure in the status bar.
///
/// Routine I/O failure — F3 on a binary file, F5 into a read-only directory,
/// Enter on a directory the user cannot read — must never leave the event loop:
/// `run_app`'s caller treats an `Err` as fatal, so a `?` here would quit the
/// file manager on a bad keypress. Only a failed terminal draw is fatal.
fn report(app: &mut App, operation: impl FnOnce(&mut App) -> io::Result<()>) {
    if let Err(error) = operation(app) {
        app.set_status(crate::status::Status::Error(error.to_string()));
    }
}

/// Apply a resolved [`Action`] to the application state, performing its effect.
///
/// The dispatch half of the Input→Action DSL: [`crate::action::resolve`] turns a
/// key into an [`Action`]; this turns that [`Action`] into an effect on `app`.
/// Updates [`Lifecycle`] when an action should terminate the event loop.
///
/// Infallible by construction: every fallible effect goes through [`report`],
/// so a failing file operation can only ever reach the status bar, never the
/// event loop.
fn apply_action<B>(terminal: &mut Terminal<B>, app: &mut App, action: Action)
where
    B: Backend + Write,
{
    log_action(&action_label(action));
    if app.view == ViewMode::Console {
        console_action(terminal, app, action);
        return;
    }
    apply_panel_action(terminal, app, action);
}

pub fn dispatch_key<B>(terminal: &mut Terminal<B>, app: &mut App, key: KeyEvent)
where
    B: Backend + Write,
{
    if app.is_overlay_active() {
        process_popup_input(app, key);
        if !app.lifecycle.is_running() {
            log_action("Quitting from overlay");
        }
        // A menu row runs its command by pressing the command's own key, so
        // the menu and the keyboard cannot drift apart and the shortcut
        // printed beside a row is the one that actually runs.
        if let Some(shortcut) = app.pending_key.take() {
            let chosen = KeyEvent::new(shortcut.code, shortcut.modifiers);
            dispatch_key(terminal, app, chosen);
        }
        return;
    }
    if panel_kind_consumes(app, key) {
        return;
    }

    apply_action(terminal, app, resolve(key));
}

/// A tree or quick-view panel is asked about a key before the file-panel
/// actions are: its rows are folders or a preview, not the entries the
/// actions would act on. `Ctrl+O`'s console has no panels to ask.
fn panel_kind_consumes(app: &mut App, key: KeyEvent) -> bool {
    if app.view != ViewMode::Panels {
        return false;
    }
    match app.get_active_panel_ref().kind {
        PanelKind::Files => false,
        PanelKind::Tree(_) => crate::popup::tree::process(app, key),
        PanelKind::QuickView(_) => crate::popup::quickview::process(app, key),
    }
}

#[cfg(test)]
mod tests;
