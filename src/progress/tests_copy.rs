//! Purpose: A copy counts finished files, a cross-filesystem move counts bytes,
//! and a relayed snapshot is drawn without a local operation.

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use super::tests::Recorder;
use super::{
    begin, copying_across, examined, file_copied, install_with, matched, relay, take_painted, tick,
    Operation, Painter, Phase, Snapshot, Tally,
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
fn a_search_counts_what_it_looked_at_and_what_matched() {
    let (recorder, _installed) = Recorder::install();
    let _run = begin(Operation::Find, &[]);
    examined();
    examined();
    matched();
    let last = recorder.last();
    assert_eq!((last.done.files, last.total.files), (1, 2));
}

#[test]
fn relay_paints_a_snapshot_without_beginning_an_operation() {
    let (recorder, _installed) = Recorder::install();
    relay(&Snapshot {
        operation: Operation::Copy,
        phase: Phase::Working,
        current: PathBuf::from("notes.txt"),
        transfer: None,
        done: Tally { files: 1, bytes: 4 },
        total: Tally { files: 2, bytes: 8 },
        elapsed: Duration::ZERO,
    });
    assert!(take_painted());
    assert_eq!(recorder.last().done.bytes, 4);
}

struct MeasureErrorGuard;

impl Drop for MeasureErrorGuard {
    fn drop(&mut self) {
        super::engine::report::FORCE_MEASURE_ENTRY_ERROR.set(false);
    }
}

struct RemovingPainter {
    victim: PathBuf,
}

impl Painter for RemovingPainter {
    fn paint(&mut self, snapshot: &Snapshot) {
        if snapshot.current == self.victim {
            drop(fs::remove_dir_all(&self.victim));
        }
    }
}

#[test]
fn idle_ticks_empty_git_pulls_and_unreadable_measured_entries_stay_safe() {
    let (_recorder, _installed) = Recorder::install();
    tick();
    assert!(!take_painted());

    {
        let _pull = begin(Operation::GitPull, &[]);
        tick();
        assert!(take_painted());
    }

    let dir = tempfile::TempDir::new().expect("tempdir");
    let file = dir.path().join("item.txt");
    fs::write(&file, b"hello").expect("write");
    {
        super::engine::report::FORCE_MEASURE_ENTRY_ERROR.set(true);
        let _guard = MeasureErrorGuard;
        let (recorder, _installed) = Recorder::install();
        let _copy = begin(Operation::Copy, &[dir.path().to_path_buf()]);
        assert_eq!(recorder.last().total.files, 0);
    }

    let sub = dir.path().join("vanishing");
    fs::create_dir(&sub).expect("mkdir");
    fs::write(sub.join("inside.txt"), b"x").expect("write");
    let _installed = install_with(Box::new(RemovingPainter { victim: sub }), Duration::ZERO);
    let _copy = begin(Operation::Copy, &[dir.path().to_path_buf()]);
}
