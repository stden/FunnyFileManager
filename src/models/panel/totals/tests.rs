//! Purpose: Totals stay current after public collection edits and panel transitions.

use super::*;
use crate::models::{DirectoryListing, EntryKind, FileName, PanelMetadata};
use std::path::PathBuf;

fn panel() -> Panel {
    let file = DirEntry {
        name: "file".to_owned(),
        size: FileSize::from_bytes(10),
        ..DirEntry::default()
    };
    let folder = DirEntry {
        name: "folder".to_owned(),
        kind: EntryKind::Directory,
        ..DirEntry::default()
    };
    Panel::from_entries(PathBuf::from("/srv/x"), vec![file, folder])
}

#[test]
fn moving_the_cursor_reuses_both_totals_without_scanning_entries() {
    let mut panel = panel();
    panel.mark_all();
    for index in 0..100 {
        panel.select(Some(index % 2));
        assert_eq!(panel.file_totals(), (1, 10));
        assert_eq!(panel.marked_totals(), (1, 1, 10));
    }
    assert_eq!(panel.totals.file_scans.get(), 1);
    assert_eq!(panel.totals.mark_scans.get(), 1);
    panel.toggle_mark_selected();
    panel.file_totals();
    panel.marked_totals();
    assert_eq!(panel.totals.file_scans.get(), 1);
    assert_eq!(panel.totals.mark_scans.get(), 2);
}

#[test]
fn totals_follow_in_place_edits_and_collection_replacement() {
    let mut panel = panel();
    assert_eq!(panel.file_totals(), (1, 10));
    assert_eq!(panel.file_totals(), (1, 10));
    panel.entries[0].size = FileSize::from_bytes(25);
    assert_eq!(panel.file_totals(), (1, 25));
    panel.entries.clear();
    assert_eq!(panel.file_totals(), (0, 0));
    panel.entries = vec![DirEntry::default()].into();
    assert_eq!(panel.file_totals(), (1, 0));
}

#[test]
fn selected_totals_follow_marks_sizes_and_restore() {
    let mut panel = panel();
    assert_eq!(panel.marked_totals(), (0, 0, 0));
    panel.toggle_mark_selected();
    assert_eq!(panel.marked_totals(), (0, 1, 10));
    panel.marked.insert(FileName::from("folder"));
    panel.set_directory_sizes(&[("folder".to_owned(), FileSize::from_bytes(50))]);
    assert_eq!(panel.marked_totals(), (1, 1, 60));
    panel.clear_marks();
    assert_eq!(panel.marked_totals(), (0, 0, 0));
    panel.restore_marks();
    assert_eq!(panel.marked_totals(), (1, 1, 60));
    panel.entries[0].size = FileSize::from_bytes(20);
    assert_eq!(panel.marked_totals(), (1, 1, 70));
}

#[test]
fn clones_and_reload_do_not_reuse_stale_totals() {
    let mut panel = panel();
    panel.mark_all();
    assert_eq!(panel.marked_totals(), (1, 1, 10));
    let mut copy = panel.clone();
    copy.entries[0].size = FileSize::from_bytes(99);
    copy.marked.remove("folder");
    assert_eq!(copy.marked_totals(), (0, 1, 99));
    assert_eq!(panel.marked_totals(), (1, 1, 10));
    panel.apply_listing(DirectoryListing {
        entries: vec![DirEntry {
            name: "new".to_owned(),
            ..DirEntry::default()
        }],
        parent: None,
        metadata: PanelMetadata::default(),
    });
    assert_eq!(panel.file_totals(), (1, 0));
    assert_eq!(panel.marked_totals(), (0, 0, 0));
}

#[test]
fn duplicate_display_names_and_overflow_keep_existing_semantics() {
    let mut panel = panel();
    panel.entries[0].size = FileSize::from_bytes(u64::MAX);
    let duplicate = panel.entries[0].clone();
    panel.entries.push(duplicate);
    panel.mark_all();
    assert_eq!(panel.file_totals(), (2, u64::MAX));
    assert_eq!(panel.marked_totals(), (1, 2, u64::MAX));
}
