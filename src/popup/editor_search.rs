//! Purpose: the editor's F7 search dialog, which owns the keyboard while open, and Shift+F7's repeat of the last search.

use crate::models::App;
use crate::search::{SearchField, SearchPrompt};
use crossterm::event::{KeyCode, KeyModifiers};

/// Keys belonging to the F7 dialog, which owns the keyboard while it is open.
///
/// Tab and the arrows walk the controls, Space flips whichever checkbox the
/// cursor is on, and Enter runs the search from wherever the cursor is — so a
/// user who only ever types a word and presses Enter never meets the rest.
pub(super) fn process_search_prompt(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Some(editor) = app.get_editor_mut() else {
        return false;
    };
    let Some(prompt) = editor.search.prompt.as_mut() else {
        return false;
    };
    // Crossterm reports Ctrl+A as `Char('a')` with CONTROL: a chord is a
    // command, never text, as in every other prompt.
    let typed = !modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);

    match code {
        KeyCode::Esc | KeyCode::F(10) => editor.search.prompt = None,
        KeyCode::Tab | KeyCode::Down => prompt.field = prompt.field.next(),
        KeyCode::BackTab | KeyCode::Up => prompt.field = prompt.field.previous(),
        KeyCode::Char(' ') if prompt.field.is_toggle() => prompt.toggle(),
        KeyCode::Backspace if prompt.field == SearchField::Query => {
            prompt.query.pop();
        }
        KeyCode::Char(ch) if typed && prompt.field == SearchField::Query => prompt.query.push(ch),
        KeyCode::Enter => submit_search(app),
        _ => {}
    }
    true
}

fn submit_search(app: &mut App) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    let Some(mut prompt) = editor.search.prompt.take() else {
        return;
    };
    if prompt.field == SearchField::Cancel {
        return;
    }
    if let Some(direction) = prompt.field.direction() {
        prompt.direction = direction;
    }
    if let Some(search) = prompt.to_search() {
        run_search(app, search);
    }
}

/// Shift+F7: run the last search again from where the caret is now.
pub(super) fn repeat_search(app: &mut App) {
    let Some(search) = app
        .get_editor()
        .and_then(|editor| editor.search.last.clone())
    else {
        // Nothing to repeat, so Shift+F7 asks what to look for — which is what
        // the user meant, and saves a trip through F7.
        if let Some(editor) = app.get_editor_mut() {
            editor.search.prompt = Some(SearchPrompt::default());
        }
        return;
    };
    run_search(app, search);
}

/// Move the caret to the next match, or record that there was none.
pub(super) fn run_search(app: &mut App, search: crate::search::Search) {
    let Some(editor) = app.get_editor_mut() else {
        return;
    };
    let found = editor.find(&search);
    editor.search.not_found = (!found).then(|| search.needle.as_str().to_owned());
    // Remembered either way: repeating a search that failed is how the user
    // checks after editing the file.
    editor.search.last = Some(search);
}
