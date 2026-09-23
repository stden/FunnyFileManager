//! Unit tests: panel paging with PgUp/PgDn (split out to keep files under 500 lines).
//!
//! Paging clamps at both ends, unlike [`Panel::next`] / [`Panel::previous`],
//! which wrap around — this mirrors Far Manager, where `PgDn` on the last screen
//! parks on the final entry instead of jumping back to the top.

use super::*;

/// Panel holding `count` synthetic files, selection on `selected`.
fn panel_with(count: usize, selected: Option<usize>) -> Panel {
    let entries = (0..count)
        .map(|i| DirEntry {
            name: format!("file{i:03}.txt"),
            kind: EntryKind::File,
            size: FileSize::from_bytes(0),
            modified: Modified::UNKNOWN,
            executable: false,
            ..DirEntry::default()
        })
        .collect();
    let mut panel = Panel::from_entries(PathBuf::new(), entries);
    panel.select(selected);
    panel
}

#[test]
fn test_page_down_advances_by_page_size() {
    let mut panel = panel_with(100, Some(10));
    panel.page_down(20);
    assert_eq!(panel.selected(), Some(30));
}

#[test]
fn test_page_up_retreats_by_page_size() {
    let mut panel = panel_with(100, Some(50));
    panel.page_up(20);
    assert_eq!(panel.selected(), Some(30));
}

#[test]
fn test_page_down_clamps_to_last_entry_without_wrapping() {
    let mut panel = panel_with(30, Some(25));
    panel.page_down(20);
    assert_eq!(
        panel.selected(),
        Some(29),
        "PgDn past the end must park on the last entry, not wrap to the top"
    );
}

#[test]
fn test_page_up_clamps_to_first_entry_without_wrapping() {
    let mut panel = panel_with(30, Some(5));
    panel.page_up(20);
    assert_eq!(
        panel.selected(),
        Some(0),
        "PgUp past the start must park on the first entry, not wrap to the bottom"
    );
}

#[test]
fn test_page_down_at_last_entry_stays_put() {
    let mut panel = panel_with(30, Some(29));
    panel.page_down(20);
    assert_eq!(panel.selected(), Some(29));
}

#[test]
fn test_page_up_at_first_entry_stays_put() {
    let mut panel = panel_with(30, Some(0));
    panel.page_up(20);
    assert_eq!(panel.selected(), Some(0));
}

#[test]
fn test_paging_on_empty_panel_is_a_noop() {
    let mut panel = panel_with(0, None);

    panel.page_down(20);
    assert_eq!(panel.selected(), None);

    panel.page_up(20);
    assert_eq!(panel.selected(), None);
}

#[test]
fn test_paging_without_selection_selects_first_entry() {
    // Mirrors `next` / `previous`, which both select entry 0 when nothing is selected.
    let mut panel = panel_with(30, None);
    panel.page_down(20);
    assert_eq!(panel.selected(), Some(0));

    let mut panel = panel_with(30, None);
    panel.page_up(20);
    assert_eq!(panel.selected(), Some(0));
}

#[test]
fn test_page_size_larger_than_listing_goes_to_the_far_end() {
    let mut panel = panel_with(5, Some(2));
    panel.page_down(1000);
    assert_eq!(panel.selected(), Some(4));

    panel.page_up(1000);
    assert_eq!(panel.selected(), Some(0));
}
