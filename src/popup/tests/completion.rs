//! Purpose: Tab completion on the command line through the popup router — the candidate menu, a single candidate, and helpers that are idle once the line has closed.

use super::*;

/// Multi-byte whitespace (NBSP from a paste) used to panic tab-completion,
/// which aborts the process with the terminal still in raw mode.
#[test]
fn test_command_line_tab_completion_survives_multibyte_whitespace() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join("Documents")).expect("mkdir");

    let mut app = App::new().expect("test app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.active_panel = crate::models::PanelSide::Left;
    app.set_overlay(Overlay::CommandLine("cd\u{a0}Doc".into()));

    assert!(process_popup_input(&mut app, key(KeyCode::Tab)));
    assert!(matches!(app.overlay, Overlay::CommandLine(_)));
}

#[test]
fn refresh_helpers_bail_out_when_the_command_line_is_closed() {
    // Only the command-line overlay owns a menu/ghost; a stray call with any
    // other overlay must be a no-op instead of a panic.
    let mut app = App::new().expect("test app");
    crate::popup::command_line::refresh_menu(&mut app);
    crate::popup::command_line::refresh_ghost(&mut app);
    assert!(matches!(app.overlay, Overlay::None));
}

#[test]
fn tab_with_an_ambiguous_prefix_opens_the_candidate_menu() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join("alpha")).expect("dir");
    std::fs::create_dir(temp.path().join("alpine")).expect("dir");
    let mut app = App::new().expect("test app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.set_overlay(Overlay::CommandLine("cd al".into()));

    process_popup_input(&mut app, key(KeyCode::Tab));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line must stay open");
    };
    assert!(
        state.menu.is_some(),
        "two candidates must open the menu, not silently pick one"
    );
}

#[test]
fn tab_with_a_single_candidate_completes_without_a_menu() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::create_dir(temp.path().join("alpha")).expect("dir");
    let mut app = App::new().expect("test app");
    app.left_panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    app.set_overlay(Overlay::CommandLine("cd al".into()));

    process_popup_input(&mut app, key(KeyCode::Tab));

    let Overlay::CommandLine(state) = &app.overlay else {
        panic!("command line must stay open");
    };
    assert!(state.text().contains("alpha"), "{}", state.text());
    assert!(
        state.menu.is_none(),
        "a lone candidate is accepted outright, no menu"
    );
}

#[test]
fn completing_is_harmless_without_the_command_line() {
    // Tab only routes to the completer from the command-line overlay, but a
    // stray call must be a no-op rather than a panic.
    let mut app = App::new().expect("test app");
    crate::popup::command_line::complete_at_cursor(&mut app);
    assert!(matches!(app.overlay, Overlay::None));
}
