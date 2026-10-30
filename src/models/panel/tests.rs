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
            dirty_repos: BTreeSet::from([FileName::from("a.txt")]),
            available_space: Some(FileSize::from_bytes(10)),
            ..PanelMetadata::default()
        },
    });
    assert_eq!(panel.entries[0].name, "a.txt");
    assert!(panel.is_hidden("z.txt"));
    assert!(!panel.is_hidden("a.txt"));
    assert!(panel.is_dirty_repo("a.txt"));
    assert!(!panel.is_dirty_repo("z.txt"));
    assert_eq!(panel.available_space(), Some(FileSize::from_bytes(10)));

    panel.apply_listing(DirectoryListing {
        entries: Vec::new(),
        parent: None,
        metadata: PanelMetadata::default(),
    });
    assert_eq!(panel.entries.len(), 0);
    assert!(!panel.is_hidden("z.txt"));
    assert!(!panel.is_dirty_repo("a.txt"));
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

#[test]
fn action_paths_follow_the_cursor_when_nothing_is_marked() {
    let panel = Panel::from_entries(PathBuf::from("/dir"), vec![file_entry("a.txt")]);
    assert_eq!(panel.action_paths(), [PathBuf::from("/dir/a.txt")]);
}

#[test]
fn action_paths_reach_a_row_by_its_name_on_disk() {
    let mut row = file_entry("caf\u{FFFD}.txt");
    row.on_disk = Some(std::ffi::OsString::from("caf-on-disk.txt"));
    let mut panel = Panel::from_entries(PathBuf::from("/dir"), vec![row]);
    panel.marked.insert(FileName::new("caf\u{FFFD}.txt"));
    assert_eq!(
        panel.action_paths(),
        [PathBuf::from("/dir/caf-on-disk.txt")]
    );
}

#[test]
fn action_paths_keep_a_mark_whose_row_is_gone() {
    let mut panel = Panel::from_entries(PathBuf::from("/dir"), vec![file_entry("a.txt")]);
    panel.marked.insert(FileName::new("gone.txt"));
    assert_eq!(panel.action_paths(), [PathBuf::from("/dir/gone.txt")]);
}

#[test]
fn directory_measurement_helpers_update_and_resort_by_size() {
    let dir_entry = |name: &str| DirEntry {
        name: name.to_owned(),
        kind: crate::models::EntryKind::Directory,
        ..DirEntry::default()
    };
    let mut panel = Panel::from_entries(
        PathBuf::from("/dir"),
        vec![dir_entry(".."), dir_entry("alpha"), dir_entry("beta")],
    );
    assert!(panel.selected_is_parent_dir());
    assert_eq!(
        panel.directory_targets(),
        vec![
            ("alpha".to_owned(), PathBuf::from("/dir/alpha")),
            ("beta".to_owned(), PathBuf::from("/dir/beta")),
        ]
    );
    panel.sort_mode = SortMode::Size;
    panel.set_directory_size_at(std::path::Path::new("/dir/alpha"), FileSize::from_bytes(10));
    panel.set_directory_sizes(&[("beta".to_owned(), FileSize::from_bytes(50))]);
    assert_eq!(panel.entries[1].name, "beta");
    assert_eq!(
        panel.entries[1].directory_size.measured(),
        Some(FileSize::from_bytes(50))
    );
    assert_eq!(panel.entries[2].name, "alpha");
    assert_eq!(
        panel.entries[2].directory_size.measured(),
        Some(FileSize::from_bytes(10))
    );
}

#[test]
fn navigation_history_retains_the_most_recent_hundred_folders_in_order() {
    let mut panel = Panel::from_entries(PathBuf::from("/dir/0"), Vec::new());
    for step in 1..=105 {
        panel.apply_navigation(
            PathBuf::from(format!("/dir/{step}")),
            DirectoryListing {
                entries: Vec::new(),
                parent: None,
                metadata: PanelMetadata::default(),
            },
        );
    }
    assert_eq!(panel.navigation_history.len(), 100);
    assert_eq!(
        panel.navigation_history.front(),
        Some(&PathBuf::from("/dir/5"))
    );
    assert_eq!(
        panel.navigation_history.back(),
        Some(&PathBuf::from("/dir/104"))
    );
}
