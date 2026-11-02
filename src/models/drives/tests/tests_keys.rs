//! Purpose: unit tests for drive keys and labels, and looking a row up by key.

use super::*;

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
