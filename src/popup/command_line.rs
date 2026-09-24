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

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::complete::{self, CompletionContext};
use crate::history::RingIndex;
use crate::lineedit::LineEditor;
use crate::models::{App, CommandLine, Menu, Overlay, Recall};
use crate::ops::execute_command;
use crate::status::Status;

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

/// The full editing keymap (see the table in the design spec).
fn editing_key(app: &mut App, key: KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    if menu_open(app) && completion_menu_key(app, key.code) {
        return;
    }

    match key.code {
        // ─── history ──────────────────────────────────────────────────────
        KeyCode::Up => history_step(app, Step::Older),
        KeyCode::Down => history_step(app, Step::Newer),
        KeyCode::Char('r') if ctrl => start_search(app),

        // ─── completion ───────────────────────────────────────────────────
        KeyCode::Tab => complete_at_cursor(app),

        // ─── submit and dismiss ───────────────────────────────────────────
        KeyCode::Enter => submit(app),
        KeyCode::Esc => {
            app.close_overlay();
            app.quick_search.clear();
        }
        // Ctrl+C clears the line rather than quitting: inside a prompt it is
        // bash's "abandon this line", and the overlay already swallows the
        // main-view quit binding.
        KeyCode::Char('c') if ctrl => edit_and_refresh(app, LineEditor::clear),
        // Ctrl+D on an empty line closes the prompt (bash's EOF); with text to
        // its right it is a forward delete, like every other readline.
        KeyCode::Char('d') if ctrl => {
            if state(app).is_some_and(|s| s.line.is_empty()) {
                app.close_overlay();
            } else {
                edit_and_refresh(app, LineEditor::delete_forward);
            }
        }

        _ => edit_text_key(app, key),
    }
}

fn edit_text_key(app: &mut App, key: KeyEvent) {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);
    match key.code {
        // ─── ghost acceptance, then cursor motion ─────────────────────────
        // Right/End accept the suggestion only with the caret already at the
        // end of the line — the state the prompt is always in until the user
        // moves the caret. So the existing gesture survives, and mid-line
        // these keys are ordinary cursor moves.
        KeyCode::Right | KeyCode::End if ghost_acceptable(app) => accept_ghost(app),
        KeyCode::Left if ctrl || alt => move_caret(app, LineEditor::word_left),
        KeyCode::Right if ctrl || alt => move_caret(app, LineEditor::word_right),
        KeyCode::Char('b') if alt => move_caret(app, LineEditor::word_left),
        KeyCode::Char('f') if alt => move_caret(app, LineEditor::word_right),
        KeyCode::Left => move_caret(app, LineEditor::left),
        KeyCode::Right => move_caret(app, LineEditor::right),
        KeyCode::Home => move_caret(app, LineEditor::home),
        KeyCode::End => move_caret(app, LineEditor::end),
        KeyCode::Char('a') if ctrl => move_caret(app, LineEditor::home),
        KeyCode::Char('e') if ctrl => move_caret(app, LineEditor::end),

        // ─── kill and yank ────────────────────────────────────────────────
        KeyCode::Char('w') if ctrl => edit_and_refresh(app, LineEditor::kill_word_left),
        KeyCode::Char('u') if ctrl => edit_and_refresh(app, LineEditor::kill_to_start),
        KeyCode::Char('k') if ctrl => edit_and_refresh(app, LineEditor::kill_to_end),
        KeyCode::Char('y') if ctrl => edit_and_refresh(app, LineEditor::yank),

        // ─── plain text ───────────────────────────────────────────────────
        KeyCode::Backspace => edit_and_refresh(app, LineEditor::backspace),
        KeyCode::Delete => edit_and_refresh(app, LineEditor::delete_forward),
        KeyCode::Char(ch) if !ctrl && !alt => {
            edit_and_refresh(app, |line| line.insert_char(ch));
        }
        _ => {}
    }
}

fn completion_menu_key(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Down => with_menu(app, Menu::next),
        KeyCode::Up => with_menu(app, Menu::previous),
        KeyCode::Tab => {
            with_menu(app, Menu::next);
            refresh_ghost(app);
        }
        KeyCode::Enter => accept_candidate(app),
        KeyCode::Esc => {
            close_menu(app);
            refresh_ghost(app);
        }
        _ => return false,
    }
    true
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

/// Move the caret, then re-derive the ghost.
///
/// A caret move is not an edit, but it still invalidates the suggestion: the
/// ghost is only offered with the caret at the end of the line, so stepping
/// left has to withdraw it. The menu is left alone — the text it was ranked
/// against has not changed.
fn move_caret(app: &mut App, motion: impl FnOnce(&mut LineEditor)) {
    edit(app, motion);
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

// ─── history ──────────────────────────────────────────────────────────────

/// Walk the history ring one step and show that entry in the buffer.
///
/// The line being typed when the walk started is stashed and restored on
/// walking forward past the newest entry, so `Up` is never destructive.
fn history_step(app: &mut App, step: Step) {
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
fn restore(app: &mut App, stash: &str) {
    if let Some(state) = state_mut(app) {
        state.line.set_text(stash);
        state.recall = Recall::Idle;
    }
    refresh_ghost(app);
}

// ─── reverse search (Ctrl+R) ──────────────────────────────────────────────

/// Enter reverse-search mode, stashing the line so `Esc` can put it back.
fn start_search(app: &mut App) {
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

/// Keys while a reverse search is running.
fn search_key(app: &mut App, key: KeyEvent) -> bool {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let alt = key.modifiers.contains(KeyModifiers::ALT);

    match key.code {
        // Another Ctrl+R steps to the next older match.
        KeyCode::Char('r') if ctrl => {
            let from = next_search_start(app);
            search_from(app, from);
        }
        // Ctrl+G is bash's abort; Esc and Ctrl+C get the same treatment.
        KeyCode::Char('g' | 'c') if ctrl => cancel_search(app),
        KeyCode::Esc => cancel_search(app),
        KeyCode::Enter => finish_search(app),
        KeyCode::Backspace => {
            edit_query(app, |query| {
                query.pop();
            });
            search_from(app, RingIndex::NEWEST);
        }
        KeyCode::Char(ch) if !ctrl && !alt => {
            edit_query(app, |query| query.push(ch));
            search_from(app, RingIndex::NEWEST);
        }
        _ => {}
    }
    true
}

/// Where the next `Ctrl+R` should resume: one past the entry on screen.
fn next_search_start(app: &App) -> RingIndex {
    match state(app).map(|s| &s.recall) {
        Some(Recall::Searching { hit: Some(hit), .. }) => hit.older(),
        _ => RingIndex::NEWEST,
    }
}

fn edit_query(app: &mut App, change: impl FnOnce(&mut String)) {
    if let Some(Recall::Searching { query, .. }) = state_mut(app).map(|s| &mut s.recall) {
        change(query);
    }
}

/// Look for the query at or after `from` and show the match in the buffer.
///
/// A query that matches nothing leaves the buffer on the previous match rather
/// than blanking it — the same thing bash does, minus the beep.
fn search_from(app: &mut App, from: RingIndex) {
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
fn finish_search(app: &mut App) {
    if let Some(state) = state_mut(app) {
        state.recall = Recall::Idle;
    }
    refresh_ghost(app);
}

/// Abandon the search and put the pre-search line back.
fn cancel_search(app: &mut App) {
    let Some(state) = state_mut(app) else { return };
    let Recall::Searching { stash, .. } = &mut state.recall else {
        return;
    };
    let stash = std::mem::take(stash);
    state.line.set_text(&stash);
    state.recall = Recall::Idle;
    refresh_ghost(app);
}

// ─── completion ───────────────────────────────────────────────────────────

/// Complete the token under the caret, preserving everything after it.
pub(super) fn complete_at_cursor(app: &mut App) {
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
fn accept_candidate(app: &mut App) {
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
pub(super) fn refresh_menu(app: &mut App) {
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

// ─── ghost text ───────────────────────────────────────────────────────────

/// Whether `Right`/`End` should accept the suggestion instead of moving.
fn ghost_acceptable(app: &App) -> bool {
    state(app).is_some_and(|s| s.ghost.is_some() && s.line.at_end())
}

fn accept_ghost(app: &mut App) {
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
pub(super) fn refresh_ghost(app: &mut App) {
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
        app.set_status(Status::Error(err.to_string()));
    }
}

#[cfg(test)]
mod tests;
