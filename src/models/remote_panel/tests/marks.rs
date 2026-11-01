//! Remote marks select only visible entries, survive refresh and never include the parent.

use super::{entry, panel, path};
use crate::mask::Mask;
use crate::models::{DirEntry, EntryKind, RemotePanel, RemotePath, RowMotion};

fn show(panel: &mut RemotePanel, path: RemotePath, entries: Vec<DirEntry>) {
    assert_eq!(panel.show(path, entries).unusable_names, 0);
}

#[test]
fn toggle_and_restore_keep_the_parent_out_and_advance_the_cursor() {
    let mut panel = panel();
    show(
        &mut panel,
        RemotePath::root(),
        vec![
            entry("a.txt", EntryKind::File),
            entry("b.txt", EntryKind::File),
        ],
    );
    panel.toggle_mark();
    assert!(panel.action_names().is_empty(), "parent is not a source");
    assert_eq!(panel.cursor(), 0);
    panel.move_cursor(RowMotion::Next);
    assert_eq!(
        panel.action_names(),
        ["a.txt"],
        "selected entry is the fallback"
    );
    panel.toggle_mark();
    assert!(panel.marked("a.txt"));
    assert_eq!(panel.cursor(), 2);
    panel.toggle_mark();
    assert_eq!(panel.action_names(), ["a.txt", "b.txt"]);
    panel.restore_marks();
    assert_eq!(panel.action_names(), ["a.txt"]);
    panel.restore_marks();
    assert_eq!(panel.action_names(), ["a.txt", "b.txt"]);
    panel.toggle_mark();
    assert!(!panel.marked("b.txt"));
}

#[test]
fn masks_and_inversion_preserve_listing_order_and_skip_parent() {
    let mut panel = panel();
    show(
        &mut panel,
        RemotePath::root(),
        vec![
            entry("z.rs", EntryKind::File),
            entry("a.txt", EntryKind::File),
            entry("b.rs", EntryKind::File),
        ],
    );
    panel.mark_matching(&Mask::parse("*.rs"), true);
    assert_eq!(panel.action_names(), ["z.rs", "b.rs"]);
    panel.mark_matching(&Mask::parse("b.*"), false);
    assert_eq!(panel.action_names(), ["z.rs"]);
    panel.restore_marks();
    assert_eq!(panel.action_names(), ["z.rs", "b.rs"]);
    panel.invert_marks();
    assert_eq!(panel.action_names(), ["a.txt"]);
    assert!(!panel.marked(".."));
    panel.restore_marks();
    assert_eq!(panel.action_names(), ["z.rs", "b.rs"]);
}

#[test]
fn refreshing_keeps_existing_marks_but_navigation_clears_them() {
    let mut panel = panel();
    show(
        &mut panel,
        path("/work"),
        vec![entry("a", EntryKind::File), entry("b", EntryKind::File)],
    );
    panel.mark_matching(&Mask::parse("*"), true);
    panel.move_cursor(RowMotion::Last);
    show(
        &mut panel,
        path("/work"),
        vec![entry("b", EntryKind::File), entry("c", EntryKind::File)],
    );
    assert!(!panel.marked("a"));
    assert!(panel.marked("b"));
    assert_eq!(panel.selected().map(|entry| entry.name.as_str()), Some("b"));
    assert_eq!(panel.action_names(), ["b"]);
    show(
        &mut panel,
        path("/other"),
        vec![entry("b", EntryKind::File)],
    );
    assert!(!panel.marked("b"));
    assert!(panel.on_parent());
    assert_eq!(panel.action_names(), Vec::<String>::new());
}
