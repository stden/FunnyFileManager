//! Purpose: Pins the Far-style console layer — its backdrop, colours, and Ctrl+O view.

use super::*;
use crate::models::ViewMode;
use crate::scroll::ScrollBack;

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

// ─── keys, through the main-view dispatcher ───────────────────────────────

/// Console mode is a view, not an overlay, so its keys go through the same
/// `Action` path the panels use. Drive them exactly as the event loop does.
fn press(app: &mut App, code: KeyCode) {
    press_with(app, KeyEvent::new(code, KeyModifiers::empty()));
}

fn press_with(app: &mut App, event: KeyEvent) {
    let mut terminal = Terminal::new(crate::WritableTestBackend::new(60, 12)).expect("terminal");
    crate::application::dispatch_key(&mut terminal, app, event);
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

/// The width type prints as its number, which is what a panic message about a
/// layout that would not fit has to carry.
#[test]
fn the_cell_count_prints_as_its_number() {
    assert_eq!(Cells::new(17).to_string(), "17");
}

fn whole_screen(terminal: &Terminal<TestBackend>) -> String {
    crate::test_support::buffer_string(terminal.backend().buffer())
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
