//! Confirmation dialogs and dismissal of informational overlays.

use crate::models::{App, Overlay};
use crossterm::event::KeyCode;

use crate::ops::handle_delete;

pub(super) fn help(app: &mut App, code: KeyCode) -> bool {
    let page = usize::from(app.viewport.rows().saturating_sub(2)).max(1);
    let max = crate::models::help::HELP_LINES.len().saturating_sub(page);
    let top = app.help_top.min(max);
    match code {
        KeyCode::Esc | KeyCode::F(1 | 10) => app.close_overlay(),
        KeyCode::Up => app.help_top = top.saturating_sub(1),
        KeyCode::Down => app.help_top = top.saturating_add(1).min(max),
        KeyCode::PageUp => app.help_top = top.saturating_sub(page),
        KeyCode::PageDown => app.help_top = top.saturating_add(page).min(max),
        KeyCode::Home => app.help_top = 0,
        KeyCode::End => app.help_top = max,
        _ => {}
    }
    true
}

pub(super) fn information(app: &mut App, code: KeyCode) -> bool {
    if matches!(
        code,
        KeyCode::Esc | KeyCode::Enter | KeyCode::F(10) | KeyCode::Char('l' | 'L')
    ) {
        app.close_overlay();
    }
    true
}

/// A copy, move or delete that stopped: OK closes it; "Retry as
/// administrator", when offered, asks Windows for the rights and goes on.
/// Left, Right and Tab move between the two; `A` retries at once; no other
/// key reaches the panels behind it.
pub(super) fn operation_failed(app: &mut App, code: KeyCode) -> bool {
    let Overlay::OperationFailed(failure) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Left | KeyCode::Right | KeyCode::Tab | KeyCode::BackTab => failure.toggle_focus(),
        KeyCode::Char('a' | 'A') if failure.retry().is_some() => {
            crate::ops::retry_as_administrator(app);
        }
        KeyCode::Enter | KeyCode::Char(' ') if failure.retry_focused() => {
            crate::ops::retry_as_administrator(app);
        }
        KeyCode::Esc | KeyCode::Enter | KeyCode::F(10) | KeyCode::Char(' ') => app.close_overlay(),
        _ => {}
    }
    true
}

/// "File already exists": Overwrite, Skip, Keep both or Cancel, each by its
/// letter or by the button with the cursor; Space ticks "Remember choice"
/// when the cursor is on it, and `E` from anywhere.
pub(super) fn existing_file(app: &mut App, code: KeyCode) -> bool {
    let Overlay::ExistingFile(question) = &mut app.overlay else {
        return false;
    };
    match clash_key(question.focus, code) {
        ClashKey::Move(forward) => question.focus = question.focus.next(forward),
        ClashKey::Remember => question.remember = !question.remember,
        ClashKey::Answer(answer) => crate::ops::answer_existing(app, answer),
        ClashKey::Cancel => crate::ops::cancel_existing(app),
        ClashKey::Ignore => {}
    }
    true
}

/// What a key does in the "File already exists" question.
enum ClashKey {
    Move(bool),
    Remember,
    Answer(crate::models::ClashAnswer),
    Cancel,
    Ignore,
}

const fn clash_key(focus: crate::models::ClashFocus, code: KeyCode) -> ClashKey {
    use crate::models::{ClashAnswer, ClashFocus};

    match code {
        KeyCode::Tab | KeyCode::Right | KeyCode::Down => ClashKey::Move(true),
        KeyCode::BackTab | KeyCode::Left | KeyCode::Up => ClashKey::Move(false),
        KeyCode::Char('e' | 'E') => ClashKey::Remember,
        KeyCode::Char('o' | 'O') => ClashKey::Answer(ClashAnswer::Overwrite),
        KeyCode::Char('s' | 'S') => ClashKey::Answer(ClashAnswer::Skip),
        KeyCode::Char('k' | 'K') => ClashKey::Answer(ClashAnswer::KeepBoth),
        KeyCode::Char('c' | 'C') | KeyCode::Esc | KeyCode::F(10) => ClashKey::Cancel,
        KeyCode::Enter | KeyCode::Char(' ') => match focus {
            ClashFocus::Remember => ClashKey::Remember,
            ClashFocus::Overwrite => ClashKey::Answer(ClashAnswer::Overwrite),
            ClashFocus::Skip => ClashKey::Answer(ClashAnswer::Skip),
            ClashFocus::KeepBoth => ClashKey::Answer(ClashAnswer::KeepBoth),
            ClashFocus::Cancel => ClashKey::Cancel,
        },
        _ => ClashKey::Ignore,
    }
}

pub(super) fn quit(app: &mut App, code: KeyCode) -> bool {
    let Overlay::QuitConfirm(answer) = &mut app.overlay else {
        return false;
    };
    // Two answers, so Left, Right and Tab all mean "the other one".
    match code {
        KeyCode::Left | KeyCode::Right | KeyCode::Tab => *answer = answer.toggled(),
        // F10 asked the question, so F10 answers it. Closing the
        // dialog instead made the key its own undo: F10 opened it,
        // F10 closed it, and the program could not be quit by
        // pressing the quit key — it just alternated forever.
        KeyCode::Char('y' | 'Y') | KeyCode::F(10) => {
            app.lifecycle = crate::models::Lifecycle::Quitting;
        }
        KeyCode::Char('n' | 'N') | KeyCode::Esc => app.close_overlay(),
        KeyCode::Enter => {
            if answer.is_yes() {
                app.lifecycle = crate::models::Lifecycle::Quitting;
            } else {
                app.close_overlay();
            }
        }
        _ => {}
    }
    true
}

pub(super) fn delete(app: &mut App, code: KeyCode) -> bool {
    match code {
        KeyCode::Char('y' | 'Y') | KeyCode::Enter => {
            // Confirmed. `handle_delete` never fails as a whole: it
            // folds per-file errors into its own status summary, so
            // there is nothing left to report here.
            drop(handle_delete(app));
        }
        KeyCode::Char('n' | 'N') | KeyCode::Esc | KeyCode::F(10) => {
            // Cancelled
            app.close_overlay();
            app.set_status(crate::status::Status::DeleteCancelled);
        }
        _ => {}
    }
    true
}
