//! Unit tests: the console mode's keys — scrolling the transcript, keys that
//! would touch the panels swallowed, Ctrl+O both ways, F10, and the command
//! line running while the console shows.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::TempDir;

use super::console_layer::{app_in, press, press_with};
use super::*;
use crate::scroll::ScrollBack;

#[test]
fn console_mode_scroll_keys_walk_the_transcript() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    for i in 0..40 {
        app.console.record_output(&format!("line {i}"));
    }
    app.view = ViewMode::Console;

    press(&mut app, KeyCode::Up);
    assert_eq!(app.console.scroll().lines(), 1);
    // A page is the terminal itself, since the console is drawn full-screen.
    press(&mut app, KeyCode::PageUp);
    assert_eq!(app.console.scroll().lines(), 13);
    press(&mut app, KeyCode::Down);
    assert_eq!(app.console.scroll().lines(), 12);
    press(&mut app, KeyCode::Home);
    assert_eq!(app.console.scroll().lines(), 39);
    press(&mut app, KeyCode::End);
    assert_eq!(app.console.scroll(), ScrollBack::BOTTOM);
}

#[test]
fn console_mode_swallows_actions_that_would_touch_the_panels() {
    let temp = TempDir::new().expect("temp dir");
    std::fs::write(temp.path().join("victim.txt"), b"x").expect("write");
    let mut app = app_in(&temp);
    app.view = ViewMode::Console;
    app.left_panel.select(Some(0));

    // The panels are off screen; F8 must not put a delete dialog behind the
    // console, and Tab must not move the directory a command would run in.
    for code in [KeyCode::F(8), KeyCode::F(5), KeyCode::Tab, KeyCode::Enter] {
        press(&mut app, code);
        assert!(
            matches!(app.overlay, Overlay::None),
            "{code:?} opened an overlay behind the console"
        );
        assert_eq!(app.view, ViewMode::Console, "{code:?} left console mode");
        assert_eq!(app.active_panel, PanelSide::Left);
    }
}

#[test]
fn ctrl_o_toggles_console_mode_in_both_directions() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);

    press_with(
        &mut app,
        KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
    );
    assert_eq!(app.view, ViewMode::Console);
    press_with(
        &mut app,
        KeyEvent::new(KeyCode::Char('o'), KeyModifiers::CONTROL),
    );
    assert_eq!(app.view, ViewMode::Panels);
}

#[test]
fn f10_still_quits_from_console_mode() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    app.view = ViewMode::Console;
    press(&mut app, KeyCode::F(10));
    assert!(!app.lifecycle.is_running());
}

// ─── typing while the console shows ───────────────────────────────────────

/// The whole reason `ViewMode` is separate from `Overlay`: Far's Ctrl+O is
/// "show me the output *and let me keep working*". As one enum these two
/// states were mutually exclusive and the prompt was unreachable here.
#[test]
fn the_command_line_opens_and_runs_while_the_console_shows() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    app.view = ViewMode::Console;

    // Typing opens the prompt without leaving console mode.
    press(&mut app, KeyCode::Char('p'));
    press(&mut app, KeyCode::Char('w'));
    press(&mut app, KeyCode::Char('d'));
    assert_eq!(app.get_command_line(), Some("pwd"));
    assert_eq!(app.view, ViewMode::Console);

    // And Enter runs it, with the echo landing in the transcript behind it.
    press(&mut app, KeyCode::Enter);
    assert_eq!(app.view, ViewMode::Console, "still watching the console");
    let transcript: Vec<String> = app
        .console
        .visible(usize::MAX)
        .into_iter()
        .map(|line| line.text().to_owned())
        .collect();
    assert!(
        transcript.iter().any(|line| line.ends_with("> pwd")),
        "{transcript:?}"
    );
}
