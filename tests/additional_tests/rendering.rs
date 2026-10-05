//! Purpose: Rendering the command line, overlays and console mode, and centered rectangles.

use super::*;

// ---------------------------------------------------------------------------
// UI rendering: overlay interactions
// ---------------------------------------------------------------------------

fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buf = terminal.backend().buffer();
    let width = buf.area.width;
    let mut s = String::new();
    for x in 0..width {
        s.push_str(buf[(x, y)].symbol());
    }
    s.trim_end().to_owned()
}

fn all_text(terminal: &Terminal<TestBackend>) -> String {
    let height = terminal.backend().buffer().area.height;
    let mut text = String::new();
    for y in 0..height {
        text.push_str(&row_text(terminal, y));
        text.push('\n');
    }
    text
}

#[test]
fn test_ui_with_command_line_active() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::CommandLine("cd /tmp".into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let text = all_text(&terminal);
    assert!(text.contains("cd /tmp"), "should show typed command");
    // The caret is a reversed cell rather than a `_` glyph, so its presence is
    // a question about styling, not about the symbols on the row.
    let buf = terminal.backend().buffer();
    assert!(
        (0..buf.area.width).any(|x| buf[(x, 22)].bg == Color::Yellow),
        "should show the caret when active"
    );
}

#[test]
fn test_ui_with_command_line_inactive() {
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.close_overlay();

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let cmd = row_text(&terminal, 22);
    // Far's command line is always drawn, whether or not it is taking input.
    // The sigil is `#` or `$` depending on the euid, so a suite run as either
    // user passes.
    assert!(
        cmd.contains('#') || cmd.contains('$'),
        "prompt should be visible even when inactive, got: '{cmd}'"
    );
}

#[test]
fn test_ui_help_overlay_covers_panels() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::Help);

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let text = all_text(&terminal);
    assert!(text.contains("F1: Toggle this help"));
    assert!(text.contains("F5: Copy"));
    assert!(text.contains("F8: Delete"));
}

#[test]
fn test_ui_delete_confirm_overlay() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::DeleteConfirm(vec![PathBuf::from("important.txt")]));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let text = all_text(&terminal);
    // Far's wording: the dialog is titled by the operation and names the file
    // rather than counting it, because a count cannot be checked.
    assert!(text.contains("Delete"), "{text}");
    assert!(text.contains("Do you wish to delete the file?"), "{text}");
    assert!(text.contains("important.txt"), "{text}");
}

#[test]
fn test_ui_new_file_overlay() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::NewFileInput("test.rs".to_owned().into()));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    let text = all_text(&terminal);
    assert!(text.contains("Create New File"));
    assert!(text.contains("test.rs"));
}

#[test]
fn test_ui_viewer_bar_names_the_viewer() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "notes.txt",
        vec!["Line 1".to_owned()],
    )));

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // Far's viewer has no frame and no hint: row 0 is the status line, with
    // the title flush left and the clock flush right.
    let bar = row_text(&terminal, 0);
    assert!(bar.starts_with("notes.txt"), "{bar:?}");
    assert!(bar.ends_with(" AM") || bar.ends_with(" PM"), "{bar:?}");
    assert_eq!(row_text(&terminal, 1), "Line 1");
}

#[test]
fn test_ui_console_mode_renders_cleanly() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut app = App::new().unwrap();
    app.view = ViewMode::Console;

    terminal.draw(|f| ui(f, &mut app)).unwrap();

    // A fresh console is a fresh terminal: a prompt on the top row, black
    // everywhere below it.
    let prompt = row_text(&terminal, 0);
    assert!(
        prompt.starts_with('●'),
        "Row 0 should carry the prompt, got: '{prompt}'"
    );
    for y in 1..24 {
        let row = row_text(&terminal, y);
        assert!(
            row.trim().is_empty(),
            "Row {y} should be blank in console mode, got: '{row}'"
        );
    }
}

// ---------------------------------------------------------------------------
// centered_rect edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_centered_rect_zero_size() {
    let base = ratatui::layout::Rect::new(0, 0, 0, 0);
    let rect = centered_rect(50, 50, base);
    assert_eq!(rect.width, 0);
    assert_eq!(rect.height, 0);
}

#[test]
fn test_centered_rect_full_size() {
    let base = ratatui::layout::Rect::new(0, 0, 80, 24);
    let rect = centered_rect(100, 100, base);
    assert_eq!(rect.width, 80);
    assert_eq!(rect.height, 24);
}
