//! Purpose: Unit tests for `set_directory_size_at` when a direct child is absent or a file.

use std::path::PathBuf;

use crate::models::{DirEntry, EntryKind, FileSize, Panel, SortMode};

#[test]
fn set_directory_size_at_ignores_missing_or_non_directory_children() {
    let base = PathBuf::from("/dir");
    let entries = vec![
        DirEntry {
            name: "note.txt".to_owned(),
            kind: EntryKind::File,
            ..DirEntry::default()
        },
        DirEntry {
            name: "sub".to_owned(),
            kind: EntryKind::Directory,
            ..DirEntry::default()
        },
    ];
    let mut panel = Panel::from_entries(base.clone(), entries);
    panel.sort_mode = SortMode::Size;

    panel.set_directory_size_at(&base.join("missing"), FileSize::from_bytes(50));
    panel.set_directory_size_at(&base.join("note.txt"), FileSize::from_bytes(50));
    assert!(!panel.entries[1].directory_size.is_measured());
}
