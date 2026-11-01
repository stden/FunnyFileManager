//! Purpose: Unit tests for deferred dirty-repository panel state transitions.

use std::path::Path;

use super::*;
use crate::models::EntryKind;

fn dir_entry(name: &str) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind: EntryKind::Directory,
        ..DirEntry::default()
    }
}

fn file_entry(name: &str) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind: EntryKind::File,
        ..DirEntry::default()
    }
}

fn pending_listing(entries: &[&str], pending: &[&str]) -> DirectoryListing {
    DirectoryListing {
        entries: entries.iter().map(|name| dir_entry(name)).collect(),
        parent: None,
        metadata: PanelMetadata {
            dirty_probe: DirtyProbe::from_pending(
                pending.iter().map(|name| FileName::from(*name)).collect(),
            ),
            ..PanelMetadata::default()
        },
    }
}

#[test]
fn dirty_probe_collapses_empty_pending_and_trims_hidden_or_unlisted_entries() {
    assert_eq!(DirtyProbe::from_pending(Vec::new()), DirtyProbe::Complete);

    let mut panel = Panel::from_entries(PathBuf::from("/work"), Vec::new());
    panel.options.hide_hidden = true;
    let mut listing = pending_listing(
        &[".hidden-repo", "visible-repo"],
        &[".hidden-repo", "visible-repo", "notes.txt", "vanished-repo"],
    );
    listing.entries.push(file_entry("notes.txt"));
    panel.apply_listing(listing);

    assert_eq!(
        panel.take_dirty_probe(),
        Some((PathBuf::from("/work"), vec![FileName::from("visible-repo")],))
    );
    assert_eq!(panel.take_dirty_probe(), None);
}

#[test]
fn reload_retains_provisional_dirty_badge_while_navigation_clears_it() {
    let mut panel = Panel::from_entries(PathBuf::from("/work"), Vec::new());
    panel.apply_listing(DirectoryListing {
        entries: vec![dir_entry("repo-a"), dir_entry("repo-b")],
        parent: None,
        metadata: PanelMetadata {
            dirty_repos: BTreeSet::from([FileName::from("repo-a"), FileName::from("repo-b")]),
            ..PanelMetadata::default()
        },
    });

    panel.apply_listing(pending_listing(&["repo-a", "repo-b"], &["repo-a"]));
    assert!(panel.is_dirty_repo("repo-a"));
    assert!(!panel.is_dirty_repo("repo-b"));

    panel.apply_navigation(
        PathBuf::from("/other"),
        pending_listing(&["repo-a"], &["repo-a"]),
    );
    assert!(!panel.is_dirty_repo("repo-a"));
}

#[test]
fn apply_dirty_probe_matches_current_directory_and_reports_only_real_changes() {
    let mut panel = Panel::from_entries(
        PathBuf::from("/work"),
        vec![dir_entry("repo-a"), file_entry("plain.txt")],
    );
    let repo_a = FileName::from("repo-a");
    let plain = FileName::from("plain.txt");
    let missing = FileName::from("gone");

    assert!(!panel.apply_dirty_probe(Path::new("/other"), &repo_a, true));
    assert!(!panel.apply_dirty_probe(Path::new("/work"), &plain, true));
    assert!(!panel.apply_dirty_probe(Path::new("/work"), &missing, true));
    assert!(!panel.apply_dirty_probe(Path::new("/work"), &repo_a, false));

    assert!(panel.apply_dirty_probe(Path::new("/work"), &repo_a, true));
    assert!(panel.is_dirty_repo("repo-a"));
    assert!(!panel.apply_dirty_probe(Path::new("/work"), &repo_a, true));

    assert!(panel.apply_dirty_probe(Path::new("/work"), &repo_a, false));
    assert!(!panel.is_dirty_repo("repo-a"));
}

#[test]
fn dirty_answers_share_the_listing_index_and_follow_in_place_changes() {
    use super::search_index::INDEX_BUILDS;

    let parent = Path::new("/work");
    let mut panel = Panel::from_entries(
        parent.to_path_buf(),
        vec![dir_entry("Repo"), dir_entry("répo"), file_entry("plain")],
    );
    INDEX_BUILDS.set(0);
    assert!(!panel.apply_dirty_probe(Path::new("/elsewhere"), &FileName::from("Repo"), true));
    assert_eq!(INDEX_BUILDS.get(), 0, "stale answers need no index");
    assert!(panel.apply_dirty_probe(parent, &FileName::from("Repo"), true));
    assert!(panel.apply_dirty_probe(parent, &FileName::from("répo"), true));
    assert!(!panel.apply_dirty_probe(parent, &FileName::from("repo"), true));
    assert!(!panel.apply_dirty_probe(parent, &FileName::from("plain"), true));
    assert_eq!(
        INDEX_BUILDS.get(),
        1,
        "a batch must share one listing index"
    );

    panel.entries[0].name = "renamed".to_owned();
    assert!(!panel.apply_dirty_probe(parent, &FileName::from("Repo"), false));
    assert!(panel.apply_dirty_probe(parent, &FileName::from("renamed"), true));
    assert_eq!(INDEX_BUILDS.get(), 2, "renaming invalidates the index");

    panel.entries[0].kind = EntryKind::File;
    assert!(!panel.apply_dirty_probe(parent, &FileName::from("renamed"), false));
    assert_eq!(
        INDEX_BUILDS.get(),
        3,
        "changing the entry kind invalidates it"
    );
}
