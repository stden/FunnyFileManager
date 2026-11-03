//! Purpose: Temp cleanup popup handler returns false when another overlay is active.

use super::process;
use crate::models::{App, Panel};
use crossterm::event::{KeyCode, KeyModifiers};
use std::path::PathBuf;

#[test]
fn process_returns_false_when_temp_cleanup_is_closed() {
    let empty = || Panel::from_entries(PathBuf::from("/tmp"), Vec::new());
    let mut app = App::with_panels(empty(), empty());
    assert!(!process(&mut app, KeyCode::Esc, KeyModifiers::NONE));
}
