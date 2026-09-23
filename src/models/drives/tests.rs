use super::{drive_key, drive_label, find_by_key, keys, row_for, rows, DriveRow, RowKind};
use crate::diskfree::{Capacity, DriveKind};
use crate::models::DriveEntry;
use std::path::Path;

/// Windows hands out `C:\`; the dialog's first column is `C:`, and the key is
/// the letter itself.
#[test]
fn a_windows_drive_loses_its_trailing_separator() {
    assert_eq!(drive_label(Path::new("C:\\")), "C:");
    assert_eq!(drive_key(Path::new("C:\\")), 'C');
    assert_eq!(drive_label(Path::new("Z:\\")), "Z:");
}

/// A Unix mount point is already the name; nothing is stripped from it.
#[test]
fn a_unix_mount_point_is_shown_as_it_is() {
    assert_eq!(drive_label(Path::new("/")), "/");
    assert_eq!(drive_key(Path::new("/")), '/');
    assert_eq!(drive_label(Path::new("/mnt/data")), "/mnt/data");
}

/// Only a real drive spec loses the backslash — a directory that merely ends
/// in one keeps its name.
#[test]
fn a_trailing_separator_survives_when_it_is_not_a_drive() {
    assert_eq!(drive_label(Path::new("share\\")), "share\\");
}

#[test]
fn an_empty_path_does_not_panic_on_the_key() {
    assert_eq!(drive_key(Path::new("")), '?');
}

/// The dialog always has at least the root filesystem to show, with a size.
#[test]
fn the_rows_include_the_machines_own_filesystems() {
    let rows = rows();
    assert!(!rows.is_empty(), "a machine always has one filesystem");
    let disks: Vec<_> = rows.iter().filter(|row| row.is_disk()).collect();
    assert!(!disks.is_empty(), "the first section is never empty");
    let RowKind::Disk { space, label, .. } = &disks[0].kind else {
        unreachable!("filtered to disks");
    };
    assert!(!label.is_empty());
    assert!(
        space.is_some_and(|space| space.total > 0),
        "a mounted filesystem reports its size"
    );
}

/// Every row is reachable by its own key, and a disk row answers to the letter
/// in either case.
#[test]
fn a_row_answers_to_its_own_key_in_either_case() {
    let row = DriveRow {
        key: Some('C'),
        kind: RowKind::Disk {
            label: "C:".to_owned(),
            device: DriveKind::Fixed,
            space: Some(Capacity {
                total: 100,
                available: 40,
            }),
        },
        entry: DriveEntry::Local("C:\\".into()),
    };
    assert!(row.answers_to('c'));
    assert!(row.answers_to('C'));
    assert!(!row.answers_to('d'));
    assert!(row.is_disk());
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

/// The rows under the rule get the digits, in order, whatever the disks above
/// them are called. Built here rather than through `rows()`, which can only
/// produce a profile on a machine with AWS configuration.
#[test]
fn profiles_take_the_digits_in_order() {
    let entries = vec![
        DriveEntry::Local("C:\\".into()),
        DriveEntry::S3Profile("work".to_owned()),
        DriveEntry::S3Profile("home".to_owned()),
    ];
    assert_eq!(keys(&entries), vec![Some('C'), Some('0'), Some('1')]);
}

/// Past the tenth profile the shortcut runs out; the row is still listed and
/// still reachable with the cursor, but it answers to no key at all — the
/// keyless state used to be a `' '` sentinel, and Space silently opened the
/// eleventh profile.
#[test]
fn an_eleventh_profile_gets_no_shortcut() {
    let entries: Vec<DriveEntry> = (0..11)
        .map(|n| DriveEntry::S3Profile(format!("p{n}")))
        .collect();
    let handed_out = keys(&entries);
    assert_eq!(handed_out.len(), 11);
    assert_eq!(handed_out[9], Some('9'));
    assert_eq!(handed_out[10], None);

    let keyless = row_for((None, DriveEntry::S3Profile("p10".to_owned())));
    assert!(!keyless.answers_to(' '), "Space is not the row's shortcut");
    assert!(!keyless.answers_to('9'));
    assert!(find_by_key(&[keyless], ' ').is_none());
}

/// A profile becomes a plugin row, named the way the panel would report it.
#[test]
fn a_profile_becomes_a_named_plugin_row() {
    let row = row_for((Some('0'), DriveEntry::S3Profile("work".to_owned())));
    assert!(!row.is_disk());
    assert_eq!(
        row.kind,
        RowKind::Plugin {
            name: "S3: work".to_owned()
        }
    );
    assert!(row.answers_to('0'));
}

/// The lookup ignores case in both directions, pinned on constructed rows:
/// on the machines the suite runs on the live key is `/`, whose case swap is
/// the identity, so a lookup through the real drive list never exercises the
/// case-insensitive branch.
#[test]
fn the_key_lookup_ignores_case_in_both_directions() {
    let rows = vec![
        row_for((Some('c'), DriveEntry::Local("C:\\".into()))),
        row_for((Some('D'), DriveEntry::Local("D:\\".into()))),
    ];
    assert!(
        matches!(find_by_key(&rows, 'C'), Some(DriveEntry::Local(path)) if path == Path::new("C:\\")),
        "an upper-case press reaches the row keyed lower case"
    );
    assert!(
        matches!(find_by_key(&rows, 'd'), Some(DriveEntry::Local(path)) if path == Path::new("D:\\")),
        "a lower-case press reaches the row keyed upper case"
    );
    assert!(find_by_key(&rows, 'x').is_none());
}

/// A local path becomes a disk row that reports its size.
#[test]
fn a_local_path_becomes_a_disk_row_with_its_size() {
    let row = row_for((Some('/'), DriveEntry::Local("/".into())));
    assert!(row.is_disk());
    let RowKind::Disk { label, space, .. } = &row.kind else {
        unreachable!("built as a disk");
    };
    assert_eq!(label, "/");
    assert!(space.is_some_and(|space| space.total > 0));
}
