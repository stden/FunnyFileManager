//! Filter details remain visible when only exclusion and dotfile rules apply.

use ratatui::{backend::TestBackend, Terminal};

use super::render_widths::{blank_app, left_panel_text};
use super::*;

#[test]
fn exclusion_and_dotfile_rules_are_named_in_the_panel_footer() {
    let (mut app, _temp) = blank_app();
    app.left_panel.filter.exclude = Some(crate::mask::Mask::parse("*.tmp"));
    app.left_panel.filter.hide_dotfiles = true;
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).expect("terminal");
    terminal.draw(|frame| ui(frame, &mut app)).expect("draw");
    let panel = left_panel_text(&terminal);
    assert!(panel.contains("Filter: !*.tmp no dotfiles"), "{panel}");
}
