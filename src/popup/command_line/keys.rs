//! Purpose: the command line's editing commands applied — history, completion, submit and dismiss, caret moves, selection, the clipboard, kill and yank, and plain text.

use crossterm::event::KeyEvent;

use super::completion::{accept_candidate, accept_ghost, ghost_acceptable};
use super::history::history_step;
use super::search::start_search;
use super::{
    close_menu, complete_at_cursor, edit_and_refresh, menu_open, move_caret, refresh_ghost, state,
    submit, with_menu, Step,
};
use crate::action::command_line::completion::{self, CompletionCommand};
use crate::action::command_line::{resolve, LineClipboard, LineCommand, LineEdit, LineMotion};
use crate::lineedit::LineEditor;
use crate::models::{App, Menu, Recall};

/// The keys of the line being edited, as the keymap binds them. An open
/// completion list is asked first; a key it does not use edits the line.
pub(super) fn editing_key(app: &mut App, key: KeyEvent) {
    if menu_open(app) {
        if let Some(command) = completion::resolve(&app.keymap.command_line_completion, key) {
            apply_completion(app, command);
            return;
        }
    }
    if let Some(command) = resolve(&app.keymap.command_line, key) {
        apply(app, command);
    } else if crate::action::resolve(&app.keymap.panels, key)
        == crate::action::Action::ToggleConsole
    {
        app.view = app.view.toggled();
    }
}

pub fn apply(app: &mut App, command: LineCommand) {
    if state(app).is_none_or(|line| matches!(line.recall, Recall::Searching { .. })) {
        return;
    }
    match command {
        LineCommand::HistoryBack => history_step(app, Step::Older),
        LineCommand::HistoryForward => history_step(app, Step::Newer),
        LineCommand::HistorySearch => start_search(app),
        LineCommand::Complete => complete_at_cursor(app),
        LineCommand::Submit => submit(app),
        LineCommand::Cancel => {
            app.close_overlay();
            app.quick_search.clear();
        }
        // Ctrl+C copies a selection, as in any field; with nothing selected it
        // clears the line rather than quitting: inside a prompt it is bash's
        // "abandon this line", and the overlay already swallows the main-view
        // quit binding.
        LineCommand::CopyOrClear if has_selection(app) => copy_selection(app),
        LineCommand::CopyOrClear => edit_and_refresh(app, LineEditor::clear),
        LineCommand::DeleteCharOrClose => delete_char_or_close(app),
        // A move to the right has nowhere to go from the end of the line,
        // the state the prompt is in until the caret is moved: there it
        // takes the suggestion, and mid-line it is an ordinary caret move.
        LineCommand::Move(motion) if goes_right(motion) && ghost_acceptable(app) => {
            accept_ghost(app);
        }
        LineCommand::Move(motion) => move_caret(app, |line| apply_motion(line, motion)),
        LineCommand::Select(motion) => move_caret(app, |line| line.extend(motion_of(motion))),
        LineCommand::Clipboard(operation) => clipboard(app, operation),
        LineCommand::Edit(edit) => edit_and_refresh(app, |line| apply_edit(line, edit)),
    }
}

/// Ctrl+D on an empty line closes the prompt (bash's EOF); with text to its
/// right it is a forward delete, like every other readline.
fn delete_char_or_close(app: &mut App) {
    if state(app).is_some_and(|s| s.line.is_empty()) {
        app.close_overlay();
    } else {
        edit_and_refresh(app, LineEditor::delete_forward);
    }
}

/// The three moves towards the end of the line, whatever keys run them.
const fn goes_right(motion: LineMotion) -> bool {
    matches!(
        motion,
        LineMotion::CharRight | LineMotion::WordRight | LineMotion::LineEnd
    )
}

fn apply_motion(line: &mut LineEditor, motion: LineMotion) {
    motion_of(motion)(line);
}

/// The caret move a motion names, as a function of the line.
fn motion_of(motion: LineMotion) -> fn(&mut LineEditor) {
    match motion {
        LineMotion::CharLeft => LineEditor::left,
        LineMotion::CharRight => LineEditor::right,
        LineMotion::WordLeft => LineEditor::word_left,
        LineMotion::WordRight => LineEditor::word_right,
        LineMotion::LineStart => LineEditor::home,
        LineMotion::LineEnd => LineEditor::end,
    }
}

fn apply_edit(line: &mut LineEditor, edit: LineEdit) {
    match edit {
        LineEdit::KillWordLeft => line.kill_word_left(),
        LineEdit::KillToStart => line.kill_to_start(),
        LineEdit::KillToEnd => line.kill_to_end(),
        LineEdit::Yank => line.yank(),
        LineEdit::Backspace => line.backspace(),
        LineEdit::DeleteChar => line.delete_forward(),
        LineEdit::Insert(ch) => line.insert_char(ch),
    }
}

fn clipboard(app: &mut App, operation: LineClipboard) {
    match operation {
        LineClipboard::Copy => copy_selection(app),
        LineClipboard::Cut => cut_selection(app),
        LineClipboard::Paste => paste(app),
    }
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

pub fn apply_completion(app: &mut App, command: CompletionCommand) {
    if !state(app)
        .is_some_and(|line| line.menu.is_some() && !matches!(line.recall, Recall::Searching { .. }))
    {
        return;
    }
    match command {
        CompletionCommand::Next => with_menu(app, Menu::next),
        CompletionCommand::Previous => with_menu(app, Menu::previous),
        // A buffer edit like any other: it refreshes the ghost itself.
        CompletionCommand::Accept => accept_candidate(app),
        // The ghost is withdrawn while a list is open; closing the list is
        // what lets a history match apply again.
        CompletionCommand::Close => {
            close_menu(app);
            refresh_ghost(app);
        }
    }
}
