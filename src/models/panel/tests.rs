//! Unit tests and fixtures for panel snapshot state.

use super::*;

#[test]
fn a_known_listing_can_be_constructed_without_filesystem_metadata() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let panel = Panel::from_entries(
        temp.path().join("does-not-exist"),
        vec![DirEntry {
            name: "file.txt".to_owned(),
            ..DirEntry::default()
        }],
    );
    assert_eq!(panel.selected(), Some(0));
    assert_eq!(panel.available_space(), None);
    assert!(!panel.is_hidden("file.txt"));
}

#[test]
fn applying_a_snapshot_sorts_entries_and_replaces_metadata_together() {
    let temp = tempfile::TempDir::new().expect("temporary directory");
    let mut panel = Panel::from_entries(temp.path().to_path_buf(), Vec::new());
    panel.apply_listing(DirectoryListing {
        entries: ["z.txt", "a.txt"].map(file_entry).to_vec(),
        parent: None,
        metadata: PanelMetadata {
            hidden_names: BTreeSet::from([FileName::from("z.txt")]),
            available_space: Some(FileSize::from_bytes(10)),
        },
    });
    assert_eq!(panel.entries[0].name, "a.txt");
    assert!(panel.is_hidden("z.txt"));
    assert!(!panel.is_hidden("a.txt"));
    assert_eq!(panel.available_space(), Some(FileSize::from_bytes(10)));

    panel.apply_listing(DirectoryListing {
        entries: Vec::new(),
        parent: None,
        metadata: PanelMetadata::default(),
    });
    assert!(panel.entries.is_empty());
    assert!(!panel.is_hidden("z.txt"));
    assert_eq!(panel.available_space(), None);
}

impl Panel {
    /// Build a [`Cursor`] directly, so a test can restore a position without
    /// having to arrange the listing that would have produced it.
    pub(crate) fn cursor_for_test(name: Option<&str>, index: usize) -> Cursor {
        Cursor {
            name: name.map(str::to_owned),
            index,
        }
    }
}

fn file_entry(name: &str) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        ..DirEntry::default()
    }
}
