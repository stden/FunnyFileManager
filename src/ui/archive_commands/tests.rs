//! Unit tests: the Archive commands menu draws every row, marks the one under
//! the cursor, and records where it landed for the mouse.

use ratatui::Terminal;

use crate::models::{App, ArchiveCommand, ArchiveMenu, Overlay};
use crate::test_support::{buffer_text, WritableTestBackend};
use crate::ui::{palette, render_overlays};

fn draw(cursor: usize) -> (String, Terminal<WritableTestBackend>, App) {
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("test terminal");
    let mut app = App::new().expect("test app");
    app.set_overlay(Overlay::ArchiveCommands(ArchiveMenu { cursor }));
    terminal
        .draw(|f| render_overlays(f, &mut app))
        .expect("draw");
    let text = buffer_text(&terminal);
    (text, terminal, app)
}

#[test]
fn every_row_is_drawn_with_its_hotkey_under_fars_title() {
    let (screen, _, app) = draw(0);
    assert!(screen.contains("Archive commands"), "{screen}");
    for command in ArchiveCommand::ALL {
        let row = format!(
            " {} {}",
            command.hotkey().to_ascii_uppercase(),
            command.label()
        );
        assert!(
            screen.contains(&row),
            "hotkey column for {command:?}: {screen}"
        );
    }
    assert!(screen.contains("Enter  Esc"), "{screen}");
    assert!(app.dialog_mouse.area.is_some(), "the frame was tracked");
    assert!(app.dialog_mouse.body.is_some(), "the rows were tracked");
}

#[test]
fn the_row_under_the_cursor_is_the_one_inverted() {
    let (screen, terminal, _) = draw(2);
    let buffer = terminal.backend().buffer();
    let row_of = |label: &str| {
        screen
            .lines()
            .position(|line| line.contains(label))
            .unwrap_or_else(|| panic!("{label} is drawn: {screen}"))
    };
    let column_of = |label: &str| {
        let line = screen.lines().nth(row_of(label)).expect("row");
        line.find(label).expect("label column")
    };
    let bg = |label: &str| {
        let (x, y) = (column_of(label), row_of(label));
        buffer[(u16::try_from(x).expect("x"), u16::try_from(y).expect("y"))]
            .style()
            .bg
    };
    assert_eq!(bg("Extract here"), Some(palette::MENU_SELECTED_BG));
    assert_eq!(bg("Test archive"), Some(palette::MENU_BG));
    assert_eq!(bg("Extract to other panel"), Some(palette::MENU_BG));
}
