//! Unit tests: where the editor's exit and overwrite questions are drawn —
//! none without a question, over the body, centred clear of the edges.

use super::editor_exit::{
    app_on_empty_dirs, body_row_source_index, cells, dialog_area, draw, editor_with_numbered_body,
    row_text, screen_text, SCREEN_W,
};
use super::*;
use crate::models::EditState;
use crate::models::ExitChoice;

#[test]
fn test_editor_without_confirm_exit_draws_no_dialog() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let editor = editor_with_numbered_body("notes.txt");
    assert!(!editor.state.is_confirming_exit(), "precondition");
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let screen = screen_text(&terminal);

    for absent in [
        "Unsaved Changes",
        "Save changes to",
        "[Y]es",
        "[N]o",
        "[Esc]",
    ] {
        assert!(
            !screen.contains(absent),
            "'{absent}' must not be drawn without confirm_exit, got:\n{screen}"
        );
    }
    // The editor itself is on screen, so the check above is about the dialog
    // and not about a frame that failed to render anything at all.
    assert!(
        screen.contains("notes.txt") && screen.contains("Ln 1/40"),
        "editor should still be open, got:\n{screen}"
    );
}

#[test]
fn test_confirm_exit_dialog_is_painted_over_the_editor_body() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let mut editor = editor_with_numbered_body("notes.txt");
    editor.state = EditState::ConfirmingExit(ExitChoice::default());
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let dialog = dialog_area(&terminal);

    // A row the dialog covers must no longer show the editor line underneath.
    let covered = body_row_source_index(dialog.y + 1);
    let covered_marker = format!("BODY{covered:02}");
    assert!(
        !row_text(&terminal, dialog.y + 1).contains(&covered_marker),
        "{covered_marker} must be hidden by the dialog, got:\n{}",
        row_text(&terminal, dialog.y + 1)
    );

    // …while the editor keeps rendering above the dialog: the dialog is drawn
    // on top of the editor, not instead of it.
    let visible = body_row_source_index(dialog.y - 1);
    let visible_marker = format!("BODY{visible:02}");
    assert!(
        row_text(&terminal, dialog.y - 1).contains(&visible_marker),
        "{visible_marker} must stay visible above the dialog, got:\n{}",
        row_text(&terminal, dialog.y - 1)
    );
}

#[test]
fn test_confirm_exit_dialog_is_centered_clear_of_the_screen_edge() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let mut editor = editor_with_numbered_body("notes.txt");
    editor.state = EditState::ConfirmingExit(ExitChoice::default());
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let dialog = dialog_area(&terminal);

    // Equal margins on both sides is what "centred" means on screen, and the
    // dialog is sized to its content, so it cannot fill the row either.
    assert!(dialog.x > 0, "dialog must not touch column 0");
    // Within a column: an odd leftover cannot be split evenly, and rounding it
    // one way is centred while dumping it all on one side is not.
    let right_margin = SCREEN_W - (dialog.x + dialog.width);
    assert!(
        dialog.x.abs_diff(right_margin) <= 1,
        "margins must match, got left={} right={right_margin}:\n{}",
        dialog.x,
        row_text(&terminal, dialog.y)
    );
    assert!(
        dialog.width < SCREEN_W,
        "a content-sized dialog must not span the screen"
    );

    // Left of the dialog the editor is untouched, so it cannot be spilling
    // across the whole row.
    assert!(
        !cells(&terminal, dialog.y + 1, 0, dialog.x).contains("modified"),
        "prompt text must stay inside the dialog:\n{}",
        row_text(&terminal, dialog.y + 1)
    );
}

// -----------------------------------------------------------------------
// "Changed on disk" question (a save found another program's write)
// -----------------------------------------------------------------------

#[test]
fn a_save_over_another_programs_write_asks_with_the_cursor_on_cancel() {
    let (mut app, _left, _right) = app_on_empty_dirs();
    let mut editor = editor_with_numbered_body("notes.txt");
    editor.state = EditState::ConfirmingOverwrite(crate::models::OverwritePrompt::new(
        crate::models::AfterSave::Stay,
    ));
    app.set_overlay(Overlay::Editor(editor));

    let terminal = draw(&mut app);
    let screen = screen_text(&terminal);

    assert!(
        screen.contains("changed by another program"),
        "the question:\n{screen}"
    );
    assert!(screen.contains("Overwrite"), "its buttons:\n{screen}");
    assert!(screen.contains("Cancel"), "its buttons:\n{screen}");
    let area = dialog_area(&terminal);
    assert!(area.width > 0, "drawn on the warning ground");
}
