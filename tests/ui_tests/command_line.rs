//! Purpose: Command-line row: prompt, typed text, cursor and the path it follows.

use super::*;

/// Column of the command-line caret on row `y`, found by its background.
fn caret_column(terminal: &Terminal<TestBackend>, y: u16) -> Option<u16> {
    let width = terminal.backend().buffer().area.width;
    (0..width).find(|&x| cell_bg(terminal, x, y) == Color::Yellow)
}

#[test]
fn test_command_line_row_exists() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Command line is at row 22 (chunks[2] on 24-row terminal):
    // panels=21, status=1, cmdline=1, fkeybar=1
    // Row 22 should contain the ">" prompt
    let cmd_row = row_text(&terminal, 22);
    assert!(
        has_prompt(&cmd_row),
        "Command line row should carry a shell prompt, got: '{cmd_row}'"
    );
}

#[test]
fn test_command_line_row_has_black_bg() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Command line row 22 should have Black background
    let bg = cell_bg(&terminal, 0, 22);
    assert_eq!(
        bg,
        Color::Black,
        "Command line background should be Black, got: {bg:?}"
    );
}

#[test]
fn test_command_line_shows_active_path() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.left_panel.path = PathBuf::from("/home/user");
    app.active_panel = PanelSide::Left;

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    assert!(
        cmd_row.contains("/home/user"),
        "Command line should show active panel path, got: '{cmd_row}'"
    );
}

#[test]
fn test_command_line_shows_typed_text() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.set_overlay(Overlay::CommandLine("ls -la".into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    assert!(
        cmd_row.contains("ls -la"),
        "Command line should show typed command text, got: '{cmd_row}'"
    );
}

/// The caret is a reversed cell, not a `_` glyph — it has to stay legible when
/// it sits *on* a character rather than after the last one. The terminal cursor
/// occupies that same cell, so it cannot disappear between redraws.
#[test]
fn test_command_line_shows_cursor_when_active() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    // Activate command line (like pressing ':')
    app.set_overlay(Overlay::CommandLine(String::new().into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let caret = caret_column(&terminal, 22).expect("active command line caret");
    assert_eq!(
        terminal.get_cursor_position().unwrap(),
        ratatui::layout::Position::new(caret, 22),
        "Terminal cursor should occupy the painted caret cell"
    );
}

#[test]
fn test_command_line_cursor_stays_visible_when_inactive() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    // command_line is None (inactive)

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd_row = row_text(&terminal, 22);
    // The prompt and its cursor stay visible before the first character is
    // typed, just as they do after the command-line editor becomes active.
    assert!(
        has_prompt(&cmd_row),
        "Inactive command line should show prompt, got: '{cmd_row}'"
    );
    let caret = caret_column(&terminal, 22).expect("idle command line caret");
    assert_eq!(
        terminal.get_cursor_position().unwrap(),
        ratatui::layout::Position::new(caret, 22),
        "Terminal cursor should stay on the idle command line"
    );
}

#[test]
fn test_command_line_path_follows_active_panel() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = test_app();
    app.left_panel.path = PathBuf::from("/left");
    app.right_panel.path = PathBuf::from("/right");

    // Left active
    app.active_panel = PanelSide::Left;
    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let cmd = row_text(&terminal, 22);
    assert!(
        cmd.contains("/left"),
        "Command line should show left path when left active, got: '{cmd}'"
    );

    // Right active
    app.active_panel = PanelSide::Right;
    terminal.draw(|f| ui(f, &mut app)).unwrap();
    let cmd = row_text(&terminal, 22);
    assert!(
        cmd.contains("/right"),
        "Command line should show right path when right active, got: '{cmd}'"
    );
}
