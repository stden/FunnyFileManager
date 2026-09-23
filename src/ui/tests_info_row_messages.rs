//! Unit tests: a message too long for the active panel's info row gives up
//! the head of the path it quotes, never the file name or the reason.
//!
//! The row is the only place a refused view or a failed edit is reported, and
//! it is 58 cells wide on a 120-column terminal — narrower than most absolute
//! paths plus a reason.

use std::path::PathBuf;

use ratatui::{backend::TestBackend, Terminal};

use super::ui;
use crate::models::{App, DirEntry, Panel};
use crate::status::Status;

/// The left (active) panel's info row, inside its frame, after drawing
/// `status` on a `width` x `height` screen.
fn info_row(status: Status, width: u16, height: u16) -> String {
    let panel = Panel::from_entries(
        PathBuf::from("/srv/x"),
        vec![DirEntry {
            name: "x.txt".to_owned(),
            ..DirEntry::default()
        }],
    );
    let mut app = App::with_panels(panel.clone(), panel);
    app.left_panel.select(Some(0));
    app.set_status(status);
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
    terminal.draw(|f| ui(f, &mut app)).expect("draw");
    // Under the info row: the panel's bottom border, the command line and
    // the key bar.
    let y = height - 4;
    let buffer = terminal.backend().buffer();
    (1..width / 2 - 1)
        .map(|x| buffer[(x, y)].symbol())
        .collect::<String>()
        .trim_end()
        .to_owned()
}

fn cannot_edit() -> Status {
    Status::CannotEdit {
        path: PathBuf::from("/srv/x/agents/v-editor-6/work/perms/unreadable.txt"),
        error: "Permission denied (os error 13)".to_owned(),
    }
}

#[test]
fn an_edit_error_at_120_columns_shows_the_file_name_and_the_reason() {
    let row = info_row(cannot_edit(), 120, 40);
    assert!(
        row.starts_with("Cannot edit …/unreadable.txt: Permission denied"),
        "{row:?}"
    );
}

/// At 80 columns the row is 38 cells: the path shrinks to its file name and
/// only then does the end of the reason give way.
#[test]
fn an_edit_error_at_80_columns_still_names_the_file() {
    let row = info_row(cannot_edit(), 80, 24);
    assert!(
        row.starts_with("Cannot edit …/unreadable.txt: Perm"),
        "{row:?}"
    );
}

#[test]
fn a_special_file_refusal_keeps_its_name_and_its_reason() {
    let fifo = || Status::NotRegularFile("/srv/x/agents/v-viewer-8/work/special/fifo".to_owned());
    let row = info_row(fifo(), 120, 40);
    assert!(
        row.starts_with("Cannot open …") && row.ends_with("/special/fifo: not a regular file"),
        "{row:?}"
    );
    assert_eq!(
        info_row(fifo(), 80, 24),
        "Cannot open …/fifo: not a regular file"
    );
}

#[test]
fn a_binary_refusal_keeps_the_file_name() {
    let binary = Status::BinaryFile("/srv/x/agents/v-viewer-8/work/docs/binary.bin".to_owned());
    let row = info_row(binary, 100, 30);
    assert!(
        row.starts_with("Cannot view binary file …") && row.ends_with("/docs/binary.bin"),
        "{row:?}"
    );
}

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
