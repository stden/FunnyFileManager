//! Unit tests: the action arms and mouse routes the rest of the suite misses.
//!
//! `apply_action` is where every key ends up, so an arm nothing exercises is a
//! command nobody has ever run — including the ones a user reaches for daily.
//! Driven through `dispatch_key` where the arm needs the terminal, and through
//! `apply_action` directly where it does not.

use std::fs;

use crossterm::event::KeyCode;
use ratatui::Terminal;
use tempfile::TempDir;

use super::*;
use crate::models::{Panel, PanelSide};
use crate::WritableTestBackend;

/// An app on a temporary directory, and a terminal to draw it into.
fn app_and_terminal(names: &[&str]) -> (App, Terminal<WritableTestBackend>, TempDir) {
    let dir = TempDir::new().expect("test setup");
    for name in names {
        fs::write(dir.path().join(name), b"x").expect("test setup");
    }
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(dir.path().to_path_buf()).expect("test setup");
    app.right_panel = Panel::new(dir.path().to_path_buf()).expect("test setup");
    let terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("test setup");
    (app, terminal, dir)
}

/// The whole console transcript, for the tests that check what a command
/// printed.
fn console_text(app: &App) -> String {
    app.console
        .visible(usize::MAX)
        .iter()
        .map(|line| line.text())
        .collect::<Vec<_>>()
        .join("\n")
}

fn select_named(panel: &mut Panel, name: &str) {
    let at = panel
        .entries
        .iter()
        .position(|entry| entry.name == name)
        .unwrap_or_else(|| panic!("{name} is in the listing"));
    panel.select(Some(at));
}

fn press(app: &mut App, terminal: &mut Terminal<WritableTestBackend>, code: KeyCode) {
    dispatch_key(terminal, app, key_event(code));
}

mod console_and_command_line;
mod dialog_arms;
mod enter_runs_programs;
mod mouse_hit_testing;
mod mouse_routing;
mod panel_navigation;
mod selection_keys;
