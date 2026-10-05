//! Purpose: Session history and the ghost suggestion drawn from it.

use super::*;

#[test]
fn executed_commands_enter_the_session_history() {
    // `App::new()` is session-only: it starts with `History::default()` and
    // does no disk I/O (production loads the real history in `main_impl`, never
    // in any test — see `models.rs`/`lib.rs`). So this App's history is empty
    // except for "pwd", recorded into the session ring by this very test; no
    // on-disk history can affect the assertion, and running these tests never
    // reads or writes the real `~/.local/share/ffm/history`.
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::CommandLine("pwd".into()));

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    assert_eq!(app.history.suggest("pw"), Some("pwd"));
}

/// Seed history, open a fresh command line, and return the App.
fn app_with_history(lines: &[&str]) -> App {
    let mut app = App::new().expect("test app");
    for line in lines {
        app.history.record(line);
    }
    app.set_overlay(Overlay::CommandLine(CommandLine::default()));
    app
}

#[test]
fn typing_shows_the_most_recent_matching_tail_as_ghost() {
    let mut app = app_with_history(&["cd /somewhere"]);

    process_popup_input(&mut app, key_event(KeyCode::Char('c')));
    process_popup_input(&mut app, key_event(KeyCode::Char('d')));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line open");
    };
    assert_eq!(state.text(), "cd");
    assert_eq!(state.ghost.as_deref(), Some(" /somewhere"));
}

#[test]
fn right_accepts_the_ghost_into_the_buffer() {
    let mut app = app_with_history(&["cd /somewhere"]);
    process_popup_input(&mut app, key_event(KeyCode::Char('c')));

    process_popup_input(&mut app, key_event(KeyCode::Right));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line open");
    };
    assert_eq!(state.text(), "cd /somewhere");
    assert_eq!(state.ghost, None, "tail fully consumed");
}

#[test]
fn backspace_recomputes_the_ghost() {
    let mut app = app_with_history(&["pwd", "pull"]);
    // "p" → newest match "pull" wins; then extend to "pw" → only "pwd".
    process_popup_input(&mut app, key_event(KeyCode::Char('p')));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert_eq!(state.ghost.as_deref(), Some("ull"));

    process_popup_input(&mut app, key_event(KeyCode::Char('w')));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert_eq!(state.ghost.as_deref(), Some("d"));

    process_popup_input(&mut app, key_event(KeyCode::Backspace));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert_eq!(state.text(), "p");
    assert_eq!(state.ghost.as_deref(), Some("ull"));
}

#[test]
fn ghost_is_suppressed_while_the_menu_is_open() {
    let mut app = app_with_history(&["mark all"]);
    process_popup_input(&mut app, key_event(KeyCode::Char('m')));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert!(state.ghost.is_some(), "ghost shows while menu closed");

    // Tab opens the menu ("m" is ambiguous among verbs) — ghost must clear.
    process_popup_input(&mut app, key_event(KeyCode::Tab));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert!(state.menu.is_some());
    assert_eq!(state.ghost, None, "no two accept-targets at once");

    // Esc closes the menu; the ghost returns for the unchanged buffer.
    process_popup_input(&mut app, key_event(KeyCode::Esc));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert!(state.menu.is_none());
    assert!(state.ghost.is_some(), "ghost returns once the menu closes");
}

#[test]
fn right_without_a_ghost_is_swallowed_and_harmless() {
    let mut app = app_with_history(&[]);
    process_popup_input(&mut app, key_event(KeyCode::Char('z')));

    assert!(process_popup_input(&mut app, key_event(KeyCode::Right)));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert_eq!(state.text(), "z");
}

#[test]
fn ghost_reappears_after_accepting_a_menu_candidate() {
    // Derive which verb candidate the menu will highlight for "m", instead of
    // guessing: `complete()` ranks candidates the same way `Tab` does, and
    // `open_menu` (src/popup.rs) always starts `selected` at index 0, so
    // `candidates[0]` is exactly the row `Enter` accepts below. The verb
    // branch of `complete()` never touches `cwd`, so any path will do.
    let comp =
        crate::complete::complete("m", Path::new(".")).expect("\"m\" matches at least one verb");
    let selected = comp.candidates[0].text.clone();

    // Verbs are always `CandidateKind::Word`, so `finish_word` appends a
    // trailing space on accept: the buffer becomes "<selected> ". Seed a
    // history line with exactly that prefix plus a tail, so `suggest` has a
    // literal match once the candidate is accepted.
    let seeded = format!("{selected} status");
    let mut app = app_with_history(&[&seeded]);

    process_popup_input(&mut app, key_event(KeyCode::Char('m')));
    process_popup_input(&mut app, key_event(KeyCode::Tab));
    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert!(state.menu.is_some(), "sanity check: \"m\" opens a menu");
    assert_eq!(state.ghost, None, "ghost suppressed while menu is open");

    process_popup_input(&mut app, key_event(KeyCode::Enter));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!()
    };
    assert!(state.menu.is_none(), "Enter accepts and closes the menu");
    assert_eq!(state.text(), format!("{selected} "));
    assert_eq!(
        state.ghost.as_deref(),
        Some("status"),
        "accepting a candidate is a buffer edit; the ghost must recompute \
         immediately instead of waiting for the next keystroke"
    );
}
