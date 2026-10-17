//! Purpose: the dialogs the editor puts up (encoding, search, not found, exit)
//! and where their rows and buttons lead.

use super::*;

#[test]
fn encoding_dialog_rows_and_buttons_route_to_encoding_actions() {
    use crate::encoding::TextEncoding;
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let mut editor = Editor::from_content("text".to_owned(), dir.path().join("text"), "hello");
    editor.encoding_choice = Some(TextEncoding::Utf8);
    app.set_overlay(Overlay::Editor(editor));
    click_dialog_row(&mut app, 4);
    let editor = app.get_editor_mut().expect("editor");
    assert_eq!(editor.encoding_choice, Some(TextEncoding::Windows1251));
    for (index, code) in [
        (0, KeyCode::Enter),
        (1, KeyCode::Char('r')),
        (2, KeyCode::Esc),
    ] {
        assert_eq!(
            dialog_button_key_in_editor(editor, index).map(|key| key.code),
            Some(code)
        );
    }
}

#[test]
fn editor_search_buttons_select_the_named_direction() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let editor = Editor {
        search: crate::search::SearchState {
            prompt: Some(SearchPrompt::default()),
            ..Default::default()
        },
        ..Default::default()
    };
    app.set_overlay(Overlay::Editor(editor));
    assert_eq!(code(&mut app, 0), KeyCode::Enter);
    let Overlay::Editor(editor) = &app.overlay else {
        panic!("editor");
    };
    assert_eq!(
        editor.search.prompt.as_ref().expect("prompt").field,
        SearchField::FindPrevious
    );
    assert_eq!(code(&mut app, 1), KeyCode::Enter);
    let Overlay::Editor(editor) = &app.overlay else {
        panic!("editor");
    };
    assert_eq!(
        editor.search.prompt.as_ref().expect("prompt").field,
        SearchField::FindNext
    );
    assert_eq!(code(&mut app, 2), KeyCode::Esc);
}

fn prompt(app: &App) -> &SearchPrompt {
    app.get_editor()
        .expect("editor")
        .search
        .prompt
        .as_ref()
        .expect("prompt")
}

#[test]
fn editor_search_rows_focus_query_and_toggle_checkboxes() {
    use crate::search::{Case, WordMatch};
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    app.set_overlay(Overlay::Editor(Editor {
        search: crate::search::SearchState {
            prompt: Some(SearchPrompt::default()),
            ..Default::default()
        },
        ..Default::default()
    }));

    click_dialog_row(&mut app, 3);
    assert_eq!(
        (prompt(&app).field, prompt(&app).case),
        (SearchField::Case, Case::Match)
    );

    click_dialog_row(&mut app, 4);
    assert_eq!(
        (prompt(&app).field, prompt(&app).words),
        (SearchField::Words, WordMatch::WholeWords)
    );

    click_dialog_row(&mut app, 1);
    assert_eq!(prompt(&app).field, SearchField::Query);
}

#[test]
fn editor_not_found_and_exit_dialogs_map_their_buttons() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let editor = Editor {
        search: crate::search::SearchState {
            not_found: Some("needle".into()),
            ..Default::default()
        },
        ..Default::default()
    };
    app.set_overlay(Overlay::Editor(editor));
    assert_eq!(code(&mut app, 0), KeyCode::Esc);

    let editor = Editor {
        state: EditState::ConfirmingExit(ExitChoice::Save),
        ..Default::default()
    };
    app.set_overlay(Overlay::Editor(editor));
    assert_eq!(code(&mut app, 0), KeyCode::Char('y'));
    assert_eq!(code(&mut app, 1), KeyCode::Char('n'));
    assert_eq!(code(&mut app, 2), KeyCode::Esc);
}

#[test]
fn the_changed_on_disk_question_maps_overwrite_and_cancel() {
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let editor = Editor {
        state: EditState::ConfirmingOverwrite(crate::models::OverwritePrompt::new(
            crate::models::AfterSave::Stay,
        )),
        ..Default::default()
    };
    app.set_overlay(Overlay::Editor(editor));
    assert_eq!(code(&mut app, 0), KeyCode::Char('y'));
    assert_eq!(code(&mut app, 1), KeyCode::Esc);
}
