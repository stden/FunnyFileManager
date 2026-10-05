//! Purpose: completion on the command line — Tab's candidate menu, and the ghost suggestion Right or End accepts at the end of the line.

use super::{state, state_mut};
use crate::complete::{self, CompletionContext};
use crate::models::{App, Menu};

/// Complete the token under the caret, preserving everything after it.
pub(in crate::popup) fn complete_at_cursor(app: &mut App) {
    if !app
        .settings
        .enabled(crate::menu::SettingsPage::AutoComplete)
    {
        return;
    }
    let Some(state) = state(app) else { return };
    let head = state.line.before_cursor().to_owned();
    let tail = state.line.after_cursor().to_owned();
    let cwd = app.get_active_panel().path.clone();
    let bookmarks = app.bookmarks.clone();
    let ctx = CompletionContext {
        cwd: &cwd,
        bookmarks: bookmarks.slots(),
    };
    // No completion for this buffer: the buffer is left unchanged, so the
    // ghost already matches it and there is nothing to refresh.
    let Some(comp) = complete::complete_with(&head, &ctx) else {
        return;
    };

    let Some(state) = state_mut(app) else { return };
    state.line.set_split(&comp.line, &tail);
    // A lone candidate is accepted outright; the menu is for choosing between
    // genuine alternatives.
    state.menu = if comp.candidates.len() > 1 {
        Menu::new(comp.candidates)
    } else {
        None
    };
    refresh_ghost(app);
}

/// Splice the highlighted dropdown row into the line and close the dropdown.
pub(super) fn accept_candidate(app: &mut App) {
    if let Some(state) = state_mut(app) {
        if let Some(menu) = state.menu.take() {
            // `selection` is total — a `Menu` cannot be empty — so there is no
            // "the highlight pointed nowhere" branch left to get wrong.
            let candidate = menu.selection();
            // Replace the token being completed, finished exactly as Tab
            // finishes a lone match. `split_target` (not the last-whitespace
            // split) so a `cd` path containing spaces is replaced as the single
            // token the completer saw.
            let head = complete::split_target(state.line.before_cursor())
                .0
                .to_owned();
            let text = complete::finish_word(&candidate.text, candidate.kind);
            let tail = state.line.after_cursor().to_owned();
            state.line.set_split(&format!("{head}{text}"), &tail);
        }
    }
    // Accepting a candidate is a buffer edit like any other — the menu that
    // suppressed the ghost is gone now, so a history match may apply again.
    refresh_ghost(app);
}

/// Recompute the open completion dropdown against the edited buffer.
///
/// Called after a printable key or a deletion **only while a menu is open**: a
/// frozen list would let `Enter` splice a candidate that no longer matches what
/// was typed. Only `candidates` is used — never `comp.line` — so typing never
/// rewrites the buffer under the user's fingers; the longest-common-prefix
/// advance stays a `Tab` gesture.
///
/// Tradeoff: `complete_with` runs once per keystroke, and for paths it may
/// shell out to fish. That is acceptable because a menu is a transient state
/// the user opened deliberately — plain typing (menu closed) never calls it.
pub(in crate::popup) fn refresh_menu(app: &mut App) {
    let Some(state) = state(app) else { return };
    let head = state.line.before_cursor().to_owned();
    let cwd = app.get_active_panel().path.clone();
    let bookmarks = app.bookmarks.clone();
    let ctx = CompletionContext {
        cwd: &cwd,
        bookmarks: bookmarks.slots(),
    };
    // No completion for the new buffer means nothing left to choose from.
    let menu = complete::complete_with(&head, &ctx).and_then(|c| Menu::new(c.candidates));
    if let Some(state) = state_mut(app) {
        state.menu = menu;
    }
}

/// Whether `Right`/`End` should accept the suggestion instead of moving.
pub(super) fn ghost_acceptable(app: &App) -> bool {
    state(app).is_some_and(|s| s.ghost.is_some() && s.line.at_end())
}

pub(super) fn accept_ghost(app: &mut App) {
    if let Some(state) = state_mut(app) {
        if let Some(tail) = state.ghost.take() {
            state.line.insert_str(&tail);
        }
    }
    refresh_ghost(app);
}

/// Recompute the ghost suggestion for the current buffer.
///
/// Cleared while the menu is open (its accept key would fight the menu's
/// selection for the same intent) and whenever the caret is not at the end of
/// the line, since a suggestion appended after mid-line text would describe a
/// line the user is not typing. Literal-prefix only, via
/// [`crate::history::History::suggest`].
pub(in crate::popup) fn refresh_ghost(app: &mut App) {
    if !app
        .settings
        .enabled(crate::menu::SettingsPage::AutoComplete)
    {
        return;
    }
    let Some(state) = state(app) else { return };
    if state.menu.is_some() || !state.line.at_end() {
        if let Some(state) = state_mut(app) {
            state.ghost = None;
        }
        return;
    }
    let buffer = state.line.text().to_owned();
    let ghost = app
        .history
        .suggest(&buffer)
        .and_then(|entry| entry.strip_prefix(buffer.as_str()))
        .map(str::to_owned);
    if let Some(state) = state_mut(app) {
        state.ghost = ghost;
    }
}
