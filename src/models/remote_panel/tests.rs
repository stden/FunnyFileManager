//! Unit tests: a remote listing goes under the `..` row, names that could
//! walk the panel elsewhere are left out and counted, and the cursor stays
//! inside the listing.

use super::{RemotePanel, Shown, PARENT};
use crate::models::{DirEntry, EntryKind, RemotePath, RowMotion, SessionName};
use crate::remote::fake::FakeRemoteFs;
use crate::remote::RemoteLink;

mod marks;

fn entry(name: &str, kind: EntryKind) -> DirEntry {
    DirEntry {
        name: name.to_owned(),
        kind,
        ..DirEntry::default()
    }
}

fn panel() -> RemotePanel {
    RemotePanel::new(
        SessionName::new("demo").expect("name"),
        RemoteLink::new(Box::new(FakeRemoteFs::default())),
    )
}

fn path(text: &str) -> RemotePath {
    RemotePath::parse(text).expect("path")
}

fn names(panel: &RemotePanel) -> Vec<&str> {
    panel.entries().iter().map(|e| e.name.as_str()).collect()
}

#[test]
fn a_new_panel_is_on_the_root_with_only_the_way_out() {
    let panel = panel();
    assert!(panel.path().is_root());
    assert_eq!(names(&panel), [PARENT]);
    assert!(panel.on_parent());
    assert_eq!(panel.title(), "demo:/");
    assert_eq!(panel.session().as_str(), "demo");
}

#[test]
fn show_puts_the_listing_under_the_parent_row_and_resets_the_cursor() {
    let mut panel = panel();
    panel.move_cursor(RowMotion::Last);
    panel.scroll = 3;
    let shown = panel.show(
        path("/var/www"),
        vec![
            entry("html", EntryKind::Directory),
            entry("a.txt", EntryKind::File),
        ],
    );
    assert_eq!(shown, Shown { unusable_names: 0 });
    assert_eq!(names(&panel), [PARENT, "html", "a.txt"]);
    assert_eq!(panel.entries()[0].kind, EntryKind::Directory);
    assert_eq!(panel.cursor(), 0);
    assert_eq!(panel.scroll, 0);
    assert_eq!(panel.title(), "demo:/var/www");
}

#[test]
fn names_that_are_not_one_path_part_are_left_out_and_counted() {
    let mut panel = panel();
    let shown = panel.show(
        RemotePath::root(),
        vec![
            entry("..", EntryKind::Directory),
            entry("etc", EntryKind::Directory),
            entry("a/b", EntryKind::File),
            entry("", EntryKind::File),
            entry(".", EntryKind::Directory),
        ],
    );
    assert_eq!(shown.unusable_names, 4);
    assert_eq!(names(&panel), [PARENT, "etc"]);
}

#[test]
fn the_cursor_moves_inside_the_listing_and_a_click_selects_a_drawn_row() {
    let mut panel = panel();
    let shown = panel.show(
        RemotePath::root(),
        vec![entry("a", EntryKind::File), entry("b", EntryKind::File)],
    );
    assert_eq!(shown, Shown { unusable_names: 0 });
    panel.move_cursor(RowMotion::Next);
    assert!(!panel.on_parent());
    assert_eq!(panel.selected().map(|e| e.name.as_str()), Some("a"));
    panel.move_cursor(RowMotion::PageDown(10));
    assert_eq!(panel.cursor(), 2);
    assert_eq!(panel.select_visible(0), Some(0));
    assert!(panel.on_parent());
    panel.scroll = 2;
    assert_eq!(panel.select_visible(0), Some(2));
    assert_eq!(panel.select_visible(1), None);
    assert_eq!(panel.cursor(), 2);
}

#[test]
fn clones_share_one_connection_and_debug_names_no_host() {
    let panel = panel();
    let copy = panel.clone();
    assert_eq!(panel.link(), copy.link(), "the same connection");
    assert_eq!(panel, copy);
    assert_ne!(
        panel.link(),
        &RemoteLink::new(Box::new(FakeRemoteFs::default())),
        "another connection is not this one"
    );
    assert_eq!(format!("{:?}", panel.link()), "RemoteLink(<connected>)");
}
