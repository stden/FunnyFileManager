//! Purpose: console-mode keys and the `:` command line over panels and console.

use super::*;

#[test]
fn console_can_open_menu_and_screen_choices() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&[]);
    app.view = ViewMode::Console;
    press(&mut app, &mut terminal, KeyCode::F(9));
    assert!(matches!(app.overlay, Overlay::Menu(_)));
    app.close_overlay();
    press(&mut app, &mut terminal, KeyCode::F(12));
    assert!(matches!(app.overlay, Overlay::MenuDialog(_)));
}

#[test]
fn the_colon_key_opens_the_command_line_empty() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);
    press(&mut app, &mut terminal, KeyCode::Char(':'));
    let Overlay::CommandLine(line) = &app.overlay else {
        panic!("the command line is open");
    };
    assert!(
        line.line.is_empty(),
        "`:` opens it empty; a letter fills it"
    );
}

// ─── console mode has its own set of keys ───────────────────────────────────

/// In console mode the navigation keys scroll the transcript instead of the
/// listing — a separate dispatcher, and one nothing else exercises.
#[test]
fn the_navigation_keys_scroll_the_console_when_it_is_showing() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);
    for line in 0..200 {
        app.console.record_output(&format!("line {line}"));
    }
    app.view = app.view.toggled();
    // The *newest* visible line, not the oldest: near the top of the
    // transcript the window is clamped against the start, so the first line
    // stays put while the view genuinely moves.
    let newest = |app: &App| {
        app.console
            .visible(10)
            .last()
            .map(|line| line.text().to_owned())
            .unwrap_or_default()
    };

    press(&mut app, &mut terminal, KeyCode::End);
    let bottom = newest(&app);
    press(&mut app, &mut terminal, KeyCode::Home);
    let top = newest(&app);
    assert_ne!(top, bottom, "Home goes to the top, End to the bottom");

    press(&mut app, &mut terminal, KeyCode::PageDown);
    let paged = newest(&app);
    assert_ne!(paged, top, "a page moved the view");

    press(&mut app, &mut terminal, KeyCode::PageUp);
    assert_eq!(newest(&app), top, "and a page back returned it");

    press(&mut app, &mut terminal, KeyCode::Down);
    let one_down = newest(&app);
    assert_ne!(one_down, top, "the arrows move one line");
    press(&mut app, &mut terminal, KeyCode::Up);
    assert_eq!(newest(&app), top);
}

/// The console is a background view, not a modal: `:` must still put a command
/// line over it so a user can act on output without first returning to panels.
#[test]
fn the_colon_key_opens_the_command_line_over_the_console() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);
    app.view = app.view.toggled();

    press(&mut app, &mut terminal, KeyCode::Char(':'));

    assert!(matches!(app.overlay, Overlay::CommandLine(_)));
}

// ─── the command line still opens from the console ──────────────────────────

#[test]
fn colon_opens_the_command_line_even_while_the_console_is_showing() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);
    app.view = app.view.toggled();

    press(&mut app, &mut terminal, KeyCode::Char(':'));

    assert!(
        matches!(app.overlay, Overlay::CommandLine(_)),
        "the command line still opens from the console, Far-style"
    );
}

/// The chord's key with its modifiers, as the terminal sends it.
fn press_chord(
    app: &mut App,
    terminal: &mut Terminal<WritableTestBackend>,
    code: KeyCode,
    modifiers: KeyModifiers,
) {
    dispatch_key(terminal, app, key_event_with_mods(code, modifiers));
}

/// In console mode the F9 menu's own keys run their commands as they always
/// have. The same command reached by a chord with extra modifiers is swallowed:
/// a lenient chord must not wipe a file or open a dialog behind the transcript.
#[test]
fn console_runs_the_menus_own_keys_and_not_their_lenient_chords() {
    let (mut app, mut terminal, _dir) = app_and_terminal(&["a.txt"]);
    app.view = ViewMode::Console;

    press_chord(
        &mut app,
        &mut terminal,
        KeyCode::Char('a'),
        KeyModifiers::CONTROL,
    );
    assert!(
        matches!(app.overlay, Overlay::Attributes(_)),
        "Ctrl+A is the menu's own key and runs Attributes"
    );
    app.close_overlay();

    press_chord(
        &mut app,
        &mut terminal,
        KeyCode::Char('a'),
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    );
    assert!(
        !app.is_overlay_active(),
        "Ctrl+Alt+A opened a dialog behind the console"
    );
}

/// The wipe chord with extra modifiers is swallowed in console mode, so it
/// cannot delete a file behind the transcript.
#[test]
fn console_does_not_wipe_behind_the_transcript() {
    let (mut app, mut terminal, dir) = app_and_terminal(&["a.txt"]);
    app.view = ViewMode::Console;

    press_chord(
        &mut app,
        &mut terminal,
        KeyCode::Delete,
        KeyModifiers::ALT | KeyModifiers::SHIFT,
    );
    assert!(!app.is_overlay_active(), "Alt+Shift+Del opened a dialog");
    assert!(
        dir.path().join("a.txt").exists(),
        "Alt+Shift+Del wiped a file behind the console"
    );
}
