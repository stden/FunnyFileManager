//! Purpose: Up and Down walk the command history, stashing the line being typed and putting it back past the newest entry.

use super::{refresh_ghost, state, state_mut, Step};
use crate::history::RingIndex;
use crate::models::{App, Recall};

/// Walk the history ring one step and show that entry in the buffer.
///
/// The line being typed when the walk started is stashed and restored on
/// walking forward past the newest entry, so `Up` is never destructive.
pub(super) fn history_step(app: &mut App, step: Step) {
    let Some(state) = state(app) else { return };

    // The stash is captured on the *first* Up: after that it is carried along,
    // because each recalled entry has already overwritten the buffer. A `None`
    // index means "walked forward past the newest entry" — put the stash back.
    let (index, stash) = match (&state.recall, step) {
        (Recall::Idle, Step::Older) => (Some(RingIndex::NEWEST), state.line.text().to_owned()),
        (Recall::Browsing { index, stash }, Step::Older) => (Some(index.older()), stash.clone()),
        // `newer()` is `None` at the newest entry, which is exactly the
        // "walked forward past the start" case that restores the stash.
        (Recall::Browsing { index, stash }, Step::Newer) => (index.newer(), stash.clone()),
        // `Idle` + `Newer`: nothing was recalled, so there is nothing to walk
        // forward to. `Searching` never reaches here — `process` routes a
        // running search to `search_key` before this function is called.
        _ => return,
    };

    let Some(index) = index else {
        restore(app, &stash);
        return;
    };

    // Clone the entry: `history` borrows `app` immutably, and the buffer write
    // below needs it mutably.
    let Some(entry) = app.history.recall(index).map(str::to_owned) else {
        // Past the oldest entry — stay where we are rather than clearing the
        // line, which is what makes holding Up harmless.
        return;
    };
    if let Some(state) = state_mut(app) {
        state.line.set_text(&entry);
        state.menu = None;
        state.recall = Recall::Browsing { index, stash };
    }
    refresh_ghost(app);
}

/// Put the stashed line back and leave history mode.
pub(super) fn restore(app: &mut App, stash: &str) {
    if let Some(state) = state_mut(app) {
        state.line.set_text(stash);
        state.recall = Recall::Idle;
    }
    refresh_ghost(app);
}
