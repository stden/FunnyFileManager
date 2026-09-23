//! Unit tests: F11 opens the list, and each row does its one thing after the
//! list has closed.

use tempfile::TempDir;

use super::{open, run};
use crate::models::{InputPurpose, Overlay, PluginCommand};
use crate::test_support::netbox::app_in;

#[test]
fn f11_opens_the_list_on_its_first_row() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    open(&mut app);
    let Overlay::PluginCommands(list) = &app.overlay else {
        panic!("the plugin commands list");
    };
    assert_eq!(list.selected(), PluginCommand::McpStatus);
}

#[test]
fn each_row_runs_after_the_list_has_closed() {
    let dir = TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    open(&mut app);
    run(&mut app, PluginCommand::McpStatus).expect("the MCP screen");
    assert_eq!(
        app.get_viewer().map(crate::models::Viewer::title),
        Some("MCP Server Status")
    );

    open(&mut app);
    run(&mut app, PluginCommand::Sessions).expect("the sessions panel");
    assert!(!app.is_overlay_active());
    assert!(app.left_panel.kind.is_netbox());

    open(&mut app);
    run(&mut app, PluginCommand::ImportNetBox).expect("the import prompt");
    let Overlay::InputPrompt(prompt) = &app.overlay else {
        panic!("the import asks for its file");
    };
    assert_eq!(prompt.purpose(), &InputPurpose::ImportNetBox);
}

#[test]
fn every_row_has_its_label() {
    let labels: Vec<&str> = PluginCommand::ALL.iter().map(|row| row.label()).collect();
    assert_eq!(
        labels,
        [
            "MCP status",
            "NetBox: stored sessions",
            "NetBox: import from .netbox file…"
        ]
    );
}
