//! Unit tests: the command line on an 80×24 terminal — typed text kept in
//! view on a long or double-width path, a long command windowed, the ghost
//! hidden behind a mid-line caret, and the prompt's colours.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::render_widths::{blank_app, caret_column, row_text};
use super::*;

/// A deep path must not push the typed command and cursor off the row.
#[test]
fn test_command_line_keeps_typed_text_visible_on_long_path() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.path = PathBuf::from(format!("/{}", vec!["component"; 30].join("/")));
    app.set_overlay(Overlay::CommandLine("ls".into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    assert!(ends_with_prompt(&cmd_row, "ls"), "got: '{cmd_row}'");
    // The path yields first and from its head — but the segments before it
    // survive, so the cut is inside the row rather than at column 0.
    assert!(
        cmd_row.contains('…'),
        "path should be cut, got: '{cmd_row}'"
    );
    // The caret sits one cell past the typed text, still inside the row.
    assert_eq!(
        caret_column(&terminal, 22),
        Some(u16::try_from(cmd_row.chars().count()).expect("row fits u16")),
        "the caret must stay on the row, got: '{cmd_row}'"
    );
}

/// A deep CJK path must not push the prompt, typed text and cursor off the row.
#[test]
fn test_command_line_survives_a_double_width_path() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.path = PathBuf::from(format!("/{}", vec!["文档目录"; 30].join("/")));
    app.set_overlay(Overlay::CommandLine("ls".into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    assert!(ends_with_prompt(&cmd_row, "ls"), "got: '{cmd_row}'");
    let caret = caret_column(&terminal, 22).expect("painted command-line caret");
    assert_eq!(
        terminal.get_cursor_position().unwrap(),
        ratatui::layout::Position::new(caret, 22),
        "the terminal cursor must follow the cell-measured caret"
    );
}

/// A command longer than the terminal is wide must keep its tail and cursor:
/// the typed text is the one thing the user cannot read anywhere else.
#[test]
fn test_command_line_windows_a_command_longer_than_the_row() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    app.left_panel.path = PathBuf::from("/tmp");
    let command = format!("cd {}", "a".repeat(117));
    app.set_overlay(Overlay::CommandLine(command.as_str().into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    assert!(
        cmd_row.ends_with('a'),
        "the command tail must stay on the row, got: '{cmd_row}'"
    );
    assert_eq!(
        caret_column(&terminal, 22),
        Some(u16::try_from(cmd_row.chars().count()).expect("row fits u16")),
        "the caret must stay on the row, got: '{cmd_row}'"
    );
    assert!(
        cmd_row.starts_with("# …") || cmd_row.starts_with("$ …"),
        "the command should be windowed from its head, got: '{cmd_row}'"
    );
}

/// Whether the row ends with a shell prompt and `text`.
///
/// `#` or `$` depending on the euid, so a suite run as either passes.
fn ends_with_prompt(row: &str, text: &str) -> bool {
    row.ends_with(&format!("# {text}")) || row.ends_with(&format!("$ {text}"))
}

/// With the caret parked mid-line, the ghost suggestion is not offered — the
/// accept gesture only makes sense at the end of the line — and the run after
/// the caret comes from the text that was typed there, not from the ghost.
#[test]
fn command_line_mid_caret_hides_the_ghost_and_shows_the_typed_tail() {
    use crate::models::CommandLine;

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let (mut app, _temp) = blank_app();
    let mut state = CommandLine::from("ls file.txt");
    // Split the line so the caret sits between "ls " and "file.txt" instead
    // of at the end, where `at_end()` would offer the ghost.
    state.line.set_split("ls ", "file.txt");
    state.ghost = Some("ignored suggestion".to_owned());
    app.set_overlay(Overlay::CommandLine(state));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    assert!(
        cmd_row.contains("file.txt"),
        "the typed tail after the caret must still show: '{cmd_row}'"
    );
    assert!(
        !cmd_row.contains("ignored suggestion"),
        "the ghost must not show while the caret sits mid-line: '{cmd_row}'"
    );
}

/// `Part::At` gets its own colour, distinct from the user and host segments
/// it sits between — the one part of the prompt whose whole job is to be
/// legible as a separator rather than as either name.
#[test]
fn prompt_spans_colours_the_at_between_user_and_host() {
    use crate::prompt::{Part, Segment};

    let prompt = vec![
        Segment {
            text: "root".to_owned(),
            part: Part::User,
        },
        Segment {
            text: "@".to_owned(),
            part: Part::At,
        },
        Segment {
            text: "box".to_owned(),
            part: Part::Host,
        },
    ];

    let spans = prompt_spans(&prompt);
    let at = spans
        .iter()
        .find(|span| span.content.as_ref() == "@")
        .expect("the '@' segment is drawn");
    assert_eq!(at.style.fg, Some(palette::PROMPT_AT));
}
