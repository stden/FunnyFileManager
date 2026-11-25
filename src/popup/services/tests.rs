//! Purpose: Key routing unit tests for Jira, HTTP MCP services, and AI assistant overlays.

mod ai;
mod jira;
mod mcp_remote;

use super::{edit_active_line, motion_for, process};
use crate::models::{App, Panel};
use crossterm::event::{KeyCode, KeyModifiers};
use std::path::Path;
use tempfile::TempDir;

pub(super) fn app_in(dir: &Path) -> App {
    let panel = Panel::new(dir.to_path_buf()).expect("panel");
    App::with_panels(panel.clone(), panel)
}

pub(super) fn press(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    assert!(process(app, code, modifiers));
}

#[test]
fn unrelated_motion_and_missing_active_line_do_nothing() {
    assert_eq!(motion_for(KeyCode::F(12)), None);
    let temp = TempDir::new().expect("temp");
    let mut app = app_in(temp.path());
    edit_active_line(&mut app, KeyCode::Char('a'), KeyModifiers::NONE, |_| None);
}
