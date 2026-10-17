//! Purpose: Ctrl+R's reverse search through the command history, which owns the keyboard while it runs.

use crossterm::event::KeyEvent;

use super::{refresh_ghost, state, state_mut};
use crate::action::command_line::search::{resolve, LineSearchCommand};
use crate::history::RingIndex;
use crate::models::{App, Recall};

/// Enter reverse-search mode, stashing the line so `Esc` can put it back.
pub(super) fn start_search(app: &mut App) {
    let Some(state) = state_mut(app) else { return };
    let stash = state.line.text().to_owned();
    state.menu = None;
    state.ghost = None;
    state.recall = Recall::Searching {
        query: String::new(),
        hit: None,
        stash,
    };
}

/// Keys while a reverse search is running: every one is consumed, because
/// the search owns the keyboard.
pub(super) fn search_key(app: &mut App, key: KeyEvent) -> bool {
    if let Some(command) = resolve(&app.keymap.command_line_search, key) {
        apply(app, command);
    }
    true
}

fn apply(app: &mut App, command: LineSearchCommand) {
    match command {
        // Another Ctrl+R steps to the next older match.
        LineSearchCommand::SearchOlder => {
            let from = next_search_start(app);
            search_from(app, from);
        }
        LineSearchCommand::Cancel => cancel_search(app),
        LineSearchCommand::Accept => finish_search(app),
        LineSearchCommand::Backspace => {
            edit_query(app, |query| {
                query.pop();
            });
            search_from(app, RingIndex::NEWEST);
        }
        LineSearchCommand::Extend(ch) => {
            edit_query(app, |query| query.push(ch));
            search_from(app, RingIndex::NEWEST);
        }
    }
}

/// Where the next `Ctrl+R` should resume: one past the entry on screen.
pub(super) fn next_search_start(app: &App) -> RingIndex {
    match state(app).map(|s| &s.recall) {
        Some(Recall::Searching { hit: Some(hit), .. }) => hit.older(),
        _ => RingIndex::NEWEST,
    }
}

pub(super) fn edit_query(app: &mut App, change: impl FnOnce(&mut String)) {
    if let Some(Recall::Searching { query, .. }) = state_mut(app).map(|s| &mut s.recall) {
        change(query);
    }
}

/// Look for the query at or after `from` and show the match in the buffer.
///
/// A query that matches nothing leaves the buffer on the previous match rather
/// than blanking it — the same thing bash does, minus the beep.
pub(super) fn search_from(app: &mut App, from: RingIndex) {
    let Some(Recall::Searching { query, .. }) = state(app).map(|s| &s.recall) else {
        return;
    };
    let query = query.clone();
    let found = app.history.search_back(&query, from).and_then(|index| {
        app.history
            .recall(index)
            .map(|line| (index, line.to_owned()))
    });

    let Some(state) = state_mut(app) else { return };
    let Some((index, line)) = found else {
        if let Recall::Searching { hit, .. } = &mut state.recall {
            *hit = None;
        }
        return;
    };
    state.line.set_text(&line);
    if let Recall::Searching { hit, .. } = &mut state.recall {
        *hit = Some(index);
    }
}

/// Accept whatever the search is showing and return to normal editing.
///
/// Deliberately does *not* execute, unlike bash's `Enter`: the accepted line
/// lands in the prompt so it can be read and amended before it runs, which
/// matters more here because a `Ctrl+R` hit may be a session-only shell command
/// the user has not seen since typing it.
pub(super) fn finish_search(app: &mut App) {
    if let Some(state) = state_mut(app) {
        state.recall = Recall::Idle;
    }
    refresh_ghost(app);
}

/// Abandon the search and put the pre-search line back.
pub(super) fn cancel_search(app: &mut App) {
    let Some(state) = state_mut(app) else { return };
    let Recall::Searching { stash, .. } = &mut state.recall else {
        return;
    };
    let stash = std::mem::take(stash);
    state.line.set_text(&stash);
    state.recall = Recall::Idle;
    refresh_ghost(app);
}
