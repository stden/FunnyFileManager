//! Purpose: Verify the priority dialog leaves the cursor unset when focus moves off the input field.

use ratatui::{backend::TestBackend, layout::Position, Terminal};

use super::priority;
use crate::models::{FrozenTargets, Niceness, PriorityForm};
use crate::mouse::DialogMouse;

#[test]
fn focused_priority_button_does_not_place_the_text_caret() {
    let mut form = PriorityForm::new(FrozenTargets::default(), Niceness::NORMAL);
    form.focus_next();
    let mut tracker = DialogMouse::default();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
    terminal
        .draw(|frame| priority(frame, &mut tracker, &form))
        .expect("draw");
    assert_eq!(
        terminal.get_cursor_position().expect("cursor"),
        Position::new(0, 0)
    );
}
