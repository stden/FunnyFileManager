//! Purpose: Panel snapshots expose the visible model and keep large-list selections reachable.

use super::*;
use crate::models::{
    DirEntry, EntryKind, FileSize, RemotePanel, RemotePath, RowMotion, SessionName,
};
use crate::remote::{fake::FakeRemoteFs, RemoteLink};
use serde_json::json;
use tempfile::TempDir;

mod images;
mod views;

fn hidden_panel(root: &TempDir) -> Panel {
    let mut panel = Panel::new(root.path().to_path_buf()).unwrap();
    panel.entries = vec![entry("hidden-local-entry", EntryKind::File)].into();
    panel.select(Some(0));
    panel
}

fn entry(name: &str, kind: EntryKind) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind,
        size: FileSize::from_bytes(17),
        ..DirEntry::default()
    }
}

fn remote() -> RemotePanel {
    RemotePanel::new(
        SessionName::new("visible-session").unwrap(),
        RemoteLink::new(Box::new(FakeRemoteFs::default())),
    )
}

#[test]
fn remote_rows_path_marks_and_cursor_come_from_the_remote_listing() {
    let root = TempDir::new().unwrap();
    let mut panel = hidden_panel(&root);
    let mut remote = remote();
    let shown = remote.show(
        RemotePath::parse("/srv").unwrap(),
        vec![
            entry("remote-link", EntryKind::SymlinkToFile),
            entry("remote-file", EntryKind::File),
        ],
    );
    assert_eq!(shown.unusable_names, 0);
    remote.move_cursor(RowMotion::Next);
    remote.toggle_mark();
    panel.kind = PanelKind::Remote(Box::new(remote));
    let value = describe(&panel);
    assert_eq!(value["kind"], "remote");
    assert_eq!(value["path"], "/srv");
    assert_eq!(value["session"], "visible-session");
    assert_eq!(value["total_entries"], 3);
    assert_eq!(value["selected_index"], 2);
    assert_eq!(value["entries"][1]["marked"], true);
    assert_eq!(value["entries"][1]["is_symlink"], true);
    assert_eq!(value["entries"][2]["selected"], true);
    assert!(!value.to_string().contains("hidden-local-entry"));
}

#[test]
fn a_bounded_local_snapshot_contains_a_cursor_beyond_the_first_five_hundred_rows() {
    let root = TempDir::new().unwrap();
    let mut panel = hidden_panel(&root);
    panel.entries = (0..750)
        .map(|at| entry(&format!("file-{at}"), EntryKind::File))
        .collect();
    panel.select(Some(749));
    panel
        .marked
        .insert(crate::models::FileName::new("file-749"));
    let value = describe(&panel);
    let entries = value["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 500);
    assert_eq!(value["total_entries"], 750);
    assert_eq!(value["entries_truncated"], true);
    assert_eq!(value["entry_offset"], 250);
    assert_eq!(value["selected_index"], 749);
    let selected = entries.iter().find(|row| row["selected"] == true).unwrap();
    assert_eq!(selected["name"], "file-749");
    assert_eq!(selected["index"], 749);
    assert_eq!(selected["marked"], true);
}

#[test]
fn a_bounded_remote_snapshot_also_contains_its_actual_last_row() {
    let root = TempDir::new().unwrap();
    let mut panel = hidden_panel(&root);
    let mut remote = remote();
    let shown = remote.show(
        RemotePath::root(),
        (0..600)
            .map(|at| entry(&format!("remote-{at}"), EntryKind::File))
            .collect(),
    );
    assert_eq!(shown.unusable_names, 0);
    remote.move_cursor(RowMotion::Last);
    panel.kind = PanelKind::Remote(Box::new(remote));
    let value = describe(&panel);
    assert_eq!(value["entries"].as_array().unwrap().len(), 500);
    assert_eq!(value["total_entries"], 601);
    assert_eq!(value["selected_index"], 600);
    assert!(value["entries"]
        .as_array()
        .unwrap()
        .iter()
        .any(|row| row["selected"] == true && row["name"] == "remote-599"));
}

#[test]
fn an_empty_local_snapshot_has_no_selected_entry() {
    let root = TempDir::new().unwrap();
    let mut panel = hidden_panel(&root);
    panel.entries.clear();
    panel.select(None);
    let value = describe(&panel);
    assert_eq!(value["entries"], json!([]));
    assert_eq!(value["entry_offset"], 0);
    assert!(value["selected_index"].is_null());
    assert_eq!(value["entries_truncated"], false);
}
