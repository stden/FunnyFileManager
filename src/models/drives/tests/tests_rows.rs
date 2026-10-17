//! Purpose: unit tests for the rows the drive menu lists: disks, plugins, and
//! the live rows at the end.

use super::*;

/// The dialog always has at least the root filesystem to show, with a size.
#[test]
fn the_rows_include_the_machines_own_filesystems() {
    let rows = rows();
    assert!(!rows.is_empty(), "a machine always has one filesystem");
    let disks: Vec<_> = rows.iter().filter(|row| row.is_disk()).collect();
    assert!(!disks.is_empty(), "the first section is never empty");
    let RowKind::Disk { space, label, .. } = &disks[0].kind else {
        panic!("filtered to disks");
    };
    assert!(!label.is_empty());
    assert!(
        space.is_some_and(|space| space.total > 0),
        "a mounted filesystem reports its size"
    );
}

/// A plugin row is not part of the boxed table, and carries the entry that
/// opening it needs.
#[test]
fn a_plugin_row_is_not_a_disk() {
    let row = DriveRow {
        key: Some('0'),
        kind: RowKind::Plugin {
            name: "S3: work".to_owned(),
        },
        entry: DriveEntry::S3Profile("work".to_owned()),
    };
    assert!(!row.is_disk());
    assert!(row.answers_to('0'));
    assert_eq!(
        row.kind,
        RowKind::Plugin {
            name: "S3: work".to_owned()
        }
    );
}

/// A local path becomes a disk row that reports its size.
#[test]
fn a_local_path_becomes_a_disk_row_with_its_size() {
    let row = row_for((Some('/'), DriveEntry::Local("/".into())));
    assert!(row.is_disk());
    let RowKind::Disk { label, space, .. } = &row.kind else {
        panic!("built as a disk");
    };
    assert_eq!(label, "/");
    assert!(space.is_some_and(|space| space.total > 0));
}

/// The live dialog always ends with the two plugin rows, each reachable by
/// its digit.
#[test]
fn the_live_rows_end_with_netbox_and_the_process_list() {
    let rows = rows();
    let tail: Vec<&DriveEntry> = rows.iter().rev().take(2).map(|row| &row.entry).collect();
    assert_eq!(tail, [&DriveEntry::ProcessList, &DriveEntry::NetBox]);
    let netbox = rows
        .iter()
        .find(|row| row.entry == DriveEntry::NetBox)
        .expect("a NetBox row");
    let digit = netbox.key.expect("a digit");
    assert_eq!(find_by_key(&rows, digit), Some(DriveEntry::NetBox));
}
