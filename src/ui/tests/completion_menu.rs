//! Unit tests: the command line's completion menu — its candidates, at most
//! ten rows, the hint, a small terminal and a zero-height one.

use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::Terminal;

use super::*;
use crate::lineedit::LineEditor;
use crate::models::Recall;
use crate::models::{Candidate, CandidateKind, CommandLine, Menu};

/// A dropdown over `candidates` with row `index` highlighted.
///
/// `Menu` keeps its selection private so it can never point past the list;
/// walking to the row with `next` is how a test asks for one, and it goes
/// through the same code the `Tab` key does.
fn menu_selecting(candidates: Vec<Candidate>, index: usize) -> Option<Menu> {
    let mut menu = Menu::new(candidates)?;
    for _ in 0..index {
        menu.next();
    }
    Some(menu)
}

// -----------------------------------------------------------------------
// Completion dropdown rendering
// -----------------------------------------------------------------------

#[test]
fn test_open_menu_renders_its_candidates() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("m"),
        menu: Menu::new(vec![
            Candidate {
                text: "mark".to_owned(),
                kind: CandidateKind::Word,
                hint: None,
            },
            Candidate {
                text: "mcp".to_owned(),
                kind: CandidateKind::Word,
                hint: None,
            },
        ]),
        ghost: None,
        recall: Recall::Idle,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(screen.contains("mark"), "{screen}");
    assert!(screen.contains("mcp"), "{screen}");
}

#[test]
fn test_menu_renders_at_most_ten_rows() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    let candidates: Vec<Candidate> = (0..30)
        .map(|i| Candidate {
            text: format!("candidate{i:02}"),
            kind: CandidateKind::Word,
            hint: None,
        })
        .collect();
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("c"),
        // Selection near the end must stay visible, so the window scrolls.
        menu: menu_selecting(candidates, 29),
        ghost: None,
        recall: Recall::Idle,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(
        screen.contains("candidate29"),
        "selection must be visible: {screen}"
    );
    assert!(
        !screen.contains("candidate00"),
        "window should have scrolled: {screen}"
    );
}

#[test]
fn test_candidate_hint_is_rendered() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    // Pin the panel paths so the command-line prompt (`{path}> `) cannot
    // accidentally contain the hint text — this repo's own checkout path
    // starts with the same prefix as a plausible bookmark hint, which would
    // make the assertion pass without the menu ever rendering.
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("bookmark go "),
        menu: Menu::new(vec![Candidate {
            text: "3".to_owned(),
            kind: CandidateKind::Word,
            hint: Some("/srv/FunnyFileManager".to_owned()),
        }]),
        ghost: None,
        recall: Recall::Idle,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(screen.contains("/srv/FunnyFileManager"), "{screen}");
}

#[test]
fn test_menu_on_small_terminal_keeps_selection_visible() {
    // 8 rows total: panels(5, Min(0)) + status(1) + cmdline(1) + fkeybar(1).
    // The command line sits at y = 6, leaving only 4 rows above
    // it, so after the menu's own 2 border rows only 2 content rows fit —
    // far fewer than the 10-candidate list below. The scroll window must be
    // sized from that clamped space, not from the uncapped MENU_MAX_ROWS, or
    // the selected row gets silently dropped off the truncated end.
    let backend = TestBackend::new(40, 8);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/test");
    app.right_panel.path = PathBuf::from("/test2");
    let candidates: Vec<Candidate> = (0..10)
        .map(|i| Candidate {
            text: format!("candidate{i:02}"),
            kind: CandidateKind::Word,
            hint: None,
        })
        .collect();
    app.set_overlay(Overlay::CommandLine(CommandLine {
        line: LineEditor::new("c"),
        menu: menu_selecting(candidates, 9),
        ghost: None,
        recall: Recall::Idle,
    }));

    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let screen = terminal.backend().to_string();

    assert!(
        screen.contains("candidate09"),
        "selected candidate must stay visible even when the popup is \
         clamped by a small terminal: {screen}"
    );
    assert!(
        !screen.contains("candidate00"),
        "window should have scrolled past the start: {screen}"
    );
}

/// On a terminal with no room above the command line, the menu is skipped
/// rather than drawing a zero-height popup.
#[test]
fn completion_menu_on_zero_height_terminal_is_skipped() {
    // Tiny: chrome leaves no rows above the command line for the menu.
    let backend = TestBackend::new(40, 4);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut app = App::new().unwrap();
    app.left_panel.path = PathBuf::from("/t");
    app.right_panel.path = PathBuf::from("/u");
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

    terminal.draw(|f| ui(f, &mut app)).unwrap();
}
