//! Unit tests: an info-row message about a partial failure, new folders or
//! a new link keeps the name it is about, however deep the folder.

use std::path::PathBuf;

use super::info_row_messages::info_row;
use crate::status::{FailedEntry, Status};

/// A file deep in the tree that a batch failed on first.
fn deep_failure() -> FailedEntry {
    FailedEntry::new(
        "/srv/x/agents/v-copy-3/work/a/deep/folder/notes.txt",
        &"Permission denied (os error 13)",
    )
}

/// A copy, a move, a delete and a wipe of 3 entries that each failed on one.
fn failed_batches() -> [Status; 4] {
    [
        Status::CopiedWithErrors {
            count: 3,
            error_count: 1,
            first_error: deep_failure(),
        },
        Status::MovedWithErrors {
            count: 3,
            error_count: 1,
            first_error: deep_failure(),
        },
        Status::DeletedWithErrors {
            count: 3,
            error_count: 1,
            first_error: deep_failure(),
        },
        Status::WipedWithErrors {
            count: 3,
            error_count: 1,
            first_error: deep_failure(),
        },
    ]
}

/// A partly failed batch names the file it failed on and why, however deep
/// it sits: the head of its path gives way, not the name or the reason.
#[test]
fn a_partial_failure_keeps_the_failed_file_name_and_the_reason() {
    let verbs = ["Copied", "Moved", "Deleted", "Wiped"];
    for (verb, failure) in verbs.into_iter().zip(failed_batches()) {
        let row = info_row(failure, 120, 40);
        let kept = format!("{verb} 3 item(s), 1 error(s): …/notes.txt: Permission");
        assert!(row.starts_with(&kept), "{row:?}");
    }
}

/// Ctrl+A that failed on a deep file names it before the reason gives way.
#[test]
fn a_partial_attribute_failure_keeps_the_failed_file_name() {
    let attributes = Status::AttributesSetWithErrors {
        count: 2,
        error_count: 1,
        first_error: deep_failure(),
    };
    assert_eq!(
        info_row(attributes, 120, 40),
        "Attributes set on 2 object(s), 1 failed: …/notes.txt: Per…"
    );
}

/// F7 with several names counts them and names the folder they went into,
/// even on an 80-column screen's 38-cell row.
#[test]
fn several_new_folders_keep_the_name_of_their_folder() {
    let made = Status::CreatedDirs {
        count: 3,
        directory: PathBuf::from("/srv/x/agents/v-mkdir-1/work/deep/folder/target"),
    };
    let row = info_row(made, 80, 24);
    assert!(
        row.starts_with("Created 3 folders in …") && row.ends_with("/folder/target"),
        "{row:?}"
    );
}

/// A link names itself by its name — it is under the other panel's cursor —
/// and gives the room to its target, which Alt+F6 fills in absolute.
#[test]
fn a_new_link_keeps_the_name_of_its_target() {
    let linked = Status::Linked {
        link: PathBuf::from("/srv/x/agents/v-link-2/work/here/notes.lnk"),
        target: PathBuf::from("/srv/x/agents/v-link-2/work/the/active/panel/notes.txt"),
    };
    let row = info_row(linked, 80, 24);
    assert!(
        row.starts_with("Link notes.lnk -> …") && row.ends_with("/panel/notes.txt"),
        "{row:?}"
    );
}
