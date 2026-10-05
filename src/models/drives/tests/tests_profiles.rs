//! Purpose: unit tests for storage profiles in the drive menu: their digits
//! and their plugin rows.

use super::*;

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

/// Far lists its plugins after the disks and the profiles, numbered from 0
/// in that order: the profiles first, then `NetBox`, then the process list.
#[test]
fn the_plugin_rows_follow_the_profiles_and_take_the_next_digits() {
    let entries = vec![
        DriveEntry::Local("/".into()),
        DriveEntry::S3Profile("work".to_owned()),
        DriveEntry::NetBox,
        DriveEntry::ProcessList,
    ];
    assert_eq!(
        keys(&entries),
        vec![Some('/'), Some('0'), Some('1'), Some('2')]
    );
    let netbox = row_for((Some('1'), DriveEntry::NetBox));
    assert!(!netbox.is_disk());
    assert!(netbox.answers_to('1'));
    assert_eq!(
        netbox.kind,
        RowKind::Plugin {
            name: "NetBox".to_owned()
        }
    );
    let tasks = row_for((Some('2'), DriveEntry::ProcessList));
    assert_eq!(
        tasks.kind,
        RowKind::Plugin {
            name: "Process list".to_owned()
        }
    );
}
