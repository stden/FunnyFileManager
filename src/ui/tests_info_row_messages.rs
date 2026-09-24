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
use crate::status::{FailedEntry, Status};

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

/// Save setup names the file it wrote, however deep the settings directory
/// sits. The line used to be cut at its end: on the Windows CI runner the
/// message with its temp path was 83 cells for a 78-cell row, and
/// `settings.yaml` was what fell off.
#[test]
fn a_saved_setup_names_the_settings_file_however_deep_it_is() -> std::io::Result<()> {
    let temp = tempfile::TempDir::new()?;
    let path = temp
        .path()
        .join("a-settings-directory-longer-than-the-info-row")
        .join("settings.yaml");
    let panel = Panel::new(temp.path().to_owned())?;
    let mut app = App::with_panels(panel.clone(), panel);
    crate::ops::menu::settings::load(&mut app, &path)?;
    crate::ops::menu::settings::save(&mut app)?;

    let row = info_row(app.status, 80, 24);
    assert!(row.starts_with("Setup saved to …"), "{row:?}");
    assert!(row.ends_with("settings.yaml"), "{row:?}");
    Ok(())
}

/// A copy into a deep folder names the file it made, and says it was
/// renamed, before any of the folder's path: at 130 columns the row is 63
/// cells, exactly the message with the path cut to its file name.
#[test]
fn a_copy_names_the_file_it_made_however_deep_the_folder() {
    let copied = Status::Transferred {
        purpose: crate::copy::Purpose::Copy,
        landed: crate::status::Landed::At(PathBuf::from(
            "/srv/x/agents/v-copy-3/work/a/deep/folder/notes_1.txt",
        )),
        skipped: 0,
        renamed: 1,
    };
    assert_eq!(
        info_row(copied, 130, 30),
        "Copied to …/notes_1.txt (existing files kept, new ones renamed)"
    );
}

/// A link refused for a name already taken says which name and why, even
/// with the whole of the other panel's path in front of it.
#[test]
fn a_taken_link_name_keeps_the_name_and_the_reason() {
    let taken = Status::AlreadyExists(PathBuf::from(
        "/srv/x/agents/v-link-2/work/the/other/panel/notes.txt",
    ));
    let row = info_row(taken, 80, 24);
    assert!(
        row.starts_with('…') && row.ends_with("/notes.txt already exists"),
        "{row:?}"
    );
}

/// A program deep in the tree still reports how it ended: the exit status is
/// what the user ran it to see.
#[test]
fn a_finished_program_keeps_its_name_and_its_exit_status() {
    let status = std::process::ExitStatus::default();
    let ran = Status::ProgramExited {
        program: PathBuf::from("/srv/x/agents/v-run-4/work/project/scripts/run.sh"),
        status,
    };
    let row = info_row(ran, 80, 24);
    assert!(
        row.starts_with('…') && row.ends_with(&format!("/run.sh: {status}")),
        "{row:?}"
    );
}

/// A file deep in the tree that a batch failed on first.
fn deep_failure() -> FailedEntry {
    FailedEntry::new(
        "/srv/x/agents/v-copy-3/work/a/deep/folder/notes.txt",
        &"Permission denied (os error 13)",
    )
}

/// A copy, a move, a delete and a wipe of 3 entries that each failed on one.
fn failed_batches() -> [Status; 4] {
    [
        Status::CopiedWithErrors {
            count: 3,
            error_count: 1,
            first_error: deep_failure(),
        },
        Status::MovedWithErrors {
            count: 3,
            error_count: 1,
            first_error: deep_failure(),
        },
        Status::DeletedWithErrors {
            count: 3,
            error_count: 1,
            first_error: deep_failure(),
        },
        Status::WipedWithErrors {
            count: 3,
            error_count: 1,
            first_error: deep_failure(),
        },
    ]
}

/// A partly failed batch names the file it failed on and why, however deep
/// it sits: the head of its path gives way, not the name or the reason.
#[test]
fn a_partial_failure_keeps_the_failed_file_name_and_the_reason() {
    let verbs = ["Copied", "Moved", "Deleted", "Wiped"];
    for (verb, failure) in verbs.into_iter().zip(failed_batches()) {
        let row = info_row(failure, 120, 40);
        let kept = format!("{verb} 3 item(s), 1 error(s): …/notes.txt: Permission");
        assert!(row.starts_with(&kept), "{row:?}");
    }
}

/// Ctrl+A that failed on a deep file names it before the reason gives way.
#[test]
fn a_partial_attribute_failure_keeps_the_failed_file_name() {
    let attributes = Status::AttributesSetWithErrors {
        count: 2,
        error_count: 1,
        first_error: deep_failure(),
    };
    assert_eq!(
        info_row(attributes, 120, 40),
        "Attributes set on 2 object(s), 1 failed: …/notes.txt: Per…"
    );
}

/// F7 with several names counts them and names the folder they went into,
/// even on an 80-column screen's 38-cell row.
#[test]
fn several_new_folders_keep_the_name_of_their_folder() {
    let made = Status::CreatedDirs {
        count: 3,
        directory: PathBuf::from("/srv/x/agents/v-mkdir-1/work/deep/folder/target"),
    };
    let row = info_row(made, 80, 24);
    assert!(
        row.starts_with("Created 3 folders in …") && row.ends_with("/folder/target"),
        "{row:?}"
    );
}

/// A link names itself by its name — it is under the other panel's cursor —
/// and gives the room to its target, which Alt+F6 fills in absolute.
#[test]
fn a_new_link_keeps_the_name_of_its_target() {
    let linked = Status::Linked {
        link: PathBuf::from("/srv/x/agents/v-link-2/work/here/notes.lnk"),
        target: PathBuf::from("/srv/x/agents/v-link-2/work/the/active/panel/notes.txt"),
    };
    let row = info_row(linked, 80, 24);
    assert!(
        row.starts_with("Link notes.lnk -> …") && row.ends_with("/panel/notes.txt"),
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
