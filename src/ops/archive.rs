//! Purpose: Archives (`Shift+F1` / `Shift+F2` / `Shift+F3`): the three dialogs share the `tar`, `zip` and `unzip` runs, so the runs live here once.
//!
//! The tools are the platform's own. They run without a shell — every path
//! reaches the tool as one argument, whatever is in it — and the line echoed
//! in the console transcript is the shell command that would do the same, so
//! what the user reads is what ran. Nothing here reads a dialog's keys, that
//! is `popup`'s, and nothing draws.

use std::io;
use std::path::Path;

use crate::models::{App, ArchiveMenu, ArchivePrompt, ExtractPrompt, Overlay, ARCHIVE_SUFFIXES};
use crate::status::Status;

mod add;
mod commands;
mod extract;
mod members;
mod tool;

pub use add::{add, command_line, create_archive, offered_formats, on_path, ArchiveRequest};
pub use commands::run_command;
pub use extract::{extract_archive, run_extract};
use members::validate_listing;
// `add` reaches `complaint` and `Run` as `super::…`.
use tool::{complaint, Run, Tool};

/// Shift+F1: open the Add to archive dialog for the marked entries or the one
/// under the cursor.
pub fn open_add(app: &mut App) {
    let panel = app.get_active_panel_ref();
    let sources = panel.action_entry_names();
    if sources.is_empty() {
        app.set_status(Status::Error(
            "Add to archive: nothing to archive".to_owned(),
        ));
        return;
    }
    let single_is_dir = match sources.as_slice() {
        [only] => panel
            .get_selected_entry()
            .is_some_and(|entry| entry.name == *only && entry.kind.is_dir()),
        _ => false,
    };
    let folder_name = panel.path.file_name().map_or_else(
        || "archive".to_owned(),
        |name| name.to_string_lossy().into_owned(),
    );
    let stem = ArchivePrompt::suggest_stem(&sources, single_is_dir, &folder_name);
    let other = app.get_inactive_panel_ref().path.clone();
    let prompt = ArchivePrompt::new(sources, &stem, &other, offered_formats());
    app.set_overlay(Overlay::ArchivePrompt(Box::new(prompt)));
}

/// Shift+F2: open the Extract files dialog for the selected archives,
/// offering the other panel as the destination.
///
/// Opens even when nothing selected is an archive: the dialog then says so
/// with its Extract button dimmed, which tells the user more than a status
/// line they may not be looking at.
pub fn open_extract(app: &mut App) {
    let panel = app.get_active_panel_ref();
    let archives = panel
        .action_paths()
        .into_iter()
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| is_archive(&name.to_string_lossy()))
        })
        .collect();
    let destination = app.get_inactive_panel_ref().path.clone();
    app.set_overlay(Overlay::ExtractPrompt(Box::new(ExtractPrompt::new(
        archives,
        &destination,
    ))));
}

/// Shift+F3: open the Archive commands menu.
pub fn open_commands(app: &mut App) {
    app.set_overlay(Overlay::ArchiveCommands(ArchiveMenu::default()));
}

/// Whether `name` ends in one of the suffixes the dialogs read.
///
/// Case-insensitive, and the suffix must follow a stem: `.zip` on its own is
/// a dotfile, not an archive.
#[must_use]
pub fn is_archive(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    ARCHIVE_SUFFIXES
        .iter()
        .any(|suffix| lower.len() > suffix.len() && lower.ends_with(suffix))
}

/// What `archive` holds, one line per member as its reader prints it —
/// `tar -tvf` for the tar family, `unzip -l` for a zip.
pub fn list_archive(archive: &Path) -> io::Result<Vec<String>> {
    let text = Tool::for_archive(archive)?.listing(archive).text()?;
    Ok(text.lines().map(str::to_owned).collect())
}

/// Read `archive` end to end without unpacking anything.
///
/// A truncated stream, a member whose checksum no longer matches, or a file
/// that is not what its name says all come back as the reader's own words.
pub fn test_archive(archive: &Path) -> io::Result<()> {
    Tool::for_archive(archive)?.check(archive).text().map(drop)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_existing;

#[cfg(test)]
mod tests_extract;
#[cfg(test)]
mod tests_guards;
#[cfg(test)]
mod tests_tool_failures;
