//! Translate main-view input into application transitions and canonical effects.
//!
//! Keys and mouse gestures share one modal-first path. Routine operation failures
//! update status; terminal failures remain the event loop's responsibility.

use std::io::{self, Write};
use std::path::PathBuf;

use crossterm::event::KeyEvent;
use ratatui::{backend::Backend, Terminal};

use crate::action::tree::TreeCommand;
use crate::action::viewer::ViewerScroll;
use crate::action::{is_menu_key, resolve, Action};
use crate::diagnostics::{action_label, log_action};
use crate::models::{App, PanelKind, ViewMode};
use crate::popup::{self, process_popup_input};

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
        app.set_status(crate::status::Status::of_error(&error));
    }
}

/// Apply a resolved [`Action`] to the application state, performing its effect.
///
/// The dispatch half of the Input→Action DSL: [`resolve`] turns a
/// key into an [`Action`]; this turns that [`Action`] into an effect on `app`.
/// Updates [`Lifecycle`] when an action should terminate the event loop.
///
/// Infallible by construction: every fallible effect goes through [`report`],
/// so a failing file operation can only ever reach the status bar, never the
/// event loop.
fn apply_action<B>(terminal: &mut Terminal<B>, app: &mut App, action: Action, menu_key: bool)
where
    B: Backend + Write,
{
    log_action(&action_label(action));
    // Far's fast find ends on any key that is not part of it, before that key
    // acts: a query typed here means nothing in another folder or panel.
    if !action.edits_quick_search() {
        app.quick_search.clear();
    }
    if app.view == ViewMode::Console {
        console_action(terminal, app, action, menu_key);
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
        run_pending_row(terminal, app);
        return;
    }
    if panel_kind_consumes(app, key) {
        return;
    }

    let action = resolve(&app.keymap.panels, key);
    let menu_key = is_menu_key(&app.keymap.panels, key);
    apply_action(terminal, app, action, menu_key);
}

/// Run the menu row chosen while the drop-down was up, now that it has left
/// the screen. The row holds the same [`Action`] its key resolves to and takes
/// the same road, so the menu and the keyboard cannot drift apart.
pub(super) fn run_pending_row<B>(terminal: &mut Terminal<B>, app: &mut App)
where
    B: Backend + Write,
{
    if let Some(action) = app.pending_row.take().and_then(|row| row.action()) {
        dispatch_action(terminal, app, action);
    }
}

/// Apply an action that arrived without a key — a menu row, a mouse gesture —
/// the way [`dispatch_key`] applies a key's: the active panel's kind reads it
/// first, then it acts on the panels or the console.
pub(super) fn dispatch_action<B>(terminal: &mut Terminal<B>, app: &mut App, action: Action)
where
    B: Backend + Write,
{
    if panel_kind_answers(app, action) {
        return;
    }
    apply_action(terminal, app, action, true);
}

/// [`panel_kind_consumes`] for an action instead of a key: the tree, quick
/// view, sessions and remote panels read the moves, Open and the file
/// actions whose keys they own as their own commands; a `NetBox` panel refuses
/// the rest that would act on the entries hidden beneath it.
fn panel_kind_answers(app: &mut App, action: Action) -> bool {
    if app.view != ViewMode::Panels {
        return false;
    }
    match app.get_active_panel_ref().kind {
        PanelKind::Files => false,
        PanelKind::Tree(_) => {
            TreeCommand::from_action(action).is_some_and(|command| popup::tree::apply(app, command))
        }
        PanelKind::QuickView(_) => ViewerScroll::from_action(action)
            .is_some_and(|scroll| popup::quickview::scroll(app, scroll)),
        PanelKind::Sessions(_) => popup::sessions::answer(app, action),
        PanelKind::Remote(_) => popup::remote::answer(app, action),
    }
}

/// A tree, quick-view, sessions or remote panel is asked about a key before
/// the file-panel actions are: its rows are folders, a preview, sessions or
/// a host's entries, not the local entries the actions would act on. `Ctrl+O`'s console has no panels to ask.
fn panel_kind_consumes(app: &mut App, key: KeyEvent) -> bool {
    if app.view != ViewMode::Panels {
        return false;
    }
    match app.get_active_panel_ref().kind {
        PanelKind::Files => false,
        PanelKind::Tree(_) => popup::tree::process(app, key),
        PanelKind::QuickView(_) => popup::quickview::process(app, key),
        PanelKind::Sessions(_) => popup::sessions::process(app, key),
        PanelKind::Remote(_) => popup::remote::process(app, key),
    }
}

#[cfg(test)]
mod tests;
