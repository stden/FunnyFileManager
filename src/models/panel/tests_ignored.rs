//! Purpose: Unit tests for deferred `git check-ignore` panel probe state and flicker-free reloads.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::models::{
    DirEntry, DirectoryListing, EntryKind, FileName, IgnoredCandidate, IgnoredProbe, Panel,
    PanelMetadata,
};

fn file_entry(name: &str) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind: EntryKind::File,
        ..DirEntry::default()
    }
}

fn dir_entry(name: &str) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind: EntryKind::Directory,
        ..DirEntry::default()
    }
}

fn candidate(name: &str, is_dir: bool) -> IgnoredCandidate {
    IgnoredCandidate {
        name: FileName::from(name),
        is_dir,
    }
}

fn repo_entries() -> Vec<DirEntry> {
    vec![
        dir_entry(".git"),
        file_entry("a.log"),
        file_entry("main.rs"),
    ]
}

#[test]
fn ignored_probe_collapses_empty_pending_and_retains_provisional_marks_on_reload() {
    assert_eq!(
        IgnoredProbe::from_pending(Vec::new()),
        IgnoredProbe::Complete
    );

    let mut panel = Panel::from_entries(PathBuf::from("/repo"), Vec::new());
    panel.apply_listing(DirectoryListing {
        entries: repo_entries(),
        parent: None,
        metadata: PanelMetadata {
            ignored: BTreeSet::from([FileName::from(".git"), FileName::from("a.log")]),
            ..PanelMetadata::default()
        },
    });
    assert!(panel.is_ignored("a.log"));

    let pending = vec![candidate("a.log", false), candidate("main.rs", false)];
    panel.apply_listing(DirectoryListing {
        entries: repo_entries(),
        parent: None,
        metadata: PanelMetadata {
            ignored: BTreeSet::from([FileName::from(".git")]),
            ignored_probe: IgnoredProbe::from_pending(pending.clone()),
            ..PanelMetadata::default()
        },
    });
    assert!(panel.is_ignored("a.log"), "provisional mark avoids flicker");
    assert_eq!(
        panel.take_ignored_probe(),
        Some((PathBuf::from("/repo"), pending))
    );
    assert_eq!(panel.take_ignored_probe(), None);
}

#[test]
fn apply_ignored_probe_updates_current_directory_and_clears_stale_marks_on_change() {
    let mut entries = repo_entries();
    entries.push(dir_entry("target"));
    let mut panel = Panel::from_entries(PathBuf::from("/repo"), Vec::new());
    panel.apply_listing(DirectoryListing {
        entries,
        parent: Some(dir_entry("..")),
        metadata: PanelMetadata {
            ignored: BTreeSet::from([FileName::from(".git"), FileName::from("a.log")]),
            ..PanelMetadata::default()
        },
    });

    let answer = BTreeSet::from([FileName::from("target"), FileName::from("unlisted.tmp")]);
    assert!(!panel.apply_ignored_probe(Path::new("/other"), &answer));
    assert!(panel.apply_ignored_probe(Path::new("/repo"), &answer));
    assert!(panel.is_ignored(".git") && panel.is_ignored("target"));
    assert!(!panel.is_ignored("a.log") && !panel.is_ignored("unlisted.tmp"));
    assert!(!panel.apply_ignored_probe(Path::new("/repo"), &answer));

    panel.apply_navigation(
        PathBuf::from("/next"),
        DirectoryListing {
            entries: vec![dir_entry("target")],
            parent: None,
            metadata: PanelMetadata::default(),
        },
    );
    assert!(!panel.is_ignored("target"));
}
