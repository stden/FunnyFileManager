//! Purpose: `Shift+F2` — unpack every archive the Extract files dialog holds
//! into its destination, each one checked before `tar` or `unzip` touches the
//! disk.
//!
//! Neither tool is trusted with the destination itself: `tar` replaces an
//! existing file, `unzip -o` too, and `unzip` follows a link it finds there
//! to wherever it points. Each archive unpacks into an empty private folder
//! inside the destination instead, and its entries move in only when none of
//! them would replace something or pass through a link.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Output;

use super::{complaint, validate_listing, Tool};
use crate::models::{App, Overlay};
use crate::status::Status;

/// The name of the private folder an archive unpacks into first. A dotfile,
/// so a listing refreshed meanwhile does not show it as the user's.
const STAGING_PREFIX: &str = ".ffm-extract";

/// Unpack `archive` into `destination`, creating the folder if needed.
///
/// The members are listed and checked first: an archive that would write
/// outside `destination`, or leave a link in it, is refused before anything
/// is written. What unpacked then moves in only as a whole — never
/// replacing an existing entry, never through a link — or not at all. The
/// error carries the archive's name and the reason, so a status line built
/// from it says which of several archives went wrong.
pub fn extract_archive(archive: &Path, destination: &Path) -> io::Result<Output> {
    let name = archive.file_name().unwrap_or_default().to_string_lossy();
    let named = |error: io::Error| io::Error::other(format!("{name}: {error}"));
    let tool = Tool::for_archive(archive).map_err(named)?;
    validate_listing(
        &tool.names(archive).text().map_err(named)?,
        &tool.details(archive).text().map_err(named)?,
    )
    .map_err(named)?;
    fs::create_dir_all(destination)?;
    let staging = tempfile::Builder::new()
        .prefix(STAGING_PREFIX)
        .tempdir_in(destination)
        .map_err(named)?;
    let output = tool
        .unpack(archive, staging.path())
        .execute(staging.path())
        .map_err(named)?;
    if !output.status.success() {
        return Err(named(io::Error::other(complaint(&output))));
    }
    settle(staging.path(), destination).map_err(named)?;
    Ok(output)
}

/// Move what unpacked in `staging` into `destination`, or nothing at all.
fn settle(staging: &Path, destination: &Path) -> io::Result<()> {
    plain_tree(staging)?;
    check_clashes(staging, destination, Path::new(""))?;
    move_in(staging, destination)
}

/// Refuse a staged tree holding anything but plain files and folders. The
/// listing already refused links; this is what stands if one got past it,
/// because every later entry could be written through it.
pub(super) fn plain_tree(staging: &Path) -> io::Result<()> {
    plain_below(staging, Path::new(""))
}

#[expect(
    clippy::filetype_is_file,
    reason = "exactly regular files and folders may be extracted; anything else is refused"
)]
fn plain_below(staging: &Path, relative: &Path) -> io::Result<()> {
    for entry in fs::read_dir(staging.join(relative))? {
        let entry = entry?;
        let relative = relative.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            plain_below(staging, &relative)?;
        } else if !kind.is_file() {
            return Err(io::Error::other(format!(
                "{} is not a plain file or folder; nothing was extracted",
                relative.display()
            )));
        }
    }
    Ok(())
}

/// Refuse the first staged entry the destination already has, unless both
/// are real folders: those are merged, so what is inside them is compared
/// in turn. A link in the destination is never a real folder.
fn check_clashes(staging: &Path, destination: &Path, relative: &Path) -> io::Result<()> {
    for entry in fs::read_dir(staging.join(relative))? {
        let entry = entry?;
        let relative = relative.join(entry.file_name());
        let Some(existing) = existing(&destination.join(&relative))? else {
            continue;
        };
        if !(existing.is_dir() && entry.file_type()?.is_dir()) {
            return Err(clash(&relative, &existing));
        }
        check_clashes(staging, destination, &relative)?;
    }
    Ok(())
}

fn clash(relative: &Path, existing: &fs::Metadata) -> io::Error {
    let why = if existing.is_symlink() {
        "is a link, and extraction never writes through one"
    } else {
        "already exists"
    };
    io::Error::new(
        io::ErrorKind::AlreadyExists,
        format!("{} {why}; nothing was extracted", relative.display()),
    )
}

/// What is at `path`, the link itself when it is one; `None` when nothing.
fn existing(path: &Path) -> io::Result<Option<fs::Metadata>> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

/// Rename each staged entry into place, going down only into the folders
/// both sides have; `check_clashes` has made sure nothing is replaced.
fn move_in(staged: &Path, destination: &Path) -> io::Result<()> {
    for entry in fs::read_dir(staged)? {
        let entry = entry?;
        let target = destination.join(entry.file_name());
        if existing(&target)?.is_some_and(|metadata| metadata.is_dir()) {
            move_in(&entry.path(), &target)?;
        } else {
            fs::rename(entry.path(), &target)?;
        }
    }
    Ok(())
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
    drop(crate::utils::refresh_panels(app));
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
