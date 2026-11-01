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
