//! Purpose: The console draws nothing into a zero-height area and does not panic.

use ratatui::layout::Rect;
use ratatui::Terminal;

use super::render_view;
use crate::models::{App, Panel};
use crate::test_support::{buffer_text, WritableTestBackend};

#[test]
fn a_console_given_no_rows_draws_nothing() {
    let root = tempfile::TempDir::new().expect("test folder");
    let app = App::with_panels(
        Panel::new(root.path().to_owned()).expect("test panel"),
        Panel::new(root.path().to_owned()).expect("test panel"),
    );
    let mut terminal = Terminal::new(WritableTestBackend::new(20, 4)).expect("terminal");
    terminal
        .draw(|f| render_view(f, &app, Rect::new(0, 0, 20, 0)))
        .expect("draw");
    assert_eq!(buffer_text(&terminal).trim(), "");
}
