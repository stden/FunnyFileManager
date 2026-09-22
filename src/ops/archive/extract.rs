//! Purpose: `Shift+F2` — unpack every archive the Extract files dialog holds
//! into its destination, each one checked before `tar` or `unzip` touches the
//! disk.

use std::io;
use std::path::{Path, PathBuf};
use std::process::Output;

use super::{complaint, validate_listing, Tool};
use crate::models::{App, Overlay};
use crate::status::Status;

/// Unpack `archive` into `destination`, creating the folder if needed.
///
/// The members are listed and checked first: an archive that would write
/// outside `destination`, or leave a link in it, is refused before anything
/// is written. The error carries the archive's name and the tool's first
/// complaint, so a status line built from it says which of several archives
/// went wrong.
pub fn extract_archive(archive: &Path, destination: &Path) -> io::Result<Output> {
    let name = archive.file_name().unwrap_or_default().to_string_lossy();
    let named = |error: io::Error| io::Error::other(format!("{name}: {error}"));
    let tool = Tool::for_archive(archive).map_err(named)?;
    validate_listing(
        &tool.names(archive).text().map_err(named)?,
        &tool.details(archive).text().map_err(named)?,
    )
    .map_err(named)?;
    std::fs::create_dir_all(destination)?;
    let output = tool
        .unpack(archive, destination)
        .execute(destination)
        .map_err(named)?;
    if output.status.success() {
        return Ok(output);
    }
    Err(named(io::Error::other(complaint(&output))))
}

/// Enter in the Extract files dialog: unpack every archive it holds into its
/// destination, then say what happened and re-read both panels.
pub fn run_extract(app: &mut App) {
    let Overlay::ExtractPrompt(prompt) = &app.overlay else {
        return;
    };
    let prompt = (**prompt).clone();
    if prompt.destination.trim().is_empty() {
        app.set_status(Status::CommandOutput(
            "Extract files: the destination is empty".to_owned(),
        ));
        return;
    }
    app.close_overlay();
    if !prompt.can_extract() {
        app.set_status(Status::CommandOutput(prompt.caption()));
        return;
    }

    let destination = resolve_destination(app, &prompt.destination);
    let outcome = extract_all(app, &prompt.archives, &destination);
    if outcome.first_error.is_none() {
        // Far spends the selection on a finished operation; a failure keeps it
        // so the same files can be retried.
        app.get_active_panel().clear_marks();
    }
    app.set_status(outcome.summary(prompt.archives.len(), &destination));
    // The archives may have unpacked into either panel; a refresh failure is
    // not the extraction's failure, so it is deliberately not propagated.
    let _ = crate::utils::refresh_panels(app);
}

/// A typed relative destination is taken from the active panel, the way a
/// `cd` on the command line would be.
fn resolve_destination(app: &App, typed: &str) -> PathBuf {
    let typed = Path::new(typed.trim());
    if typed.is_absolute() {
        return typed.to_path_buf();
    }
    app.get_active_panel_ref().path.join(typed)
}

/// How a batch of extractions went.
struct Outcome {
    done: usize,
    first_error: Option<io::Error>,
}

impl Outcome {
    fn summary(&self, total: usize, destination: &Path) -> Status {
        let noun = if total == 1 { "archive" } else { "archives" };
        Status::CommandOutput(match &self.first_error {
            None => format!("Extracted {total} {noun} to {}", destination.display()),
            Some(error) => format!(
                "Extracted {} of {total} {noun}, first error: {error}",
                self.done
            ),
        })
    }
}

fn extract_all(app: &mut App, archives: &[PathBuf], destination: &Path) -> Outcome {
    let mut outcome = Outcome {
        done: 0,
        first_error: None,
    };
    for archive in archives {
        match extract_one(app, archive, destination) {
            Ok(()) => outcome.done += 1,
            Err(error) => {
                outcome.first_error.get_or_insert(error);
            }
        }
    }
    outcome
}

/// One archive, with its command and everything it printed left in the
/// console transcript (`Ctrl+O`), the way Far prints to the console screen.
fn extract_one(app: &mut App, archive: &Path, destination: &Path) -> io::Result<()> {
    let command = Tool::for_archive(archive).map_or_else(
        |_| format!("extract {}", archive.display()),
        |tool| tool.unpack(archive, destination).command_line(),
    );
    app.console.record_command(destination, &command);
    match extract_archive(archive, destination) {
        Ok(output) => {
            app.console
                .record_output(&String::from_utf8_lossy(&output.stdout));
            app.console
                .record_error(&String::from_utf8_lossy(&output.stderr));
            Ok(())
        }
        Err(error) => {
            app.console.record_error(&error.to_string());
            Err(error)
        }
    }
}
