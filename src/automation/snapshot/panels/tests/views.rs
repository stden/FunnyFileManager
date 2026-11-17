//! Purpose: Virtual panels never publish the local rows hidden underneath them.

use super::*;
use crate::models::{
    FolderTree, HostName, Origin, Port, Preview, Protocol, QuickView, Session, SessionsPanel,
    TextPreview,
};

#[test]
fn sessions_publish_visible_names_and_cursor_without_serializing_private_profile_fields() {
    let root = TempDir::new().unwrap();
    let mut panel = hidden_panel(&root);
    let mut session = Session::new(
        Protocol::Sftp,
        HostName::new("shown.invalid").unwrap(),
        Port::SSH,
        None,
    );
    session.name = SessionName::new("visible-name").unwrap();
    session.origin = Origin::NetBoxFile {
        path: root.path().join("PRIVATE-IMPORT-MARKER"),
    };
    let mut store = crate::sessions::SessionStore::default();
    store.upsert(session);
    let mut sessions = SessionsPanel::from_store(&store);
    sessions.move_cursor(RowMotion::Last);
    panel.kind = PanelKind::Sessions(Box::new(sessions));
    let value = describe(&panel);
    assert_eq!(value["kind"], "sessions");
    assert_eq!(value["entries"][0]["name"], "..");
    assert_eq!(value["entries"][1]["name"], "visible-name");
    assert_eq!(value["entries"][1]["selected"], true);
    assert_eq!(value["selected_index"], 1);
    assert!(!value.to_string().contains("PRIVATE-IMPORT-MARKER"));
    assert!(!value.to_string().contains("hidden-local-entry"));
}

#[test]
fn tree_snapshot_uses_tree_paths_shape_and_cursor() {
    let root = TempDir::new().unwrap();
    let mut panel = hidden_panel(&root);
    let mut tree = FolderTree::rooted_at(root.path().join("shown-root"));
    tree.expand(&mut |path| vec![path.join("branch-a"), path.join("branch-b")]);
    tree.cursor = 2;
    panel.kind = PanelKind::Tree(Box::new(tree));
    let value = describe(&panel);
    assert_eq!(value["kind"], "tree");
    assert_eq!(value["total_entries"], 3);
    assert_eq!(value["entries"][2]["name"], "branch-b");
    assert_eq!(value["entries"][2]["depth"], 1);
    assert_eq!(value["entries"][2]["selected"], true);
    assert_eq!(value["selected_index"], 2);
    assert!(!value.to_string().contains("hidden-local-entry"));
}

#[test]
fn quick_view_publishes_preview_facts_without_a_hidden_local_listing() {
    let root = TempDir::new().unwrap();
    let mut panel = hidden_panel(&root);
    let shown = root.path().join("shown-preview.txt");
    panel.kind = PanelKind::QuickView(Box::new(QuickView {
        shown: Some(shown.clone()),
        preview: Preview::Text(TextPreview::decoded(&shown, b"first\nsecond")),
        ..QuickView::default()
    }));
    let value = describe(&panel);
    assert_eq!(value["entries"], json!([]));
    assert_eq!(value["total_entries"], 0);
    assert!(value["selected_index"].is_null());
    assert_eq!(value["preview"]["kind"], "text");
    assert_eq!(value["preview"]["line_count"], 2);
    assert_eq!(value["path"], shown.to_string_lossy().as_ref());
    assert!(!value.to_string().contains("hidden-local-entry"));
}

#[test]
fn quick_view_reports_empty_folder_and_refused_previews_as_their_own_states() {
    let root = TempDir::new().unwrap();
    let mut panel = hidden_panel(&root);
    for (preview, kind) in [
        (Preview::Nothing, "nothing"),
        (Preview::Folder(Box::default()), "folder"),
        (Preview::Refused(crate::models::Refusal::Binary), "refused"),
    ] {
        panel.kind = PanelKind::QuickView(Box::new(QuickView {
            preview,
            ..QuickView::default()
        }));
        let value = describe(&panel);
        assert_eq!(value["preview"]["kind"], kind);
        assert!(value["path"].is_null());
        assert_eq!(value["entries"], json!([]));
    }
}
