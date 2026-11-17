//! Purpose: Panel-specific interpreters never fall through into an unrelated user binding.

use super::*;
use crate::keymap::Context;
use crate::models::FolderTree;

#[test]
fn tree_commands_use_the_canonical_interpreter_with_unbound_keys() {
    let mut fixture = Fixture::new();
    fixture.app.left_panel.kind = PanelKind::Tree(Box::new(FolderTree::rooted_at(
        fixture.root.path().to_path_buf(),
    )));
    fixture.app.keymap.tree_panel = crate::action::Bindings::default();
    assert_eq!(current_context(&fixture.app), Some(Context::TreePanel));
    fixture.run("tree_panel", "expand").unwrap();
    fixture.run("tree_panel", "cursor_down").unwrap();
    let PanelKind::Tree(tree) = &fixture.app.left_panel.kind else {
        panic!("tree closed")
    };
    assert_eq!(tree.cursor, 1);
    fixture.run("panels", "switch_panel").unwrap();
    assert_eq!(fixture.app.active_panel, crate::models::PanelSide::Right);
}

#[test]
fn sessions_context_dispatches_session_actions_instead_of_local_file_actions() {
    let mut fixture = Fixture::new();
    crate::ops::sessions::open(&mut fixture.app);
    assert_eq!(current_context(&fixture.app), Some(Context::NetboxPanel));
    fixture.run("netbox_panel", "new_session").unwrap();
    assert!(matches!(fixture.app.overlay, Overlay::SessionEdit(_)));
    assert_eq!(
        fs::read_to_string(fixture.root.path().join("left/file.txt")).unwrap(),
        "alpha"
    );
}

#[test]
fn quick_view_closing_cannot_fall_through_to_a_user_f24_quit_binding() {
    let mut fixture = Fixture::new();
    fixture.app.left_panel.kind = PanelKind::QuickView(Box::default());
    fixture
        .app
        .keymap
        .panels
        .bind(crate::action::Action::Quit, "F24".parse().unwrap());
    assert_eq!(current_context(&fixture.app), Some(Context::Viewer));
    assert!(fixture.run("viewer", "close").is_err());
    assert!(fixture.app.lifecycle.is_running());
    assert!(matches!(
        fixture.app.left_panel.kind,
        PanelKind::QuickView(_)
    ));
}

#[test]
fn viewer_scroll_uses_preview_interpreter_without_falling_through() {
    let mut fixture = Fixture::new();
    fixture.app.left_panel.kind = PanelKind::QuickView(Box::default());
    fixture
        .app
        .keymap
        .panels
        .bind(crate::action::Action::Quit, "F24".parse().unwrap());
    let original = fixture.app.keymap.clone();
    fixture.run("viewer", "line_down").unwrap();
    assert!(fixture.app.lifecycle.is_running());
    assert_eq!(fixture.app.keymap, original);
}

#[test]
fn console_uses_panels_context_even_when_the_hidden_panel_is_a_tree() {
    let mut fixture = Fixture::new();
    fixture.app.left_panel.kind = PanelKind::Tree(Box::default());
    fixture.app.view = crate::models::ViewMode::Console;
    assert_eq!(current_context(&fixture.app), Some(Context::Panels));
    assert!(fixture.run("tree_panel", "expand").is_err());
}

fn remote_with_file() -> crate::models::RemotePanel {
    use crate::models::{DirEntry, EntryKind, RemotePanel, RemotePath, SessionName};
    use crate::remote::{fake::FakeRemoteFs, RemoteLink};
    let mut remote = RemotePanel::new(
        SessionName::new("fixture-remote").unwrap(),
        RemoteLink::new(Box::new(FakeRemoteFs::default())),
    );
    let shown = remote.show(
        RemotePath::root(),
        vec![DirEntry {
            name: "remote.txt".into(),
            kind: EntryKind::File,
            ..DirEntry::default()
        }],
    );
    assert_eq!(shown.unusable_names, 0);
    remote
}

#[test]
fn remote_named_navigation_changes_remote_cursor_without_touching_local_files() {
    let mut fixture = Fixture::new();
    fixture.app.left_panel.kind = PanelKind::Remote(Box::new(remote_with_file()));
    let local_selection = fixture.app.left_panel.selected();
    assert_eq!(current_context(&fixture.app), Some(Context::NetboxPanel));
    fixture.run("netbox_panel", "cursor_down").unwrap();
    let PanelKind::Remote(remote) = &fixture.app.left_panel.kind else {
        panic!("remote panel closed")
    };
    assert_eq!(remote.selected().unwrap().name, "remote.txt");
    assert_eq!(fixture.app.left_panel.selected(), local_selection);
    assert_eq!(
        fs::read_to_string(fixture.root.path().join("left/file.txt")).unwrap(),
        "alpha"
    );
}

#[test]
fn fullscreen_viewer_named_scroll_restores_bindings_and_close_returns_to_panels() {
    let mut fixture = Fixture::new();
    fixture.app.overlay = Overlay::Viewer(crate::models::Viewer::report(
        "fixture report",
        (0..100).map(|line| format!("line {line}")).collect(),
    ));
    let original = fixture.app.keymap.clone();
    assert_eq!(current_context(&fixture.app), Some(Context::Viewer));
    fixture.run("viewer", "line_down").unwrap();
    let Overlay::Viewer(viewer) = &fixture.app.overlay else {
        panic!("viewer closed before close command")
    };
    assert_eq!(viewer.scroll.lines(), 1);
    assert_eq!(fixture.app.keymap, original);
    fixture.run("viewer", "close").unwrap();
    assert!(matches!(fixture.app.overlay, Overlay::None));
    assert_eq!(current_context(&fixture.app), Some(Context::Panels));
    assert_eq!(fixture.app.keymap, original);
}
