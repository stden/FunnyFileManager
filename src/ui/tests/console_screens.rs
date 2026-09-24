//! Unit tests: every layer renders on a tiny terminal, and the panel view
//! leaks none of the transcript into the menu bar.

use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;

use super::console_layer::{app_in, row_text, screen_text};
use super::*;
use crate::models::Panel;

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

/// The panel view must not paint the transcript at all.
///
/// It used to be drawn as the bottom layer and "covered by the panels", but a
/// ratatui widget writes symbols only where its text reaches: the menu bar's
/// headings stop after 44 cells, so the tail of a longer echoed command stayed
/// on the top row — repainted from the log on every frame, surviving every
/// redraw and every change of directory.
#[test]
fn the_panel_view_does_not_leak_the_transcript_into_the_menu_bar() {
    let temp = TempDir::new().expect("temp");
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(temp.path().to_path_buf()).expect("left");
    app.right_panel = Panel::new(temp.path().to_path_buf()).expect("right");
    app.console
        .record_command(temp.path(), "echo MARKER_FROM_A_LONG_COMMAND_LINE");
    app.view = ViewMode::Panels;

    let mut terminal = Terminal::new(TestBackend::new(90, 16)).expect("terminal");
    terminal.draw(|f| ui(f, &mut app)).expect("draw");

    let screen = screen_text(&terminal);
    assert!(
        !screen.contains("MARKER"),
        "the transcript showed through the panel view: {screen}"
    );
    assert!(
        row_text(&terminal, 0).trim_end().ends_with("Right")
            || row_text(&terminal, 0).contains("Right  "),
        "the menu bar row carried something besides its headings and the clock: {:?}",
        row_text(&terminal, 0)
    );

    // Ctrl+O still has it: the transcript was hidden, not lost.
    app.view = ViewMode::Console;
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    assert!(
        screen_text(&terminal).contains("MARKER"),
        "the console screen lost the transcript"
    );
}
