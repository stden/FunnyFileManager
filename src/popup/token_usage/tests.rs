//! Purpose: Usage overlay ignores unmapped keys and returns false when closed.

use super::process;
use crate::models::{App, Panel};
use crossterm::event::{KeyCode, KeyModifiers};
use std::path::PathBuf;

#[test]
fn closed_returns_false_and_unmapped_key_is_consumed() {
    let empty = || Panel::from_entries(PathBuf::from("/tmp"), Vec::new());
    let mut app = App::with_panels(empty(), empty());
    assert!(!process(&mut app, KeyCode::Esc, KeyModifiers::NONE));

    crate::ops::token_usage::open(&mut app);
    assert!(process(&mut app, KeyCode::Char('z'), KeyModifiers::NONE));
    assert!(app.is_overlay_active());
}
