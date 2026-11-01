//! Purpose: The progress dialog's frame and what every operation shares: the
//! title and heading, the current path cut from its start, a search's counts,
//! and a small screen.

use std::path::PathBuf;
use std::time::Duration;

use super::*;
use crate::progress::Tally;

/// A snapshot of `operation`, with one file done and nothing being sent.
pub(super) fn snapshot(operation: Operation, phase: Phase, current: &str) -> Snapshot {
    Snapshot {
        operation,
        phase,
        current: PathBuf::from(current),
        transfer: None,
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

/// The dialog's rows as text, frame included.
pub(super) fn rows(buffer: &Buffer) -> Vec<String> {
    let area = buffer.area;
    (area.y..area.y + area.height)
        .map(|y| {
            (area.x..area.x + area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect()
        })
        .collect()
}

pub(super) fn screen() -> Rect {
    Rect::new(0, 0, 100, 30)
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
fn every_operation_has_its_title_and_heading() {
    for (operation, title_text, heading) in [
        (Operation::Copy, " Copy ", "Copying the file"),
        (Operation::Move, " Rename/Move ", "Moving the file"),
        (Operation::Delete, " Delete ", "Deleting the file"),
        (Operation::Find, " Find ", "Searching in"),
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
fn a_small_screen_gets_a_small_dialog() {
    let buffer = progress_dialog(
        Rect::new(0, 0, 30, 5),
        &snapshot(Operation::Copy, Phase::Working, "x"),
    );
    assert_eq!(buffer.area, Rect::new(1, 0, 28, 5));
}

/// A search has no total to fill a bar with: it says where it is, how many
/// matches it found and how many entries it looked at.
#[test]
fn a_search_shows_its_folder_and_counts_without_a_bar() {
    let text = rows(&progress_dialog(
        screen(),
        &snapshot(Operation::Find, Phase::Scanning, "/src/deep"),
    ))
    .join("\n");
    assert!(text.contains(" Find "), "{text}");
    assert!(text.contains("Searching in"), "{text}");
    assert!(text.contains("/src/deep"), "{text}");
    assert!(text.contains("Found: 1   Checked: 4"), "{text}");
    assert!(!text.contains('░') && !text.contains('%'), "{text}");
}

fn path_row(path: &str, width: usize) -> String {
    let state = snapshot(Operation::Copy, Phase::Working, path);
    lines(&state, width)[1]
        .spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

#[test]
fn a_long_path_keeps_its_end() {
    assert_eq!(path_row("/a/b/c/file.txt", 9), "…file.txt");
    assert_eq!(path_row("short", 9), "short");
    // Wide characters are measured in cells.
    assert_eq!(path_row("/dir/日本語", 5), "…本語");
}

#[test]
fn a_cut_path_respects_empty_width_and_drops_orphaned_marks() {
    assert_eq!(path_row("/tmp/name", 0), "");
    assert_eq!(path_row("/tmp/e\u{0301}f", 2), "…f");
    assert_eq!(path_row("/tmp/e\u{0301}f", 3), "…e\u{0301}f");
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
