//! Purpose: unit tests for the "File already exists" warning: both entries' facts and every answer on screen.

use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use ratatui::Terminal;

use super::{facts_line, render};
use crate::copy::CopyPrompt;
use crate::models::{Clash, EntryFacts, ExistingFile};

fn file(size: u64, seconds: u64) -> EntryFacts {
    EntryFacts {
        size: Some(size),
        modified: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds)),
        is_folder: false,
    }
}

fn drawn(question: &ExistingFile) -> String {
    let mut terminal = Terminal::new(crate::WritableTestBackend::new(80, 24)).expect("terminal");
    let mut tracker = crate::mouse::DialogMouse::default();
    terminal
        .draw(|frame| render(frame, &mut tracker, question))
        .expect("draw");
    crate::test_support::buffer_text(&terminal)
}

fn one_clash() -> ExistingFile {
    let clash = Clash {
        source: PathBuf::from("src").join("report.txt"),
        dest: PathBuf::from("dst").join("report.txt"),
        incoming: file(12_345, 2_000_000_000),
        existing: file(1_000, 1_000_000_000),
    };
    ExistingFile::new(
        CopyPrompt::new("report.txt", &PathBuf::from("dst")),
        vec![clash.source.clone()],
        PathBuf::from("src"),
        vec![clash],
    )
}

#[test]
fn the_warning_shows_both_entries_and_every_answer() {
    let screen = drawn(&one_clash());
    for expected in [
        "Destination already exists",
        "report.txt",
        "New",
        "12,345",
        "Existing",
        "1,000",
        "Remember choice",
        "Overwrite",
        "Skip",
        "Keep both",
        "Cancel",
    ] {
        assert!(
            screen.contains(expected),
            "{expected:?} missing from:\n{screen}"
        );
    }
    assert!(
        !screen.contains("(1 of 1)"),
        "a single clash is not counted"
    );
}

#[test]
fn a_folder_reads_as_a_folder_and_an_unknown_time_as_a_question_mark() {
    let folder = EntryFacts {
        size: None,
        modified: None,
        is_folder: true,
    };
    let line = facts_line("Existing", &folder);
    assert!(line.contains("<Folder>"), "{line}");
    assert!(line.ends_with('?'), "{line}");
}

#[test]
fn the_sizes_of_both_lines_end_in_one_column() {
    let new = facts_line("New", &file(5, 0));
    let old = facts_line("Existing", &file(5_000_000, 0));
    assert_eq!(
        new.find(" 1970").or_else(|| new.find(" 1969")),
        old.find(" 1970").or_else(|| old.find(" 1969"))
    );
}
