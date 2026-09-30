//! Purpose: The progress dialog's text: heading, the file's path cut from the
//! left, the bar and its percentage, the counts and the time.

use std::path::PathBuf;
use std::time::Duration;

use super::*;
use crate::progress::Tally;

fn snapshot(operation: Operation, phase: Phase, current: &str) -> Snapshot {
    Snapshot {
        operation,
        phase,
        current: PathBuf::from(current),
        done: Tally {
            files: 1,
            bytes: 512,
        },
        total: Tally {
            files: 4,
            bytes: 2048,
        },
        elapsed: Duration::from_secs(3725),
    }
}

/// The dialog's rows as text.
fn rows(buffer: &Buffer) -> Vec<String> {
    let area = buffer.area;
    (area.y..area.y + area.height)
        .map(|y| {
            (area.x..area.x + area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        })
        .collect()
}

fn screen() -> Rect {
    Rect::new(0, 0, 100, 30)
}

#[test]
fn a_copy_shows_its_file_bar_counts_and_time() {
    let buffer = progress_dialog(
        screen(),
        &snapshot(Operation::Copy, Phase::Working, "/src/a.txt"),
    );
    let text = rows(&buffer).join("\n");
    assert_eq!(buffer.area, Rect::new(12, 11, MAX_WIDTH, HEIGHT));
    assert!(text.contains(" Copy "), "{text}");
    assert!(text.contains("Copying"), "{text}");
    assert!(text.contains("/src/a.txt"), "{text}");
    assert!(
        text.contains("█") && text.contains("░") && text.contains("25%"),
        "{text}"
    );
    assert!(
        text.contains("Files: 1 of 4   Size: 512 B of 2.00 KB"),
        "{text}"
    );
    assert!(text.contains("Time: 01:02:05"), "{text}");
}

#[test]
fn every_cell_of_the_dialog_stands_on_its_ground() {
    let buffer = progress_dialog(screen(), &snapshot(Operation::Delete, Phase::Working, "x"));
    assert!(buffer
        .content
        .iter()
        .all(|cell| cell.bg == palette::DIALOG_BG));
}

#[test]
fn a_scan_counts_what_it_found_without_a_bar() {
    let text = rows(&progress_dialog(
        screen(),
        &snapshot(Operation::Copy, Phase::Scanning, "/src"),
    ))
    .join("\n");
    assert!(text.contains("Scanning"), "{text}");
    assert!(!text.contains('█') && !text.contains('%'), "{text}");
    assert!(text.contains("Files: 4   Size: 2.00 KB"), "{text}");
}

#[test]
fn counts_without_bytes_are_files_alone() {
    let mut delete = snapshot(Operation::Delete, Phase::Working, "x");
    delete.total.bytes = 0;
    assert_eq!(counts(&delete), "Files: 1 of 4");
    delete.phase = Phase::Scanning;
    assert_eq!(counts(&delete), "Files: 4");
}

#[test]
fn every_operation_has_its_title_and_heading() {
    for (operation, title_text, heading) in [
        (Operation::Copy, " Copy ", "Copying"),
        (Operation::Move, " Move ", "Moving"),
        (Operation::Delete, " Delete ", "Deleting"),
    ] {
        assert_eq!(title(operation), title_text);
        let text = rows(&progress_dialog(
            screen(),
            &snapshot(operation, Phase::Working, "x"),
        ))
        .join("\n");
        assert!(text.contains(heading), "{text}");
    }
}

#[test]
fn a_long_path_keeps_its_end() {
    assert_eq!(tail("/a/b/c/file.txt", 9), "…file.txt");
    assert_eq!(tail("short", 9), "short");
    // Wide characters are measured in cells.
    assert_eq!(tail("/dir/日本語", 5), "…本語");
}

#[test]
fn the_bar_fills_its_width() {
    assert_eq!(bar(0, 10), "░░░░    0%");
    assert_eq!(bar(500, 10), "██░░   50%");
    assert_eq!(bar(1000, 10), "████  100%");
}

#[test]
fn a_small_screen_gets_a_small_dialog() {
    let buffer = progress_dialog(
        Rect::new(0, 0, 30, 5),
        &snapshot(Operation::Copy, Phase::Working, "x"),
    );
    assert_eq!(buffer.area, Rect::new(1, 0, 28, 5));
}

/// A canonicalized panel path is Windows' verbatim `\\?\C:\…`; the dialog
/// shows it as people type it.
#[cfg(windows)]
#[test]
fn a_verbatim_windows_path_is_shown_without_its_prefix() {
    let text = rows(&progress_dialog(
        screen(),
        &snapshot(Operation::Copy, Phase::Working, r"\\?\C:\work\a.txt"),
    ))
    .join("\n");
    assert!(text.contains(r"C:\work\a.txt"), "{text}");
    assert!(!text.contains(r"\\?\"), "{text}");
}
