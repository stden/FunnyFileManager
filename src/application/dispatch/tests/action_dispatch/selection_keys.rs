//! Purpose: the Gray +, -, * and Ctrl+M selection-group keys through the dispatcher.

use super::*;

// ─── the selection group actions, through the dispatcher ────────────────────

#[test]
fn the_group_keys_reach_their_actions_through_the_dispatcher() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.rs", "b.txt"]);

    press(&mut app, &mut terminal, KeyCode::Char('+'));
    assert!(
        matches!(app.overlay, Overlay::MaskPrompt(_)),
        "Gray + asks which files"
    );
    press(&mut app, &mut terminal, KeyCode::Esc);

    press(&mut app, &mut terminal, KeyCode::Char('-'));
    assert!(matches!(app.overlay, Overlay::MaskPrompt(_)));
    press(&mut app, &mut terminal, KeyCode::Esc);
}

#[test]
fn inverting_and_restoring_report_how_many_are_selected() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.rs", "b.txt", "c.md"]);

    press(&mut app, &mut terminal, KeyCode::Char('*'));
    assert_eq!(app.get_active_panel().marked.len(), 3, "all of them");
    assert!(app.status.to_string().contains('3'), "{}", app.status);

    press(&mut app, &mut terminal, KeyCode::Char('*'));
    assert_eq!(app.get_active_panel().marked.len(), 0);

    dispatch_key(
        &mut terminal,
        &mut app,
        KeyEvent::new(KeyCode::Char('m'), KeyModifiers::CONTROL),
    );
    assert_eq!(
        app.get_active_panel().marked.len(),
        3,
        "Ctrl+M puts back what the last inversion replaced"
    );
    assert!(app.status.to_string().contains('3'), "{}", app.status);
}

/// The mask dialog opens on the extension under the cursor, which is what
/// makes the key worth pressing on a listing of mixed types.
#[test]
fn the_mask_dialog_opens_on_the_extension_of_the_row_under_the_cursor() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["report.md"]);
    select_named(&mut app.left_panel, "report.md");

    press(&mut app, &mut terminal, KeyCode::Char('+'));

    let Overlay::MaskPrompt(prompt) = &app.overlay else {
        panic!("the mask dialog is open");
    };
    assert_eq!(prompt.mask(), "*.md");
}
