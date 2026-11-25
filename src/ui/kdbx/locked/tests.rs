//! Purpose: Verify the `.kdbx` unlock prompt skips cursor placement when the screen cannot fit it.

use ratatui::{backend::TestBackend, layout::Position, Terminal};

use super::render_locked;
use crate::kdbx::UnlockPrompt;
use crate::mouse::DialogMouse;

#[test]
fn tiny_screen_does_not_place_the_unlock_caret() {
    let prompt = UnlockPrompt::default();
    let mut tracker = DialogMouse::default();
    let mut terminal = Terminal::new(TestBackend::new(4, 3)).expect("terminal");
    terminal
        .draw(|frame| render_locked(frame, "vault.kdbx", &prompt, &mut tracker))
        .expect("draw");
    assert_eq!(
        terminal.get_cursor_position().expect("cursor"),
        Position::new(0, 0)
    );
}
