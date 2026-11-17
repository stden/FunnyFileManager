//! Purpose: An open completion menu follows typing, Backspace and Tab, and never corrupts the line.

use super::*;

#[test]
fn test_typing_refreshes_an_open_menu() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    // Move the selection off 0 first, so the post-typing assertion below can
    // actually distinguish "reset to 0" from "never touched".
    process_popup_input(&mut app, key_event(KeyCode::Down));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(
        state.menu.as_ref().expect("menu should be open").selected(),
        1,
        "sanity check: Down must move the selection before typing resets it"
    );

    process_popup_input(&mut app, key_event(KeyCode::Char('c')));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(state.text(), "mc");
    let menu = state.menu.as_ref().expect("menu stays open while typing");
    assert_eq!(menu.selected(), 0, "refreshing resets the selection");
    // The list itself must be recomputed for the new buffer, not just
    // re-highlighted: only "mcp" still matches "mc".
    assert_eq!(
        menu_texts(&app),
        vec!["mcp".to_owned()],
        "the candidate list must track the edited buffer"
    );
}

#[test]
fn test_backspace_refreshes_an_open_menu() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));
    let wide_list = menu_texts(&app);
    assert!(
        wide_list.len() > 1,
        "sanity check: \"m\" is ambiguous, got {wide_list:?}"
    );

    process_popup_input(&mut app, key_event(KeyCode::Char('c')));
    assert_eq!(menu_texts(&app), vec!["mcp".to_owned()]);

    process_popup_input(&mut app, key_event(KeyCode::Backspace));

    assert_eq!(buffer_of(&app), "m");
    assert_eq!(
        menu_texts(&app),
        wide_list,
        "deleting a character must widen the list back out"
    );
}

#[test]
fn test_typing_into_a_dead_end_closes_the_menu() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    // No verb matches "mz", so there is nothing left to show.
    process_popup_input(&mut app, key_event(KeyCode::Char('z')));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(state.text(), "mz");
    assert!(
        state.menu.is_none(),
        "an empty candidate set must close the dropdown"
    );
}

#[test]
fn test_typed_command_is_not_corrupted_by_a_stale_menu() {
    // Regression: the menu used to stay frozen at Tab-time, so Enter spliced a
    // long-obsolete candidate ("mcp") over the last token and destroyed the
    // text typed since — "mark all" became "mark mcp" and never ran.
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    for ch in "ark all".chars() {
        process_popup_input(&mut app, key_event(KeyCode::Char(ch)));
    }
    assert_eq!(
        buffer_of(&app),
        "mark all",
        "typing must never rewrite the buffer"
    );

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    // The refreshed menu offers only "all", so Enter accepts that — the typed
    // command is preserved, never overwritten with a stale candidate.
    let buffer = buffer_of(&app);
    assert_ne!(buffer, "mark mcp", "stale candidate must not be spliced in");
    assert_eq!(buffer, "mark all ");

    // With the menu closed by that acceptance, Enter now runs the command.
    process_popup_input(&mut app, key_event(KeyCode::Enter));
    assert!(
        matches!(app.overlay, Overlay::None),
        "the second Enter must execute and close the command line"
    );
}

#[test]
fn test_tab_cycles_an_open_menu_instead_of_rebuilding_it() {
    let mut app = App::new().expect("test setup");
    app.set_overlay(Overlay::CommandLine("m".into()));
    process_popup_input(&mut app, key_event(KeyCode::Tab));
    let candidates = menu_texts(&app);
    let count = candidates.len();
    assert!(count > 2, "sanity check: need >2 candidates, got {count}");

    process_popup_input(&mut app, key_event(KeyCode::Down));
    process_popup_input(&mut app, key_event(KeyCode::Tab));

    assert_eq!(
        selected_candidate(&app),
        2,
        "Tab advances the existing selection"
    );
    assert_eq!(
        menu_texts(&app),
        candidates,
        "cycling must not rebuild the list"
    );

    // Keep cycling past the end: the selection wraps back to the first row.
    for _ in 2..count {
        process_popup_input(&mut app, key_event(KeyCode::Tab));
    }
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line should still be open");
    };
    assert_eq!(
        state.menu.as_ref().expect("menu stays open").selected(),
        0,
        "Tab wraps around at the end of the list"
    );
}

fn selected_candidate(app: &App) -> usize {
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line stays open")
    };
    state.menu.as_ref().expect("menu stays open").selected()
}
