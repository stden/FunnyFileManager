//! Unit tests: where the console's prompt sits — after the newest line, on
//! the last row of a full transcript, alone on an empty one — the completion
//! popup under it, and the banners that replace it.

use crossterm::event::KeyCode;
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use tempfile::TempDir;

use super::console_layer::{app_in, press, row_text, screen_text, terminal, whole_screen};
use super::*;
use crate::scroll::ScrollBack;

/// The shape of a real console: output grows downwards and the prompt follows
/// it. Pinned to the screen bottom instead, the prompt left a band of dead
/// black between the last line of output and where the next command is typed.
#[test]
fn the_prompt_sits_on_the_row_after_the_newest_line() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    app.console.record_output("earlier output");
    app.view = ViewMode::Console;

    let mut terminal = terminal();
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let screen = screen_text(&terminal);
    assert_eq!(row_text(&terminal, 0), "earlier output");
    assert!(row_text(&terminal, 1).starts_with('●'), "{screen}");
    // Nothing below it, all the way down: no hint, no second prompt.
    for y in 2..12 {
        assert_eq!(row_text(&terminal, y), "", "row {y} of {screen}");
    }

    // Typing lands on that same row, not on one eight lines below the output.
    app.set_overlay(Overlay::CommandLine("ls -la".into()));
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let screen = screen_text(&terminal);
    assert!(row_text(&terminal, 1).contains("ls -la"), "{screen}");
    assert_eq!(row_text(&terminal, 0), "earlier output", "{screen}");
}

/// A transcript tall enough to fill the screen pushes the prompt onto the last
/// row — which is all "scrolling" means in a terminal.
#[test]
fn a_full_transcript_pushes_the_prompt_onto_the_last_row() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    for i in 0..40 {
        app.console.record_output(&format!("line {i}"));
    }
    app.view = ViewMode::Console;

    let mut terminal = terminal();
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let screen = screen_text(&terminal);
    // Eleven transcript rows, ending on the newest line, then the prompt.
    assert_eq!(row_text(&terminal, 0), "line 29", "{screen}");
    assert_eq!(row_text(&terminal, 10), "line 39", "{screen}");
    assert!(row_text(&terminal, 11).starts_with('●'), "{screen}");
}

/// An empty console is a fresh terminal: a prompt on the top row and nothing
/// else. It used to explain itself in a sentence, which no console does.
#[test]
fn an_empty_console_is_just_a_prompt_on_the_first_row() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    app.view = ViewMode::Console;

    let mut terminal = terminal();
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let screen = screen_text(&terminal);
    assert!(row_text(&terminal, 0).starts_with('●'), "{screen}");
    for y in 1..12 {
        assert_eq!(row_text(&terminal, y), "", "row {y} of {screen}");
    }
}

/// Completion has to stay usable when the prompt is at the very top of a fresh
/// console, where the popup's usual home above it does not exist.
#[test]
fn the_completion_popup_drops_below_a_prompt_with_nothing_above_it() {
    use crate::lineedit::LineEditor;
    use crate::models::{Candidate, CandidateKind, CommandLine, Menu, Recall};

    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    app.view = ViewMode::Console;
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("m"),
        menu: Menu::new(vec![Candidate {
            text: "mark".to_owned(),
            kind: CandidateKind::Word,
            hint: None,
        }]),
        ghost: None,
        recall: Recall::Idle,
    }));

    let mut terminal = terminal();
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let screen = screen_text(&terminal);
    assert!(
        screen.contains("mark"),
        "candidates must still show: {screen}"
    );
}

/// Scrolled back and then typing: a terminal jumps to the bottom, because the
/// prompt is down there and typing at a prompt you cannot see is typing blind.
/// The scrollback note must not be left holding the row the prompt needs.
#[test]
fn typing_after_scrolling_back_returns_to_the_prompt() {
    let temp = TempDir::new().expect("temp dir");
    let mut app = app_in(&temp);
    for i in 0..40 {
        app.console.record_output(&format!("line {i}"));
    }
    app.view = ViewMode::Console;
    app.console.scroll_up(6);

    press(&mut app, KeyCode::Char('p'));
    assert_eq!(app.console.scroll(), ScrollBack::BOTTOM);
    assert_eq!(app.get_command_line(), Some("p"));

    let mut terminal = terminal();
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let screen = screen_text(&terminal);
    assert_eq!(row_text(&terminal, 10), "line 39", "{screen}");
    assert!(row_text(&terminal, 11).contains('●'), "{screen}");
    assert!(!screen.contains("more line(s) below"), "{screen}");
}

// ─── the console hint, and the command row while a search is running ────────

/// Scrolled back, the prompt has scrolled off the bottom the way it does in a
/// terminal, and the row it vacated says how far down the newest line is — the
/// one thing a scrollbar would show and this console cannot.
#[test]
fn scrolling_back_swaps_the_prompt_for_a_scrollback_note() {
    let mut app = App::new().expect("app");
    for line in 0..50 {
        app.console.record_output(&format!("line {line}"));
    }
    app.view = app.view.toggled();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");

    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let at_bottom = whole_screen(&terminal);
    // At the bottom there is nothing to say, so nothing is said.
    assert!(!at_bottom.contains("more line(s) below"), "{at_bottom}");
    assert!(row_text(&terminal, 23).starts_with('●'), "{at_bottom}");

    app.console.scroll_up(5);
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    let scrolled = whole_screen(&terminal);
    assert!(
        scrolled.contains("5 more line(s) below"),
        "scrolled back, it says how much is below: {scrolled}"
    );
    assert!(
        !row_text(&terminal, 23).starts_with('●'),
        "the prompt has scrolled off: {scrolled}"
    );
}

/// A reverse search replaces the whole prompt with bash's banner, so the user
/// can see what is being matched instead of a path that is now irrelevant.
#[test]
fn a_reverse_search_replaces_the_prompt_with_its_own_banner() {
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    use crate::models::{CommandLine, Overlay};

    let mut app = App::new().expect("app");
    app.history.record("cargo test");
    app.set_overlay(Overlay::CommandLine(CommandLine::default()));
    crate::popup::process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('r'), KeyModifiers::CONTROL),
    );
    crate::popup::process_popup_input(
        &mut app,
        KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()),
    );

    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
    terminal.draw(|f| ui(f, &mut app)).expect("draw");

    assert!(
        whole_screen(&terminal).contains("reverse-i-search"),
        "the banner is on the command row"
    );
}
