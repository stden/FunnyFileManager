//! Purpose: Pins the Far-style console layer — its backdrop, colours, and Ctrl+O view.

use super::*;
use crate::models::ViewMode;

use crate::models::{App, Overlay, Panel};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;

fn terminal() -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(60, 12)).expect("terminal")
}

fn app_in(temp: &TempDir) -> App {
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    app.right_panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    app
}

fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buf = terminal.backend().buffer();
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol())
        .collect::<String>()
        .trim_end()
        .to_string()
}

fn screen_text(terminal: &Terminal<TestBackend>) -> String {
    let buf = terminal.backend().buffer();
    (0..buf.area.height)
        .map(|y| row_text(terminal, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The console transcript is painted first, so the panels cover it — exactly
/// as Far Manager draws panels over the console screen.
#[test]
fn the_transcript_is_hidden_behind_the_panels() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    app.console.record_output("secret-transcript-line");

    let mut terminal = terminal();
    terminal.draw(|f| ui(f, &mut app)).expect("draw");

    assert!(
        !screen_text(&terminal).contains("secret-transcript-line"),
        "panels must cover the console layer: {}",
        screen_text(&terminal)
    );
}

/// Ctrl+O lifts the panels off and leaves that same layer on screen.
#[test]
fn ctrl_o_reveals_the_transcript_and_hides_the_panels() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    app.console.record_command(temp.path(), "ls -la");
    app.console.record_output("total 0");
    app.view = ViewMode::Console;

    let mut terminal = terminal();
    terminal.draw(|f| ui(f, &mut app)).expect("draw");

    let screen = screen_text(&terminal);
    assert!(screen.contains("> ls -la"), "{screen}");
    assert!(screen.contains("total 0"), "{screen}");
    // The panel chrome is gone: no borders, no F-key bar.
    assert!(!screen.contains('┌'), "panels still drawn: {screen}");
    assert!(screen.contains("Ctrl+O returns"), "{screen}");
}

/// The transcript sits on black, and each kind of line is told apart by colour
/// so a long one reads as a sequence of commands rather than a wall of text.
#[test]
fn command_output_and_error_lines_are_coloured_apart_on_black() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    app.console.record_command(temp.path(), "build");
    app.console.record_output("ok");
    app.console.record_error("boom");
    app.view = ViewMode::Console;

    let mut terminal = terminal();
    terminal.draw(|f| ui(f, &mut app)).expect("draw");

    let buf = terminal.backend().buffer();
    // A console fills downwards from the top, so the three lines are rows 0-2.
    assert_eq!(buf[(0, 0)].bg, Color::Black);
    assert_eq!(buf[(0, 0)].fg, Color::White);
    assert_eq!(buf[(0, 1)].fg, Color::Gray);
    assert_eq!(buf[(0, 2)].fg, Color::LightRed);
}

/// The newest output is what Ctrl+O shows, whatever the reader last scrolled to.
#[test]
fn the_view_is_anchored_to_the_newest_lines() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    for i in 0..40 {
        app.console.record_output(&format!("line {i}"));
    }
    app.view = ViewMode::Console;

    let mut terminal = terminal();
    terminal.draw(|f| ui(f, &mut app)).expect("draw");

    let screen = screen_text(&terminal);
    // The newest line must be readable, not hidden under the hint row.
    assert!(screen.contains("line 39"), "{screen}");
    assert!(!screen.contains("line 0\n"), "{screen}");
}

/// A line wider than the terminal is cut, never wrapped: one transcript line
/// per row is what keeps the scroll arithmetic in `ConsoleLog` honest.
#[test]
fn an_overlong_line_is_cut_rather_than_wrapped() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    app.console.record_output(&"x".repeat(200));
    app.console.record_output("second line");
    app.view = ViewMode::Console;

    let mut terminal = terminal();
    terminal.draw(|f| ui(f, &mut app)).expect("draw");

    // The two records occupy exactly two rows: a wrapped first line would
    // push the second one down and desynchronise the scroll arithmetic.
    assert_eq!(row_text(&terminal, 1), "second line");
}

// ─── keys, through the main-view dispatcher ───────────────────────────────

/// Console mode is a view, not an overlay, so its keys go through the same
/// `Action` path the panels use. Drive them exactly as the event loop does.
fn press(app: &mut App, code: KeyCode) {
    press_with(app, KeyEvent::new(code, KeyModifiers::empty()));
}

fn press_with(app: &mut App, event: KeyEvent) {
    let mut terminal =
        ratatui::Terminal::new(crate::WritableTestBackend::new(60, 12)).expect("terminal");
    let mut last_nav = std::time::Instant::now();
    crate::dispatch_key(
        &mut terminal,
        app,
        event,
        &mut last_nav,
        std::time::Duration::ZERO,
    );
}

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
        assert_eq!(app.active_panel, crate::models::PanelSide::Left);
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

#[test]
fn the_prompt_takes_the_bottom_row_from_the_hint() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    app.console.record_output("earlier output");
    app.view = ViewMode::Console;

    let mut terminal = terminal();
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    assert!(screen_text(&terminal).contains("Ctrl+O returns"));

    app.set_overlay(Overlay::CommandLine("ls -la".into()));
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let screen = screen_text(&terminal);
    assert!(screen.contains("ls -la"), "{screen}");
    assert!(
        !screen.contains("Ctrl+O returns"),
        "the prompt replaces the hint rather than fighting it: {screen}"
    );
    assert!(screen.contains("earlier output"), "{screen}");
}

/// Command output arrives from a program that thought it had a terminal.
/// Nothing here emulates one, so the transcript stores the printable form and
/// the screen shows that — not the literal `[31mred[0m` ratatui would leave.
#[test]
fn colour_codes_and_tabs_never_reach_the_screen() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    app.console.record_output("\u{1b}[31mred\u{1b}[0m\tcol");
    app.view = ViewMode::Console;

    let mut terminal = Terminal::new(TestBackend::new(40, 6)).expect("terminal");
    terminal.draw(|f| ui(f, &mut app)).expect("draw");

    assert_eq!(row_text(&terminal, 0), "red     col");
}

/// Every layer has to survive a terminal too small to hold it: the panels, the
/// console, and the prompt each do their own cell budgeting.
#[test]
fn tiny_terminals_render_every_layer_without_panicking() {
    for (width, height) in [(1_u16, 1_u16), (2, 1), (1, 2), (3, 3), (80, 1)] {
        let temp = TempDir::new().expect("temp dir");
        let mut app = app_in(&temp);
        app.console.record_command(temp.path(), "ls");
        app.console.record_output("some output line");

        let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
        terminal
            .draw(|f| ui(f, &mut app))
            .unwrap_or_else(|e| panic!("{width}x{height} panels: {e}"));

        app.view = ViewMode::Console;
        terminal
            .draw(|f| ui(f, &mut app))
            .unwrap_or_else(|e| panic!("{width}x{height} console: {e}"));

        app.set_overlay(Overlay::CommandLine("cd /tmp".into()));
        terminal
            .draw(|f| ui(f, &mut app))
            .unwrap_or_else(|e| panic!("{width}x{height} prompt: {e}"));
    }
}
