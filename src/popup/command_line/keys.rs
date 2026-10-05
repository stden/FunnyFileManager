//! Purpose: the command line's editing keymap — history, completion, submit and dismiss, cursor motion, kill and yank, and plain text.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::completion::{accept_candidate, accept_ghost, ghost_acceptable};
use super::history::history_step;
use super::search::start_search;
use super::{
    close_menu, complete_at_cursor, edit_and_refresh, menu_open, move_caret, refresh_ghost, state,
    submit, with_menu, Step,
};
use crate::lineedit::LineEditor;
use crate::models::{App, Menu};

/// The full editing keymap (see the table in the design spec).
pub(super) fn editing_key(app: &mut App, key: KeyEvent) {
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
        // Ctrl+C copies a selection, as in any field; with nothing selected
        // it clears the line rather than quitting: inside a prompt it is
        // bash's "abandon this line", and the overlay already swallows the
        // main-view quit binding.
        KeyCode::Char('c') if ctrl && has_selection(app) => copy_selection(app),
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
    if selection_key(app, key) || clipboard_key(app, key) {
        return;
    }
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

/// Shift with an arrow, Home or End grows the selection, as in every field;
/// Ctrl+Shift moves it by word. Whether `key` was one of them.
fn selection_key(app: &mut App, key: KeyEvent) -> bool {
    if !key.modifiers.contains(KeyModifiers::SHIFT) {
        return false;
    }
    let word = key.modifiers.contains(KeyModifiers::CONTROL);
    let motion: fn(&mut LineEditor) = match key.code {
        KeyCode::Left if word => LineEditor::word_left,
        KeyCode::Right if word => LineEditor::word_right,
        KeyCode::Left => LineEditor::left,
        KeyCode::Right => LineEditor::right,
        KeyCode::Home => LineEditor::home,
        KeyCode::End => LineEditor::end,
        _ => return false,
    };
    move_caret(app, |line| line.extend(motion));
    true
}

/// Ctrl+Insert copies, Ctrl+X and Shift+Del cut, Ctrl+V and Shift+Insert
/// paste. (Ctrl+C is above: it also clears the line.) Whether `key` was one.
fn clipboard_key(app: &mut App, key: KeyEvent) -> bool {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let shift = key.modifiers.contains(KeyModifiers::SHIFT);
    match key.code {
        KeyCode::Insert if ctrl => copy_selection(app),
        KeyCode::Char('x') if ctrl => cut_selection(app),
        KeyCode::Delete if shift => cut_selection(app),
        KeyCode::Char('v') if ctrl => paste(app),
        KeyCode::Insert if shift => paste(app),
        _ => return false,
    }
    true
}

fn has_selection(app: &App) -> bool {
    state(app).is_some_and(|state| state.line.selection().is_some())
}

fn copy_selection(app: &mut App) {
    let Some(text) = state(app).and_then(|state| state.line.selected_text().map(str::to_owned))
    else {
        return;
    };
    crate::popup::text_field::copy(&mut app.clipboard, &text);
}

fn cut_selection(app: &mut App) {
    copy_selection(app);
    edit_and_refresh(app, |line| {
        line.delete_selection();
    });
}

fn paste(app: &mut App) {
    let Some(text) = app
        .clipboard
        .backend_mut()
        .and_then(|backend| backend.get_native().ok())
    else {
        return;
    };
    let text = crate::popup::text_field::one_line(&text);
    edit_and_refresh(app, |line| line.insert_str(&text));
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
