//! Unit tests: the rule that fits a message quoting a path into the info
//! row — the path gives way first, from its head, then the reason's end.

use super::info_row_messages::info_row;
use crate::status::Status;

/// A message that fits is shown whole, path and all.
#[test]
fn a_message_that_fits_is_not_cut() {
    let row = info_row(Status::NotRegularFile("/srv/x/fifo".to_owned()), 120, 40);
    assert_eq!(row, "Cannot open /srv/x/fifo: not a regular file");
}

// ── The fitter itself ─────────────────────────────────────────────────────

fn fit(before: &str, path: &str, after: &str, width: usize) -> String {
    super::fit_quoting_path(before, path, after, super::Cells::new(width))
}

#[test]
fn the_path_gives_way_first_from_its_head() {
    assert_eq!(
        fit("Cannot open ", "/srv/x/special/fifo", ": no", 30),
        "Cannot open …/special/fifo: no"
    );
    assert_eq!(
        fit("Viewing ", "/srv/x/a.txt", "", 60),
        "Viewing /srv/x/a.txt"
    );
}

/// Past the file name, the rest of the line is cut at its end instead.
#[test]
fn the_file_name_outlasts_the_end_of_the_reason() {
    assert_eq!(
        fit(
            "Cannot edit ",
            "/srv/x/unreadable.txt",
            ": Permission denied",
            36
        ),
        "Cannot edit …/unreadable.txt: Permi…"
    );
}

/// A message that quotes no path is cut at its end, as before.
#[test]
fn a_message_without_a_path_is_cut_at_its_end() {
    assert_eq!(
        fit("Copied 3 item(s), 1 error(s)", "", "", 12),
        "Copied 3 it…"
    );
    assert_eq!(fit("", "", "", 5), "");
}

#[test]
fn a_double_width_file_name_is_measured_in_cells() {
    let row = fit("Viewing ", "/srv/日本語/ファイル.txt", "", 22);
    assert_eq!(row, "Viewing …/ファイル.txt");
    for width in 0..30 {
        let row = fit("Viewing ", "/srv/日本語/ファイル.txt", "", width);
        assert!(super::Cells::of(&row).get() <= width, "{width}: {row:?}");
    }
}
