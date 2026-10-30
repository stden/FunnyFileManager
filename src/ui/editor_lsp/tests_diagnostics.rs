//! Purpose: Assert diagnostic underlines: the Unicode range under an emoji,
//! ranges that are invalid or off screen, exclusive ends and point ranges,
//! and the colour of each severity.

use super::underline::{diagnostic_columns, paint};
use super::*;
use crate::lsp::features::Diagnostic;
use crate::lsp::positions::{Position, Range};
use crate::ui::{Cells, EditorLine};
use ratatui::style::Modifier;
use ratatui::{backend::TestBackend, Terminal};
use tempfile::TempDir;

#[test]
fn diagnostics_underlines_the_emoji_range_and_shows_the_message() {
    let root = TempDir::new().unwrap();
    let mut editor = Editor::from_content("main.rs".into(), root.path().join("main.rs"), "a😀z");
    editor.cursor_col = 1;
    editor.lsp.state = ServerState::Ready;
    editor.lsp.diagnostics.push(emoji_diagnostic());
    let mut terminal = Terminal::new(TestBackend::new(20, 5)).unwrap();
    terminal
        .draw(|frame| diagnostics(frame, &editor, frame.area(), 0, Cells::new(0)))
        .unwrap();
    for (column, underlined) in [(1, true), (2, true), (3, false)] {
        assert_eq!(
            terminal.backend().buffer()[(column, 0)]
                .modifier
                .contains(Modifier::UNDERLINED),
            underlined
        );
    }
    assert!(status(&editor).contains("Error: bad type"));
}

fn emoji_diagnostic() -> Diagnostic {
    Diagnostic {
        range: Range {
            start: Position {
                line: 0,
                character: 1,
            },
            end: Position {
                line: 0,
                character: 3,
            },
        },
        message: "bad type".into(),
        severity: Some(1),
    }
}

/// `text` as an editor line with the default tab stops.
fn line(text: &str) -> EditorLine<'_> {
    EditorLine {
        text,
        tab: crate::models::TabSize::DEFAULT,
    }
}

#[test]
fn invalid_and_offscreen_diagnostics_do_not_paint_other_text() {
    let mut diagnostic = emoji_diagnostic();
    assert_eq!(
        diagnostic_columns(&diagnostic, 1, line("plain"), Cells::new(0), 8),
        None
    );
    assert_eq!(
        diagnostic_columns(&diagnostic, 0, line("a😀z"), Cells::new(8), 8),
        None
    );
    diagnostic.range.start.character = 2;
    assert_eq!(
        diagnostic_columns(&diagnostic, 0, line("a😀z"), Cells::new(0), 8),
        None
    );
    diagnostic.range.start.character = 5;
    diagnostic.range.end.character = 5;
    assert_eq!(
        diagnostic_columns(&diagnostic, 0, line("a😀z"), Cells::new(0), 8),
        None
    );
    diagnostic.range.start = Position::default();
    diagnostic.range.end = Position {
        line: 2,
        character: 0,
    };
    assert_eq!(
        diagnostic_columns(&diagnostic, 1, line("middle"), Cells::new(0), 3),
        Some(0..3)
    );
}

#[test]
fn diagnostic_messages_stop_at_the_exclusive_end_but_include_point_ranges() {
    let root = TempDir::new().unwrap();
    let mut editor = Editor::from_content("source".into(), root.path().join("source.rs"), "a😀z");
    editor.lsp.state = ServerState::Ready;
    editor.lsp.diagnostics.push(emoji_diagnostic());
    editor.cursor_col = 2;
    assert!(!status(&editor).contains("bad type"));
    editor.lsp.diagnostics[0].range.start = editor.lsp.diagnostics[0].range.end;
    assert!(status(&editor).contains("bad type"));
}

#[test]
fn diagnostic_severity_selects_warning_and_hint_colors() {
    let mut terminal = Terminal::new(TestBackend::new(4, 1)).unwrap();
    terminal
        .draw(|frame| {
            paint(frame, 0, 0, 0..1, Some(2));
            paint(frame, 0, 0, 1..2, Some(4));
        })
        .unwrap();
    assert_eq!(terminal.backend().buffer()[(0, 0)].fg, Color::Yellow);
    assert_eq!(terminal.backend().buffer()[(1, 0)].fg, Color::Cyan);
}
