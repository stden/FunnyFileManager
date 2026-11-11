//! Purpose: Progress remains useful across a multi-file transfer or delete.

use super::*;

#[test]
fn transfer_shows_current_file_and_total_progress() {
    let mut state = moving();
    state.done = Tally {
        files: 2,
        bytes: 50,
    };
    state.total = Tally {
        files: 10,
        bytes: 100,
    };
    state.transfer.as_mut().expect("transfer").file = FileProgress::Copying {
        done: 25,
        size: 100,
    };
    let text = dialog(&state).join("\n");

    assert!(text.contains("25%"), "current file: {text}");
    assert!(text.contains("50%"), "whole operation: {text}");
    assert!(text.contains("2 of 10"), "completed files: {text}");
    assert!(text.contains("50 of 100"), "completed bytes: {text}");
}

#[test]
fn deletion_shows_completed_files_and_rate_is_labelled_as_speed() {
    let mut state = moving();
    state.operation = Operation::Delete;
    state.transfer = None;
    state.done.files = 2;
    state.total.files = 10;
    let text = dialog(&state).join("\n");

    assert!(text.contains("2 of 10"), "completed files: {text}");
    assert!(
        !text.contains("Remaining:"),
        "rate is not time remaining: {text}"
    );
}

#[test]
fn short_terminal_preserves_transfer_counts_and_elapsed_time() {
    let mut state = moving();
    state.done = Tally {
        files: 2,
        bytes: 50,
    };
    state.total = Tally {
        files: 10,
        bytes: 100,
    };
    let text = rows(&progress_dialog(Rect::new(0, 0, 80, 8), &state)).join("\n");

    assert!(text.contains("2 of 10"), "completed files: {text}");
    assert!(text.contains("50 of 100"), "completed bytes: {text}");
    assert!(text.contains("00:00:40"), "elapsed time: {text}");
}

#[test]
fn short_terminal_preserves_delete_counts_and_elapsed_time() {
    let mut state = moving();
    state.operation = Operation::Delete;
    state.transfer = None;
    state.done.files = 2;
    state.total.files = 10;
    let text = rows(&progress_dialog(Rect::new(0, 0, 80, 8), &state)).join("\n");

    assert!(text.contains("2 of 10"), "completed files: {text}");
    assert!(text.contains("00:00:40"), "elapsed time: {text}");
}
