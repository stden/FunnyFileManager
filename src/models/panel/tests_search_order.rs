//! Purpose: quick search follows listing order, including on a panel that is
//! not sorted by name, and builds its prefix index once per listing.

use super::search_index::INDEX_BUILDS;
use super::*;
use crate::models::EntryKind;

fn entry(name: &str) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind: if name == "docs" {
            EntryKind::Directory
        } else {
            EntryKind::File
        },
        ..DirEntry::default()
    }
}

fn listing() -> Panel {
    Panel::from_entries(
        PathBuf::from("/listing"),
        vec![
            entry(".."),
            entry("zeta"),
            entry("alpha"),
            entry("Alpine"),
            entry("beta"),
            entry("İstanbul"),
            entry("istanbul"),
        ],
    )
}

fn selected(panel: &Panel) -> String {
    panel
        .get_selected_entry()
        .map(|entry| entry.name.clone())
        .unwrap_or_default()
}

#[test]
fn first_and_next_follow_listing_order_not_alphabetical_order() {
    let mut panel = listing();
    INDEX_BUILDS.set(0);

    assert!(panel.find_first_matching("a"));
    assert_eq!(selected(&panel), "alpha");
    assert!(panel.find_next_matching("a"));
    assert_eq!(selected(&panel), "Alpine");
    assert!(panel.find_next_matching("a"));
    assert_eq!(selected(&panel), "alpha", "the only later miss wraps");

    assert!(panel.find_first_matching("i"));
    assert_eq!(selected(&panel), "İstanbul");
    assert!(panel.find_next_matching("i"));
    assert_eq!(selected(&panel), "istanbul");

    assert!(panel.find_first_matching("İ"));
    assert_eq!(selected(&panel), "İstanbul");
    assert!(panel.find_next_matching("İ"));
    assert_eq!(
        selected(&panel),
        "İstanbul",
        "the one match wraps to itself"
    );

    let stayed = panel.selected();
    assert!(!panel.find_first_matching("zzz"));
    assert!(!panel.find_first_matching(""));
    assert!(!panel.find_first_matching("."));
    assert_eq!(panel.selected(), stayed);
    assert_eq!(INDEX_BUILDS.get(), 1, "one listing builds the index once");
}
