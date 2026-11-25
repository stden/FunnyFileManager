//! Purpose: Unit tests for single-pass streaming delete progress, deferred
//! path updates under throttling, and batched folder measurement.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tempfile::TempDir;

use super::tests::Recorder;
use super::{
    begin, begin_delete, delete_progress, install_with, is_shown, removed, take_painted, tick,
    Operation, Phase, Tally,
};

const fn tally(files: u64, bytes: u64) -> Tally {
    Tally { files, bytes }
}

#[test]
fn begin_delete_starts_working_without_upfront_scan_and_streams_totals() {
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("tempdir");
    let folder = temp.path().join("big");
    fs::create_dir(&folder).expect("mkdir");
    fs::write(folder.join("one.txt"), b"123").expect("write");

    let run = begin_delete(std::slice::from_ref(&folder));
    assert!(is_shown());
    assert_eq!(
        recorder.frames().len(),
        0,
        "begin_delete does not walk disk"
    );

    let first = folder.join("one.txt");
    assert!(delete_progress(3, 1, Some(&first)));
    assert_eq!(recorder.last().phase, Phase::Working);
    assert_eq!(recorder.last().total, tally(3, 0));
    assert_eq!(recorder.last().done, tally(1, 0));
    assert_eq!(recorder.last().current, first);

    assert!(delete_progress(0, 2, None));
    assert_eq!(recorder.last().total, tally(3, 0));
    assert_eq!(recorder.last().done, tally(3, 0));
    drop(run);
    assert!(take_painted());
    assert!(!is_shown());
}

#[test]
fn delete_progress_returns_false_when_idle_even_with_painter_installed() {
    let silent = begin_delete(&[PathBuf::from("none")]);
    assert!(!delete_progress(1, 1, Some(Path::new("none"))));
    drop(silent);

    let (_recorder, _installed) = Recorder::install();
    assert!(!delete_progress(1, 0, Some(Path::new("idle"))));

    let recorder = Recorder::default();
    let _installed = install_with(Box::new(recorder.clone()), Duration::from_secs(3600));
    let initial = PathBuf::from("root_folder");
    let run = begin_delete(std::slice::from_ref(&initial));
    assert!(!delete_progress(2, 1, Some(Path::new("ignored_a.txt"))));
    removed(Path::new("ignored_b.txt"));
    assert_eq!(recorder.frames().len(), 0);
    assert!(!take_painted());
    drop(run);
}

#[test]
fn nested_begin_delete_and_prescanned_or_non_delete_runs_keep_their_totals() {
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("tempdir");
    let file = temp.path().join("only.txt");
    fs::write(&file, b"abc").expect("write");

    let prescanned = begin(Operation::Delete, std::slice::from_ref(&file));
    let nested = begin_delete(&[PathBuf::from("nested")]);
    drop(nested);
    assert!(is_shown(), "nested begin_delete does not end outer run");

    assert!(delete_progress(5, 1, Some(&file)));
    assert_eq!(
        recorder.last().total,
        tally(1, 0),
        "prescanned delete does not double-count discovered files"
    );
    assert_eq!(recorder.last().done, tally(1, 0));
    drop(prescanned);

    let move_run = begin(Operation::Move, &[PathBuf::from("moved_a")]);
    assert!(delete_progress(4, 2, Some(Path::new("moved_a"))));
    assert_eq!(recorder.last().total, tally(1, 0));
    assert_eq!(recorder.last().done, tally(0, 0));
    tick();
    drop(move_run);
}

#[test]
fn batched_measure_counts_large_folders_across_batches() {
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("tempdir");
    let folder = temp.path().join("many");
    let sub = folder.join("nested");
    fs::create_dir_all(&sub).expect("mkdir");
    for index in 0..140_u32 {
        fs::write(folder.join(format!("f_{index:03}.bin")), b"abcd").expect("write");
    }
    fs::write(sub.join("tail.bin"), b"xy").expect("write");

    let run = begin(Operation::Copy, std::slice::from_ref(&folder));
    let last = recorder.last();
    assert_eq!(last.total, tally(141, 140 * 4 + 2));
    assert!(
        recorder.frames().len() <= 8,
        "folder measurement batches in 64-entry chunks"
    );
    drop(run);

    let target = temp.path().join("link_target");
    fs::create_dir(&target).expect("mkdir target");
    fs::write(target.join("inside.bin"), b"12345").expect("write target");
    let link = temp.path().join("dir_link");
    crate::test_support::make_dir_link(&target, &link).expect("make dir link");
    let link_run = begin(Operation::Copy, std::slice::from_ref(&link));
    assert_eq!(recorder.last().total, tally(1, 0));
    drop(link_run);
}

#[test]
fn exact_batch_and_empty_folder_measurement_and_due_removed_paths() {
    let (recorder, _installed) = Recorder::install();
    let temp = TempDir::new().expect("tempdir");
    let empty = temp.path().join("empty");
    let exact = temp.path().join("exact_64");
    fs::create_dir_all(&empty).expect("mkdir empty");
    fs::create_dir_all(&exact).expect("mkdir exact");
    for index in 0..64_u32 {
        fs::write(exact.join(format!("e_{index:02}.txt")), b"abc").expect("write exact");
    }

    let empty_run = begin(Operation::Copy, std::slice::from_ref(&empty));
    assert_eq!(recorder.last().total, tally(0, 0));
    drop(empty_run);

    let exact_run = begin(Operation::Copy, std::slice::from_ref(&exact));
    assert_eq!(recorder.last().total, tally(64, 64 * 3));
    drop(exact_run);

    let del_run = begin_delete(&[PathBuf::from("gone.txt")]);
    removed(Path::new("gone.txt"));
    assert_eq!(recorder.last().done, tally(1, 0));
    assert_eq!(recorder.last().current, Path::new("gone.txt"));
    drop(del_run);

    let move_run = begin(Operation::Move, &[PathBuf::from("moved.txt")]);
    removed(Path::new("moved.txt"));
    assert_eq!(recorder.last().done, tally(0, 0));
    assert_eq!(recorder.last().current, Path::new("moved.txt"));
    drop(move_run);
}
