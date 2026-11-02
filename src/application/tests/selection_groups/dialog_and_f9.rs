//! Purpose: the group-selection dialog end to end — it marks what was typed, Esc and Cancel leave the marks — and F9 as a toggle.

use super::*;

// ─── the dialog end to end ──────────────────────────────────────────────────

#[test]
fn the_dialog_selects_what_was_typed_into_it() {
    let (mut app, _dir) = app_over(&FILES);
    app.set_overlay(Overlay::MaskPrompt(crate::models::MaskPrompt::new(
        MaskPurpose::Select,
        "",
    )));
    for ch in "*.txt".chars() {
        process_popup_input(&mut app, key_event(KeyCode::Char(ch)));
    }
    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert_eq!(marked(&mut app), vec!["c.txt", "d.txt"]);
    assert!(
        app.status.to_string().contains('2'),
        "the status counts what changed, got {}",
        app.status
    );
}

#[test]
fn escape_and_the_cancel_button_leave_the_selection_alone() {
    for cancel_with in [key_event(KeyCode::Esc), key_event(KeyCode::Enter)] {
        let (mut app, _dir) = app_over(&FILES);
        let mut prompt = crate::models::MaskPrompt::new(MaskPurpose::Select, "*");
        if cancel_with.code == KeyCode::Enter {
            prompt.toggle_button();
        }
        app.set_overlay(Overlay::MaskPrompt(prompt));
        process_popup_input(&mut app, cancel_with);
        assert_eq!(marked(&mut app).len(), 0, "cancelled with {cancel_with:?}");
    }
}

// ─── F9 opens and closes ────────────────────────────────────────────────────

/// A key that only opens leaves the user hunting for the one that shuts it.
#[test]
fn f9_closes_the_menu_it_opened() {
    let (mut app, _dir) = app_over(&FILES);
    app.set_menu(crate::menu::Menu::open(crate::menu::Heading::Files));
    assert!(matches!(app.overlay, Overlay::Menu(_)));

    process_popup_input(&mut app, key_event(KeyCode::F(9)));
    assert!(
        !app.overlay.is_active(),
        "F9 must close the bar as well as open it"
    );
}

/// …and it does not run whatever row the cursor happened to be on.
#[test]
fn closing_with_f9_runs_nothing() {
    let (mut app, _dir) = app_over(&FILES);
    app.set_menu(crate::menu::Menu::open(crate::menu::Heading::Files));
    process_popup_input(&mut app, key_event(KeyCode::F(9)));
    assert!(app.pending_row.is_none(), "no row was handed on");
}
