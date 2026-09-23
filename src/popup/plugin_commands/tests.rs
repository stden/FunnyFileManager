//! Unit tests: F11's list walks its rows, runs one on Enter, closes on Esc
//! and F10, and swallows every other key.

use crossterm::event::KeyCode;
use tempfile::TempDir;

use crate::models::{App, Overlay, PluginCommand};
use crate::popup::process_popup_input;
use crate::popup::tests::key;
use crate::test_support::netbox::app_in;

fn cursor(app: &App) -> usize {
    match &app.overlay {
        Overlay::PluginCommands(list) => list.cursor(),
        _ => panic!("not the plugin commands list"),
    }
}

#[test]
fn esc_and_f10_close_it_and_other_keys_are_swallowed() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    for closing in [KeyCode::Esc, KeyCode::F(10)] {
        crate::ops::plugin_commands::open(&mut app);
        assert!(process_popup_input(&mut app, key(KeyCode::Char('x'))));
        assert!(matches!(app.overlay, Overlay::PluginCommands(_)));
        assert!(process_popup_input(&mut app, key(closing)));
        assert!(!app.is_overlay_active(), "{closing:?}");
    }
}

#[test]
fn the_list_keys_walk_the_rows_within_the_table() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    crate::ops::plugin_commands::open(&mut app);
    process_popup_input(&mut app, key(KeyCode::Up));
    assert_eq!(cursor(&app), 0);
    process_popup_input(&mut app, key(KeyCode::End));
    assert_eq!(cursor(&app), PluginCommand::ALL.len() - 1);
    process_popup_input(&mut app, key(KeyCode::Down));
    assert_eq!(cursor(&app), PluginCommand::ALL.len() - 1);
    process_popup_input(&mut app, key(KeyCode::PageUp));
    process_popup_input(&mut app, key(KeyCode::Down));
    assert_eq!(cursor(&app), 1);
}

#[test]
fn enter_runs_the_row_under_the_cursor() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    crate::ops::plugin_commands::open(&mut app);
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert_eq!(
        app.get_viewer().map(crate::models::Viewer::title),
        Some("MCP Server Status"),
        "the first row is what F11 used to open"
    );

    crate::ops::plugin_commands::open(&mut app);
    process_popup_input(&mut app, key(KeyCode::Down));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(!app.is_overlay_active(), "the list closes first");
    assert!(app.left_panel.kind.is_netbox());

    crate::ops::plugin_commands::open(&mut app);
    process_popup_input(&mut app, key(KeyCode::End));
    process_popup_input(&mut app, key(KeyCode::Enter));
    assert!(matches!(app.overlay, Overlay::InputPrompt(_)));
}

/// Asked about another dialog's key, this one says it is not its own.
#[test]
fn another_overlay_is_not_this_dialogs() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    assert!(!super::process(
        &mut app,
        KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE
    ));
    assert!(!app.is_overlay_active());
}
