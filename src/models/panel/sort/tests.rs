//! Purpose: Direct unit tests for folded sort keys, out-of-range index guards, and child-path matching.

use std::cmp::Ordering;
use std::path::PathBuf;

use super::{cmp_at, fold, place, sort_entries};
use crate::models::{DirEntry, EntryKind, FileSize, Panel, SortMode, SortOrder};

fn file_entry(name: &str, bytes: u64) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        size: FileSize::from_bytes(bytes),
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

#[test]
fn folded_keys_implement_eq_and_partial_ord_across_ascii_and_unicode() {
    assert!(fold("Readme.MD") == fold("readme.md"));
    assert!(fold("Äpfel") == fold("äpfel"));
    assert_eq!(fold("a").partial_cmp(&fold("B")), Some(Ordering::Less));
}

#[test]
fn cmp_at_and_place_guard_out_of_bounds_indices() {
    assert_eq!(cmp_at::<()>(&[], &[], &[], 2, 1), Ordering::Greater);
    let mut entries = vec![file_entry("a", 0)];
    place(&mut entries, &[99]);
    assert_eq!(entries[0].name, "");
}

#[test]
fn sort_entries_by_name_and_size_preserves_stable_tie_order_and_reversed_groups() {
    let mut entries = vec![
        file_entry("beta.txt", 10),
        file_entry("Alpha.txt", 10),
        file_entry("alpha.txt", 10),
        dir_entry("docs"),
    ];
    sort_entries(&mut entries, SortMode::Name, SortOrder::Direct);
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, vec!["docs", "Alpha.txt", "alpha.txt", "beta.txt"]);

    sort_entries(&mut entries, SortMode::Size, SortOrder::Reversed);
    let rev_names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        rev_names,
        vec!["docs", "beta.txt", "alpha.txt", "Alpha.txt"]
    );

    let mut unicode_ties = vec![
        file_entry("яблоко.txt", 5),
        file_entry("Äpfel.txt", 5),
        file_entry("äpfel.txt", 5),
        file_entry("big.bin", 50),
    ];
    sort_entries(&mut unicode_ties, SortMode::Size, SortOrder::Direct);
    let tied_names: Vec<&str> = unicode_ties.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(
        tied_names,
        vec!["big.bin", "Äpfel.txt", "äpfel.txt", "яблоко.txt"]
    );
}

#[test]
fn set_directory_size_at_and_kind_of_match_direct_children_only() {
    let base = PathBuf::from("root");
    let entries = vec![dir_entry(".."), dir_entry("sub"), file_entry("note.txt", 1)];
    let mut panel = Panel::from_entries(base.clone(), entries);
    panel.sort_mode = SortMode::Size;

    assert_eq!(panel.kind_of(&base.join("sub")), Some(EntryKind::Directory));
    assert_eq!(panel.kind_of(&base.join("note.txt")), Some(EntryKind::File));
    assert_eq!(panel.kind_of(&base.join("sub").join("deep.txt")), None);
    assert_eq!(panel.kind_of(&PathBuf::from("other").join("sub")), None);

    panel.set_directory_size_at(&base.join("sub").join("deep"), FileSize::from_bytes(999));
    assert!(!panel.entries[1].directory_size.is_measured());

    panel.set_directory_size_at(&base.join("sub"), FileSize::from_bytes(512));
    assert_eq!(
        panel.entries[1].directory_size.measured(),
        Some(FileSize::from_bytes(512))
    );
}
