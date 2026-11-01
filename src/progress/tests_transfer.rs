//! Purpose: What the progress dialog is told about the file a copy or a move is
//! sending: where it goes, how much of it is copied and that it is done, and
//! the average rate its time line shows.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use tempfile::TempDir;

use super::tests::Recorder;
use super::{
    begin, sending, source_done, FileProgress, Operation, Phase, Snapshot, Tally, Transfer,
};
use crate::utils::{copy_file, CopyRules};

mod tests_scoped;

#[test]
fn entering_another_path_clears_the_previous_destination() {
    let (recorder, _installed) = Recorder::install();
    let _run = begin(Operation::Move, &[PathBuf::from("source")]);
    sending(Path::new("source/file"), Path::new("target/file"), 10);

    super::entering(Path::new("source/next-folder"));

    let last = recorder.last();
    assert_eq!(last.current, Path::new("source/next-folder"));
    assert_eq!(last.transfer, None);
}

#[cfg(unix)]
#[test]
fn copying_a_link_reports_its_own_source_and_destination() {
    let dir = TempDir::new().expect("tempdir");
    let source = dir.path().join("link");
    let target = dir.path().join("new-link");
    std::os::unix::fs::symlink("missing", &source).expect("link");
    let (recorder, _installed) = Recorder::install();
    let _run = begin(Operation::Copy, std::slice::from_ref(&source));
    sending(Path::new("previous"), Path::new("previous-target"), 1);

    crate::utils::copy_link(&source, &target, CopyRules::PLAIN).expect("link copied");

    let last = recorder.last();
    assert_eq!(last.current, source);
    assert_eq!(last.transfer.expect("link transfer").destination, target);
}

/// The transfer of every frame drawn since the operation began, in order.
fn transfers(recorder: &Recorder) -> Vec<Transfer> {
    recorder
        .frames()
        .into_iter()
        .filter_map(|frame| frame.transfer)
        .collect()
}

/// A move that has moved `bytes` in `elapsed`.
fn moved(bytes: u64, elapsed: Duration) -> Snapshot {
    Snapshot {
        operation: Operation::Move,
        phase: Phase::Working,
        current: PathBuf::new(),
        transfer: None,
        done: Tally { files: 0, bytes },
        total: Tally::default(),
        elapsed,
    }
}

#[test]
fn a_copy_streams_the_bar_of_its_file_block_by_block() {
    let dir = TempDir::new().expect("tempdir");
    let (source, target) = (dir.path().join("big.bin"), dir.path().join("copy.bin"));
    fs::write(&source, vec![7_u8; 9 * 1024 * 1024]).expect("source");
    let (recorder, _installed) = Recorder::install();
    let _run = begin(Operation::Copy, std::slice::from_ref(&source));

    copy_file(&source, &target, CopyRules::PLAIN).expect("copied");

    let files: Vec<FileProgress> = transfers(&recorder).iter().map(|t| t.file).collect();
    let inside = files.iter().any(|file| match file {
        FileProgress::Copying { done, size } => *done > 0 && *done < *size,
        FileProgress::Done => false,
    });
    assert!(inside, "the bar stands inside the file: {files:?}");
    assert_eq!(files.last(), Some(&FileProgress::Done));
}

#[test]
fn a_file_names_its_destination_and_size_when_it_starts() {
    let dir = TempDir::new().expect("tempdir");
    let (source, target) = (dir.path().join("a.txt"), dir.path().join("b.txt"));
    fs::write(&source, "hello").expect("source");
    let (recorder, _installed) = Recorder::install();
    let _run = begin(Operation::Copy, std::slice::from_ref(&source));

    copy_file(&source, &target, CopyRules::PLAIN).expect("copied");

    let frames = transfers(&recorder);
    assert!(
        frames.iter().any(|transfer| {
            transfer.destination == target
                && transfer.file == FileProgress::Copying { done: 0, size: 0 }
        }),
        "the destination is named before the size is read"
    );
    assert!(
        frames.iter().any(|transfer| {
            transfer.destination == target
                && transfer.file == FileProgress::Copying { done: 0, size: 5 }
        }),
        "the size is known before copying starts"
    );
}

#[test]
fn an_entry_sent_by_a_move_is_done_when_its_turn_ends() {
    let (recorder, _installed) = Recorder::install();
    let _run = begin(Operation::Move, &[PathBuf::from("a.txt")]);

    sending(Path::new("a.txt"), Path::new("b.txt"), 0);
    let before = recorder.last().transfer.map(|transfer| transfer.file);
    assert_eq!(before, Some(FileProgress::Copying { done: 0, size: 0 }));

    source_done();
    let after = recorder.last().transfer.map(|transfer| transfer.file);
    assert_eq!(after, Some(FileProgress::Done));
}

#[test]
fn a_delete_sends_no_file() {
    let (recorder, _installed) = Recorder::install();
    let _run = begin(Operation::Delete, &[]);

    sending(Path::new("a"), Path::new("b"), 3);

    assert_eq!(transfers(&recorder), Vec::new());
}

#[test]
fn the_rate_is_the_bytes_so_far_over_the_time_so_far() {
    assert_eq!(
        moved(3_813_360, Duration::from_secs(40)).bytes_per_second(),
        95_334
    );
    assert_eq!(
        moved(3_813_360, Duration::ZERO).bytes_per_second(),
        0,
        "nothing is known before a millisecond has passed"
    );
}

#[test]
fn a_file_is_full_only_once_it_is_done() {
    assert_eq!(FileProgress::Copying { done: 0, size: 0 }.permille(), 0);
    assert_eq!(
        FileProgress::Copying {
            done: 512,
            size: 2048
        }
        .permille(),
        250
    );
    assert_eq!(FileProgress::Done.permille(), 1000);
}
