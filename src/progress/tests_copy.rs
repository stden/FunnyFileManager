//! Purpose: A copy counts finished files, a cross-filesystem move counts bytes,
//! and a relayed snapshot is drawn without a local operation.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use super::tests::Recorder;
use super::{
    begin, copying_across, file_copied, relay, take_painted, Operation, Phase, Snapshot, Tally,
};

#[test]
fn file_copied_counts_a_file_only_for_a_copy() {
    let (recorder, _installed) = Recorder::install();
    let _run = begin(Operation::Copy, &[PathBuf::from("/srv/x/a.txt")]);
    file_copied();
    assert_eq!(recorder.last().done.files, 1);

    let (recorder, _installed) = Recorder::install();
    let _run = begin(Operation::Move, &[PathBuf::from("/srv/x/a.txt")]);
    file_copied();
    assert_eq!(recorder.last().done.files, 0);
}

#[test]
fn copying_across_adds_the_file_bytes_to_a_move() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let notes = dir.path().join("notes.txt");
    fs::write(&notes, "abcd").expect("notes");
    let (recorder, _installed) = Recorder::install();
    let _run = begin(Operation::Move, std::slice::from_ref(&notes));
    copying_across(&notes);
    assert_eq!(recorder.last().total.bytes, 4);
}

#[test]
fn relay_paints_a_snapshot_without_beginning_an_operation() {
    let (recorder, _installed) = Recorder::install();
    relay(&Snapshot {
        operation: Operation::Copy,
        phase: Phase::Working,
        current: PathBuf::from("notes.txt"),
        done: Tally { files: 1, bytes: 4 },
        total: Tally { files: 2, bytes: 8 },
        elapsed: Duration::ZERO,
    });
    assert!(take_painted());
    assert_eq!(recorder.last().done.bytes, 4);
}
