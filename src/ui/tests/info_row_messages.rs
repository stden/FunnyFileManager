//! Unit tests: a message too long for the active panel's info row gives up
//! the head of the path it quotes, never the file name or the reason.
//!
//! The row is the only place a refused view or a failed edit is reported, and
//! it is 58 cells wide on a 120-column terminal — narrower than most absolute
//! paths plus a reason.

use std::path::PathBuf;

use ratatui::{backend::TestBackend, Terminal};

use crate::models::{App, DirEntry, Panel};
use crate::status::Status;
use crate::ui::ui;

/// The left (active) panel's info row, inside its frame, after drawing
/// `status` on a `width` x `height` screen.
pub(super) fn info_row(status: Status, width: u16, height: u16) -> String {
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
