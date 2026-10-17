use super::PanelFilter;
use crate::mask::Mask;
use crate::models::DirEntry;

fn names(entries: &[DirEntry]) -> Vec<&str> {
    entries.iter().map(|entry| entry.name.as_str()).collect()
}

fn listing() -> Vec<DirEntry> {
    ["..", ".git", "a.rs", "b.txt"]
        .into_iter()
        .map(|name| DirEntry {
            name: name.to_owned(),
            ..DirEntry::default()
        })
        .collect()
}

#[test]
fn the_default_filter_changes_nothing() {
    let mut entries = listing();
    PanelFilter::default().apply(&mut entries);
    assert_eq!(names(&entries), ["..", ".git", "a.rs", "b.txt"]);
    assert!(!PanelFilter::default().is_active());
}

#[test]
fn include_keeps_only_matches_and_exclude_wins_over_include() {
    let mut entries = listing();
    let filter = PanelFilter {
        include: Some(Mask::parse("*.rs,*.txt")),
        exclude: Some(Mask::parse("b.*")),
        hide_dotfiles: false,
    };
    assert!(filter.is_active());
    filter.apply(&mut entries);
    assert_eq!(names(&entries), ["..", "a.rs"]);
}

/// The parent row survives every filter: hiding it would trap the user.
#[test]
fn hiding_dotfiles_never_hides_the_parent_row() {
    let mut entries = listing();
    let filter = PanelFilter {
        hide_dotfiles: true,
        ..PanelFilter::default()
    };
    filter.apply(&mut entries);
    assert_eq!(names(&entries), ["..", "a.rs", "b.txt"]);
}
