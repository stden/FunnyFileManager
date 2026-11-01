//! Purpose: The transfer dialog of F5, F6 and F8 row by row: the file being
//! sent with its destination and bar, the totals of the whole operation, and
//! the time with the rate.

use std::path::PathBuf;
use std::time::Duration;

use super::tests::{rows, screen, snapshot};
use super::*;
use crate::progress::{FileProgress, Tally, Transfer};

mod tests_regressions;

/// A row without the frame's side borders and the padding around the text.
fn inner(row: &str) -> &str {
    row.trim_matches(|ch| ch == '║' || ch == ' ')
}

/// The Move of the screenshot: 467 files and 47,125,964 bytes, of which
/// 3,813,360 have moved in 40 seconds (93.1 KB/s), with its first file at 0%.
fn moving() -> Snapshot {
    Snapshot {
        operation: Operation::Move,
        phase: Phase::Working,
        current: PathBuf::from("/from/ABLAGE0045196407.PDF"),
        transfer: Some(Transfer {
            destination: PathBuf::from(r"F:\kmstarc\lw\ABLAGE0045196407.PDF"),
            file: FileProgress::Copying {
                done: 0,
                size: 1_000,
            },
        }),
        done: Tally {
            files: 0,
            bytes: 3_813_360,
        },
        total: Tally {
            files: 467,
            bytes: 47_125_964,
        },
        elapsed: Duration::from_secs(40),
    }
}

/// The dialog's rows of `snapshot`, as text: row 0 is the top border.
fn dialog(snapshot: &Snapshot) -> Vec<String> {
    rows(&progress_dialog(screen(), snapshot))
}

#[test]
fn a_move_shows_its_file_its_destination_its_bar_and_the_totals() {
    let buffer = progress_dialog(screen(), &moving());
    let lines = rows(&buffer);
    assert_eq!(buffer.area, Rect::new(20, 9, 60, 12));
    assert!(lines[0].contains(" Rename/Move "), "{}", lines[0]);
    assert_eq!(inner(&lines[1]), "Moving the file");
    assert_eq!(inner(&lines[2]), "/from/ABLAGE0045196407.PDF");
    assert_eq!(inner(&lines[3]), "to");
    assert_eq!(inner(&lines[4]), r"F:\kmstarc\lw\ABLAGE0045196407.PDF");
    assert!(inner(&lines[5]).starts_with('░'), "{}", lines[5]);
    assert!(inner(&lines[5]).ends_with(" 0%"), "{}", lines[5]);
    assert!(inner(&lines[7]).contains(" Total "), "{}", lines[7]);
    assert!(inner(&lines[8]).starts_with("Files:"), "{}", lines[8]);
    assert!(inner(&lines[8]).ends_with("467"), "{}", lines[8]);
    assert!(inner(&lines[9]).starts_with("Bytes:"), "{}", lines[9]);
    assert!(inner(&lines[9]).ends_with("47,125,964"), "{}", lines[9]);
    assert!(
        inner(&lines[10]).starts_with("Time: 00:00:40"),
        "{}",
        lines[10]
    );
    assert!(
        inner(&lines[10]).ends_with("Speed: 93.1 KB/s"),
        "{}",
        lines[10]
    );
}

#[test]
fn the_totals_and_the_rule_fill_the_width_of_the_text() {
    let lines = dialog(&moving());
    for row in [5, 7, 8, 9, 10] {
        assert_eq!(inner(&lines[row]).chars().count(), 56, "{}", lines[row]);
    }
}

#[test]
fn a_copy_shows_its_bar_at_the_bytes_of_its_file_so_far() {
    let mut copy = moving();
    copy.operation = Operation::Copy;
    copy.transfer = Some(Transfer {
        destination: PathBuf::from("/to/a.txt"),
        file: FileProgress::Copying {
            done: 512,
            size: 2048,
        },
    });
    let lines = dialog(&copy);
    assert_eq!(inner(&lines[1]), "Copying the file");
    assert!(inner(&lines[5]).ends_with("25%"), "{}", lines[5]);
}

#[test]
fn a_file_of_no_bytes_is_full_only_once_it_is_done() {
    let mut copy = moving();
    copy.operation = Operation::Copy;
    copy.transfer.as_mut().expect("moving sends a file").file =
        FileProgress::Copying { done: 0, size: 0 };
    assert!(inner(&dialog(&copy)[5]).ends_with("0%"));
    copy.transfer.as_mut().expect("moving sends a file").file = FileProgress::Done;
    assert!(inner(&dialog(&copy)[5]).ends_with("100%"));
}

#[test]
fn a_scan_shows_what_it_found_so_far_without_a_bar() {
    let mut scan = snapshot(Operation::Copy, Phase::Scanning, "/src");
    scan.done = Tally::default();
    let lines = dialog(&scan);
    assert_eq!(inner(&lines[1]), "Scanning");
    assert_eq!(inner(&lines[5]), "");
    assert!(inner(&lines[8]).ends_with('4'), "{}", lines[8]);
    assert!(inner(&lines[9]).ends_with("2,048"), "{}", lines[9]);
    assert!(inner(&lines[10]).ends_with("Speed: 0 B/s"), "{}", lines[10]);
}

/// A delete reads no bytes, so it names no bytes total, sends no file and
/// has no destination; its bar counts the files it has removed.
#[test]
fn a_delete_names_no_destination_and_totals_no_bytes() {
    let mut delete = snapshot(Operation::Delete, Phase::Working, "/src/a.txt");
    delete.total.bytes = 0;
    let lines = dialog(&delete);
    assert_eq!(inner(&lines[1]), "Deleting the file");
    assert_eq!(inner(&lines[3]), "");
    assert_eq!(inner(&lines[4]), "");
    assert!(inner(&lines[5]).ends_with("25%"), "{}", lines[5]);
    assert_eq!(inner(&lines[9]), "Bytes:");
}

#[test]
fn a_long_destination_is_cut_in_the_middle_to_the_text_width() {
    let mut long = moving();
    if let Some(transfer) = long.transfer.as_mut() {
        transfer.destination = PathBuf::from(
            "/example/archive/long/path/for/documents/project/quarterly/reports/report.pdf",
        );
    }
    let destination = dialog(&long)[4].clone();
    let text = inner(&destination);
    assert_eq!(text.chars().count(), 56, "{destination}");
    assert!(text.starts_with("/example/archive"), "{destination}");
    assert!(text.contains('…'), "{destination}");
    assert!(text.ends_with("report.pdf"), "{destination}");
}

#[test]
fn the_rate_reads_zero_before_any_time_has_passed() {
    let mut idle = moving();
    idle.elapsed = Duration::ZERO;
    assert!(inner(&dialog(&idle)[10]).ends_with("Speed: 0 B/s"));
}

/// A Windows verbatim path is shown as people type it.
#[cfg(windows)]
#[test]
fn a_verbatim_windows_destination_is_shown_without_its_prefix() {
    let mut verbatim = moving();
    if let Some(transfer) = verbatim.transfer.as_mut() {
        transfer.destination = PathBuf::from(r"\\?\C:\work\a.txt");
    }
    let destination = dialog(&verbatim)[4].clone();
    assert_eq!(inner(&destination), r"C:\work\a.txt");
}
