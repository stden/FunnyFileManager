//! Purpose: Pins the Far-style console layer — its backdrop, colours, and Ctrl+O view.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;

use super::*;
use crate::models::Panel;

pub(super) fn terminal() -> Terminal<TestBackend> {
    Terminal::new(TestBackend::new(60, 12)).expect("terminal")
}

pub(super) fn app_in(temp: &TempDir) -> App {
    let mut app = App::new().expect("test app");
    app.left_panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    app.right_panel = Panel::new(temp.path().to_path_buf()).expect("panel");
    app
}

pub(super) fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buf = terminal.backend().buffer();
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol())
        .collect::<String>()
        .trim_end()
        .to_owned()
}

pub(super) fn screen_text(terminal: &Terminal<TestBackend>) -> String {
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

    // Wide enough that the echoed `cwd> ls -la` line survives `head_cells`
    // truncation even when the OS temp root is long (macOS's
    // `/var/folders/.../T/.tmpXXXXXX` runs well past 60 columns).
    let mut terminal = Terminal::new(TestBackend::new(160, 12)).expect("terminal");
    terminal.draw(|f| ui(f, &mut app)).expect("draw");

    let screen = screen_text(&terminal);
    assert!(screen.contains("> ls -la"), "{screen}");
    assert!(screen.contains("total 0"), "{screen}");
    // The panel chrome is gone: no borders, no F-key bar.
    assert!(!screen.contains('╔'), "panels still drawn: {screen}");
    // What is left is a console: the echo, its output, then a prompt on the
    // very next row — no hint bar pinned to the bottom of the screen.
    assert!(row_text(&terminal, 2).starts_with('●'), "{screen}");
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

/// The width type prints as its number, which is what a panic message about a
/// layout that would not fit has to carry.
#[test]
fn the_cell_count_prints_as_its_number() {
    assert_eq!(Cells::new(17).to_string(), "17");
}

// ─── keys, through the main-view dispatcher ───────────────────────────────

/// Console mode is a view, not an overlay, so its keys go through the same
/// `Action` path the panels use. Drive them exactly as the event loop does.
pub(super) fn press(app: &mut App, code: KeyCode) {
    press_with(app, KeyEvent::new(code, KeyModifiers::empty()));
}

pub(super) fn press_with(app: &mut App, event: KeyEvent) {
    let mut terminal = Terminal::new(crate::WritableTestBackend::new(60, 12)).expect("terminal");
    crate::application::dispatch_key(&mut terminal, app, event);
}

pub(super) fn whole_screen(terminal: &Terminal<TestBackend>) -> String {
    crate::test_support::buffer_string(terminal.backend().buffer())
}
