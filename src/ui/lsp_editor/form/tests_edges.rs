//! Purpose: A server form on a screen too small to place it draws nothing and
//! clears the mouse map, and a field with no text line shows no text.

use ratatui::Terminal;

use super::*;
use crate::test_support::WritableTestBackend;

#[test]
fn a_form_too_big_for_the_screen_clears_the_mouse_map() {
    let form = ServerForm::new_server();
    let mut tracker = DialogMouse {
        area: Some(Rect::new(0, 0, 4, 2)),
        ..DialogMouse::default()
    };
    let mut terminal = Terminal::new(WritableTestBackend::new(6, 2)).expect("terminal");
    terminal
        .draw(|f| render(f, &mut tracker, &form))
        .expect("draw");
    assert!(tracker.area.is_none());
}

#[test]
fn a_field_without_a_text_line_shows_no_text() {
    let form = ServerForm::new_server();
    assert_eq!(
        shown(&form, FormField::Enabled, Cells::new(10)),
        (String::new(), String::new())
    );
}
