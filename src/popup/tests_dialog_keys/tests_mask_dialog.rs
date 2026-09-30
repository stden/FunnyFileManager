//! Purpose: the mask dialog's keys — Tab between its buttons, the caret in its field, and editing the mask where the caret stands.

use super::*;

// ─── the mask dialog ────────────────────────────────────────────────────────

fn open_mask(app: &mut App, offered: &str) {
    app.set_overlay(Overlay::MaskPrompt(MaskPrompt::new(
        MaskPurpose::Select,
        offered,
    )));
}

fn mask_prompt(app: &App) -> &MaskPrompt {
    let Overlay::MaskPrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    prompt
}

#[test]
fn tab_in_the_mask_dialog_means_the_other_button() {
    let (mut app, _dir) = app_over(&[]);
    open_mask(&mut app, "*");
    press(&mut app, KeyCode::Tab);
    assert!(mask_prompt(&app).on_cancel(), "Tab moved to Cancel");
}

/// The reported case: Right used to move the cursor to Cancel while the
/// typed `.img` still went into the field, and Enter then threw it away.
#[test]
fn left_and_right_in_the_mask_field_move_the_caret_not_the_focus() {
    let (mut app, _dir) = app_over(&["sparse.img"]);
    open_mask(&mut app, "*");
    press(&mut app, KeyCode::Right);
    for ch in ".img".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    assert!(!mask_prompt(&app).on_cancel(), "the field keeps the cursor");
    assert_eq!(mask_prompt(&app).mask(), "*.img");

    press(&mut app, KeyCode::Enter);
    assert!(app.left_panel.marked.contains("sparse.img"));
}

/// The caret moves inside the text, so a typo early in the mask is fixed in
/// place rather than by deleting back to it.
#[test]
fn the_caret_edits_the_mask_where_it_stands() {
    let (mut app, _dir) = app_over(&[]);
    open_mask(&mut app, "*.txt");
    for code in [KeyCode::Left, KeyCode::Left, KeyCode::Left] {
        press(&mut app, code);
    }
    press(&mut app, KeyCode::Char('x'));
    assert_eq!(mask_prompt(&app).mask(), "*.xtxt");
    press(&mut app, KeyCode::Delete);
    press(&mut app, KeyCode::Home);
    press(&mut app, KeyCode::Delete);
    press(&mut app, KeyCode::End);
    press(&mut app, KeyCode::Backspace);
    assert_eq!(mask_prompt(&app).mask(), ".xx");
}

/// On a button, Left and Right walk the buttons, as in Far's dialogs.
#[test]
fn on_a_button_left_and_right_walk_the_buttons() {
    for code in [KeyCode::Left, KeyCode::Right] {
        let (mut app, _dir) = app_over(&[]);
        open_mask(&mut app, "*");
        press(&mut app, KeyCode::Tab);
        press(&mut app, code);
        assert!(!mask_prompt(&app).on_cancel(), "{code:?} went back to OK");
    }
}

#[test]
fn backspace_edits_the_mask() {
    let (mut app, _dir) = app_over(&[]);
    app.set_overlay(Overlay::MaskPrompt(MaskPrompt::new(
        MaskPurpose::Select,
        "*.rs",
    )));
    press(&mut app, KeyCode::End);
    press(&mut app, KeyCode::Backspace);
    let Overlay::MaskPrompt(prompt) = &app.overlay else {
        panic!("still open");
    };
    assert_eq!(prompt.mask(), "*.r");
}
