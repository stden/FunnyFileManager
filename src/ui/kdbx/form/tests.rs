//! Purpose: Verify the `.kdbx` edit form skips cursor placement when the screen cannot fit it.

use ratatui::{backend::TestBackend, layout::Position, Terminal};

use super::render;
use crate::kdbx::tests_support::{focus, unlocked_sample};

#[test]
fn tiny_screen_does_not_place_the_edit_form_caret() {
    let (_dir, mut session, now) = unlocked_sample();
    focus(&mut session, "GitHub", now);
    session.begin_edit(now).expect("edit");
    let form = session
        .vault()
        .expect("unlocked")
        .form()
        .expect("edit form");
    let mut terminal = Terminal::new(TestBackend::new(4, 3)).expect("terminal");
    terminal
        .draw(|frame| render(frame, "vault.kdbx", form))
        .expect("draw");
    assert_eq!(
        terminal.get_cursor_position().expect("cursor"),
        Position::new(0, 0)
    );
}
