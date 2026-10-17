//! Unit tests: the attributes dialog's rules — three-state boxes, the tab
//! ring, what a changed box asks for, and how the time fields parse.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use super::{AttributesDialog, AttributesField, Check, EntrySnapshot, Flag};

#[cfg(not(windows))]
const SET_FLAG: Flag = Flag::OwnerWrite;
#[cfg(windows)]
const SET_FLAG: Flag = Flag::ReadOnly;
#[cfg(not(windows))]
const CLEAR_FLAG: Flag = Flag::OtherWrite;
#[cfg(windows)]
const CLEAR_FLAG: Flag = Flag::Hidden;
#[cfg(not(windows))]
const MIXED_FLAG: Flag = Flag::GroupRead;
#[cfg(windows)]
const MIXED_FLAG: Flag = Flag::System;

fn snapshot(name: &str, bits: u32, is_dir: bool) -> EntrySnapshot {
    EntrySnapshot {
        path: PathBuf::from(name),
        bits,
        is_dir,
        owner: Some("alice".to_owned()),
        group: Some("staff".to_owned()),
        modified: Some(SystemTime::UNIX_EPOCH + Duration::from_hours(24)),
        accessed: None,
    }
}

fn flag_box(dialog: &AttributesDialog, flag: Flag) -> &super::FlagBox {
    dialog
        .flags
        .iter()
        .find(|it| it.flag == flag)
        .expect("the platform lists this flag")
}

/// A single file opens with its own times filled in, in the format the
/// field parses back.
#[test]
fn a_single_file_offers_its_original_times() {
    let dialog = AttributesDialog::from_snapshots(&[snapshot("a", 0o644, false)]);
    let offered = super::time_field::parse_time("Modification time", &dialog.modified.current)
        .expect("the offered text parses");
    assert_eq!(
        offered,
        Some(SystemTime::UNIX_EPOCH + Duration::from_hours(24))
    );
    assert!(
        dialog.accessed.current.is_empty(),
        "unknown at open: left empty"
    );
}

mod tests_boxes;
mod tests_controls;
mod tests_times;
