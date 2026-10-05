//! Panel menu modes and their advertised shortcuts share real state transitions.

use std::io;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::{action, models::Panel, models::PanelView};
use tempfile::TempDir;

#[test]
fn every_panel_mode_has_a_distinct_control_digit_binding() -> io::Result<()> {
    let directory = TempDir::new()?;
    let panel = Panel::new(directory.path().to_path_buf())?;
    assert_eq!(panel.view, PanelView::Full);
    for (index, mode) in PanelView::ALL.into_iter().enumerate() {
        let digit = char::from(b'0' + u8::try_from((index + 1) % 10).unwrap());
        let key = KeyEvent::new(KeyCode::Char(digit), KeyModifiers::CONTROL);
        assert_eq!(action::resolve(key), action::Action::PanelView(mode));
        assert!(!mode.label().is_empty());
    }
    Ok(())
}
