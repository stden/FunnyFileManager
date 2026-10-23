//! Unit tests: what each row of the "Change drive" dialog says — a plugin's
//! key and name, blank columns for a drive with no answer, a bare label for
//! an unknown kind, and no columns on a plugin row.

use super::columns;
use super::tests::{disk, empty_bay, plugin, text};
use crate::diskfree::DriveKind;
use crate::models::RowKind;

/// A plugin row shows its key, and it is wide enough that the box still closes.
#[test]
fn a_plugin_row_shows_its_key_and_name() {
    let rows = [
        disk("C:", DriveKind::Fixed, 100, 50),
        plugin('2', "S3: work"),
    ];
    let lines = text(&rows, 1);
    let row = lines
        .iter()
        .find(|line| line.contains("S3: work"))
        .expect("the plugin row");
    assert!(row.contains(" 2 "), "{row:?}");
}

/// A drive that will not report its size leaves the columns blank rather than
/// claiming zero bytes — an empty bay is not a full disk.
#[test]
fn a_drive_with_no_answer_shows_blank_columns_not_zero() {
    let lines = text(&[empty_bay("E:")], 0);
    let row = &lines[1];
    assert!(row.contains("E: CD-ROM"), "{row:?}");
    assert!(!row.contains('0'), "{row:?}");
}

/// A kind the platform would not name contributes nothing to the label.
#[test]
fn an_unknown_kind_leaves_the_label_bare() {
    let cols = columns(&RowKind::Disk {
        label: "X:".to_owned(),
        device: DriveKind::Unknown,
        space: None,
    })
    .expect("a disk row has columns");
    assert_eq!(cols.label, "X:");
    assert_eq!(cols.total.len(), 0);
}

#[test]
fn a_plugin_row_has_no_columns() {
    assert!(columns(&RowKind::Plugin {
        name: "S3: work".to_owned()
    })
    .is_none());
}
