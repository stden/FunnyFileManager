//! Purpose: Tab completion's menu: opening, moving, accepting, cycling and closing.

use super::*;

#[test]
fn test_tab_opens_a_menu_when_several_candidates_match() {
    let mut app = App::new().expect("test setup");
    // "m" matches at least "mark", "mcp" and "unmark".
    app.set_overlay(Overlay::CommandLine("m".into()));

    process_popup_input(&mut app, key_event(KeyCode::Tab));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    let menu = state.menu.as_ref().expect("menu should be open");
    assert!(menu.candidates().len() > 1);
    assert_eq!(menu.selected(), 0);
}

#[test]
fn test_tab_accepts_directly_when_only_one_candidate_matches() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("unm".into()));

    process_popup_input(&mut app, key_event(KeyCode::Tab));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(state.text(), "unmark ");
    assert!(state.menu.is_none(), "a lone candidate opens no menu");
}

#[test]
fn test_down_moves_the_menu_selection() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    process_popup_input(&mut app, key_event(KeyCode::Down));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(state.menu.as_ref().expect("menu").selected(), 1);
}

#[test]
fn test_enter_accepts_the_selected_candidate_instead_of_running_it() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    // Read the highlighted candidate's text before accepting, so the
    // expectation tracks the real ranking instead of a hardcoded guess.
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    let menu = state.menu.as_ref().expect("menu should be open");
    let expected = menu.candidates()[menu.selected()].text.clone();

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("Enter with a menu open must accept, not execute and close");
    };
    assert!(state.menu.is_none(), "accepting closes the menu");
    // A verb is a `Word`, so accepting it finishes it with a trailing space —
    // the same rule Tab applies to a lone candidate.
    assert_eq!(state.text(), format!("{expected} "));
}

#[test]
fn test_enter_accepts_a_selection_below_the_first_row() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    process_popup_input(&mut app, key_event(KeyCode::Down));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    let menu = state.menu.as_ref().expect("menu should be open");
    assert_eq!(menu.selected(), 1, "sanity check: Down moved the selection");
    let expected = menu.candidates()[1].text.clone();

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert_eq!(
        buffer_of(&app),
        format!("{expected} "),
        "Enter must accept the highlighted row, not always the first one"
    );
}

#[test]
fn test_enter_accept_splits_on_multi_byte_whitespace() {
    // U+3000 is 3 bytes; splitting the head one byte past its start would cut
    // the char in half and panic, taking the whole TUI down.
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("m\u{3000}x"),
        menu: Menu::new(vec![Candidate {
            text: "mcp".to_owned(),
            kind: CandidateKind::Word,
            hint: None,
        }]),
        ghost: None,
        recall: Recall::Idle,
    }));

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert_eq!(
        buffer_of(&app),
        "m\u{3000}mcp ",
        "only the last token is replaced, and the separator survives intact"
    );
}

#[test]
fn test_esc_closes_only_the_menu_not_the_overlay() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    process_popup_input(&mut app, key_event(KeyCode::Esc));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("first Esc must not close the command line");
    };
    assert!(state.menu.is_none());

    // A second Esc closes the overlay itself.
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn test_up_at_the_first_candidate_wraps_to_the_last() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));
    let last = menu_texts(&app).len() - 1;

    process_popup_input(&mut app, key_event(KeyCode::Up));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(state.menu.as_ref().expect("menu").selected(), last);
}

#[test]
fn test_moving_caret_closes_the_menu_so_enter_does_not_splice_elsewhere() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));
    assert!(
        matches!(&app.overlay, Overlay::CommandLine(state) if state.menu.is_some()),
        "menu opens on ambiguous prefix"
    );

    process_popup_input(&mut app, key_event(KeyCode::Left));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert!(
        state.menu.is_none(),
        "moving the caret must close the completion menu"
    );
}
