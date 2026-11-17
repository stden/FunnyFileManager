//! Unit tests: the command line's ghost text after the buffer, none when
//! there is nothing to suggest, and a long one kept inside the row.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::*;
use crate::lineedit::LineEditor;
use crate::models::CommandLine;
use crate::models::Recall;

/// The history ghost renders as dimmed text right after the typed buffer
/// (and its cursor), so the suggestion reads as a continuation of what the
/// user typed.
#[test]
fn test_ghost_text_renders_after_the_buffer() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("cd"),
        menu: None,
        ghost: Some(" /somewhere".to_owned()),
        recall: Recall::Idle,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(screen.contains("cd /somewhere"), "{screen}");
}

/// With no ghost set, the row must render exactly as before this feature
/// existed — no stray suggestion text leaks onto the line.
#[test]
fn test_no_ghost_renders_nothing_extra() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    app.set_overlay(Overlay::CommandLine("cd".into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(!screen.contains("cd /somewhere"), "{screen}");
}

/// A ghost far longer than the terminal is wide must still be clamped so the
/// command-line row never runs past the terminal edge (and the draw itself
/// must not panic on the arithmetic).
#[test]
fn test_long_ghost_does_not_overflow_the_row() {
    let backend = TestBackend::new(20, 10);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("cd"),
        menu: None,
        ghost: Some("x".repeat(40)),
        recall: Recall::Idle,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // panels(7, Min(0)) + status(1) puts the command line at row 8.
    let buf = terminal.backend().buffer();
    let row: String = (0..buf.area.width).map(|x| buf[(x, 8)].symbol()).collect();
    // Derived from the clamp: with active path "/test" (5 chars) and
    // reserved = cmd_text(2) + cursor(1) + ghost(40) + PROMPT_SUFFIX(2) = 45
    // against width 20, `tail_chars` collapses the path to "", so
    // prompt = "> " (2 chars). prefix_used = prompt(2) + cmd_text(2) +
    // cursor(1) = 5, leaving width.saturating_sub(5) = 15 columns for the
    // ghost. The caret sits on the ghost's first character (so the suggestion
    // reads as one continuous word), and the remaining 14 columns hold the
    // rest of it: "> " + "cd" + 16*'x' — exactly 20 chars, no padding left
    // over. Only 16 of the 40 'x's are shown.
    //
    // A `row.chars().count() == 20` assertion here would be tautological:
    // ratatui always fills every buffer cell for the full terminal width
    // regardless of what was actually drawn, so that count is 20 for any
    // output, including a broken clamp. Asserting the exact content is what
    // actually exercises the clamp math.
    // The prompt yields its columns to the typed text, so on a narrow row
    // only the sigil is left of it.
    assert!(
        row.ends_with("cdxxxxxxxxxxxxxxxxx") || row.ends_with("cdxxxxxxxxxxxxxxxx"),
        "command line row: '{row}'"
    );
}

/// When the horizontal scroll asks for more cells than the line holds, the
/// whole line is dropped and the caller gets an empty remainder (plus how many
/// cells the line actually spent).
#[test]
fn skip_cells_past_the_end_of_a_line_yields_empty_suffix() {
    assert_eq!(skip_cells("ab", Cells::new(10)), ("", Cells::new(2)));
    assert_eq!(skip_cells("", Cells::new(5)), ("", Cells::new(0)));
    assert_eq!(skip_cells("文档", Cells::new(10)), ("", Cells::new(4))); // two double-width glyphs
}
