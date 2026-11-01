//! Purpose: Readline-grade key handling for the `:` command-line overlay.
//!
//! Split out of `popup.rs` because the command line is the one overlay with a
//! keymap of its own size — cursor motion, kill/yank, history recall, reverse
//! search and completion, each with a mode in which it means something
//! different. Keeping it beside the other overlays' single-digit match arms
//! would bury all of them.
//!
//! Every handler here follows the same borrow discipline as the rest of
//! `popup`: read what is needed out of `app.overlay`, end the borrow, then act
//! on the whole `App`.

use crossterm::event::KeyEvent;

use crate::lineedit::LineEditor;
use crate::models::{App, CommandLine, Menu, Overlay, Recall};
use crate::ops::execute_command;
use crate::status::Status;

mod completion;
mod history;
mod keys;
mod search;

pub(super) use completion::{complete_at_cursor, refresh_ghost, refresh_menu};
use keys::editing_key;
pub use keys::{apply, apply_completion};
pub use search::apply as apply_search;
use search::search_key;

/// Direction of a history walk.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Step {
    /// `Up` — further back in time.
    Older,
    /// `Down` — back towards what was being typed.
    Newer,
}

/// Handle one key for the open command-line overlay.
///
/// Always returns `true`: the prompt is modal like every other overlay, so keys
/// it does not use are swallowed rather than passed to the panel handler.
/// Falling through would let `Left`/`Right` move the active panel out from
/// under the command (which takes its cwd at `Enter`) and let `F5`/`F8`/`F10`
/// act on the panels behind the prompt while the typed text was discarded.
pub(super) fn process(app: &mut App, key: KeyEvent) -> bool {
    // A reverse search owns the keyboard: while it is running, every key means
    // something to the search, so there is no fall-through to normal editing.
    if matches!(
        state(app).map(|s| &s.recall),
        Some(Recall::Searching { .. })
    ) {
        return search_key(app, key);
    }
    editing_key(app, key);
    true
}

pub(super) fn paste(app: &mut App, text: &str) {
    let line = crate::popup::text_field::one_line(text);
    if matches!(
        state(app).map(|s| &s.recall),
        Some(Recall::Searching { .. })
    ) {
        search::edit_query(app, |query| query.push_str(&line));
        search::search_from(app, crate::history::RingIndex::NEWEST);
    } else {
        edit_and_refresh(app, |editor| editor.insert_str(&line));
    }
}

// ─── borrow helpers ───────────────────────────────────────────────────────

/// The command-line state, when that overlay is the open one.
fn state(app: &App) -> Option<&CommandLine> {
    match &app.overlay {
        Overlay::CommandLine(state) => Some(state),
        _ => None,
    }
}

fn state_mut(app: &mut App) -> Option<&mut CommandLine> {
    match &mut app.overlay {
        Overlay::CommandLine(state) => Some(state),
        _ => None,
    }
}

fn menu_open(app: &App) -> bool {
    state(app).is_some_and(|s| s.menu.is_some())
}

fn close_menu(app: &mut App) {
    if let Some(state) = state_mut(app) {
        state.menu = None;
    }
}

fn with_menu(app: &mut App, step: impl FnOnce(&mut Menu)) {
    if let Some(menu) = state_mut(app).and_then(|s| s.menu.as_mut()) {
        step(menu);
    }
}

/// Apply a pure [`LineEditor`] operation to the buffer.
fn edit(app: &mut App, operation: impl FnOnce(&mut LineEditor)) {
    if let Some(state) = state_mut(app) {
        operation(&mut state.line);
    }
}

/// Move the caret, close any open completion menu, then re-derive the ghost.
///
/// Moving the caret changes the token before the cursor that completion ranks
/// and splices against, so an open dropdown must be dismissed before `Enter`
/// could splice a stale candidate into the new caret position.
fn move_caret(app: &mut App, motion: impl FnOnce(&mut LineEditor)) {
    edit(app, motion);
    close_menu(app);
    refresh_ghost(app);
}

/// Apply an edit, then bring the dependent state back in step with it.
fn edit_and_refresh(app: &mut App, operation: impl FnOnce(&mut LineEditor)) {
    edit(app, operation);
    refresh_after_edit(app);
}

/// Recompute what an edited buffer implies: the dropdown (only if one is open)
/// and the ghost. History browsing deliberately survives an edit — bash keeps
/// its place in the ring when you amend a recalled command, so `Up` continues
/// from there rather than restarting at the newest entry.
fn refresh_after_edit(app: &mut App) {
    if menu_open(app) {
        refresh_menu(app);
    }
    refresh_ghost(app);
}

// ─── execution ────────────────────────────────────────────────────────────

/// Close the prompt and run what was typed.
fn submit(app: &mut App) {
    let command = state(app)
        .map(|s| s.line.text().trim().to_owned())
        .unwrap_or_default();
    app.close_overlay();
    if command.is_empty() {
        return;
    }
    app.history.record(&command);
    if let Err(err) = execute_command(app, &command) {
        app.set_status(Status::of_error(&err));
    }
}

#[cfg(test)]
mod tests;
