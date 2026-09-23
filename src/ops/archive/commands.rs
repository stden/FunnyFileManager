//! Purpose: `Shift+F3` — run the chosen Archive commands row on the archive
//! under the panel cursor.
//!
//! The menu is a launcher, not a second implementation. Test and List run
//! the readers and report; the extract and add rows open the `Shift+F2` and
//! `Shift+F1` dialogs with their one obvious answer filled in, so the same
//! code unpacks and packs whichever way the user arrived.

use std::path::Path;

use crate::models::{App, ArchiveCommand, ArchiveFormat, ArchivePrompt, Overlay, Viewer};
use crate::status::Status;

/// Take the menu down and run `command` on the archive under the cursor.
///
/// The menu closes first: a row that opens another dialog replaces it, and a
/// row that only reports leaves the panels in view under the status line.
pub fn run_command(app: &mut App, command: ArchiveCommand) {
    app.close_overlay();
    let Some(archive) = app.get_active_panel_ref().selected_path() else {
        app.set_status(Status::Error(
            "Archive commands: put the cursor on an archive".to_owned(),
        ));
        return;
    };
    match command {
        ArchiveCommand::Test => test(app, &archive),
        ArchiveCommand::List => list(app, &archive),
        ArchiveCommand::ExtractHere => {
            let here = app.get_active_panel_ref().path.clone();
            extract_to(app, &here);
        }
        ArchiveCommand::ExtractToOtherPanel => {
            let other = app.get_inactive_panel_ref().path.clone();
            extract_to(app, &other);
        }
        ArchiveCommand::AddSelected => add_to(app, &archive),
    }
}

fn test(app: &mut App, archive: &Path) {
    match super::test_archive(archive) {
        Ok(()) => app.set_status(Status::CommandOutput(format!(
            "Archive OK: {}",
            name_of(archive)
        ))),
        Err(error) => fail(app, archive, &error),
    }
}

/// The listing goes to the viewer, titled by the archive's full path the way
/// a viewed file is, so Esc lands back on the panels with nothing to tidy.
fn list(app: &mut App, archive: &Path) {
    match super::list_archive(archive) {
        Ok(lines) => {
            app.set_overlay(Overlay::Viewer(Viewer::report(
                archive.display().to_string(),
                lines,
            )));
            app.set_status(Status::OpenedView(name_of(archive)));
        }
        Err(error) => fail(app, archive, &error),
    }
}

/// The Shift+F2 dialog, with the destination this row stands for.
fn extract_to(app: &mut App, destination: &Path) {
    super::open_extract(app);
    if let Overlay::ExtractPrompt(prompt) = &mut app.overlay {
        prompt.destination = destination.display().to_string();
    }
}

/// The Shift+F1 dialog on this archive: the marked files join it. The
/// archive itself is never one of them, and a format that cannot grow is
/// refused here rather than by the tool.
fn add_to(app: &mut App, archive: &Path) {
    let name = name_of(archive);
    let sources: Vec<String> = app
        .get_active_panel_ref()
        .action_entry_names()
        .into_iter()
        .filter(|source| *source != name)
        .collect();
    if sources.is_empty() {
        app.set_status(Status::Error(format!(
            "Add to archive: mark the files to add to {name}"
        )));
        return;
    }
    match ArchiveFormat::from_name(&name).filter(|format| format.appendable()) {
        Some(format) => {
            let prompt = ArchivePrompt::appending(sources, archive, format);
            app.set_overlay(Overlay::ArchivePrompt(Box::new(prompt)));
        }
        None => app.set_status(Status::Error(format!(
            "Add to archive: files can be added to .tar and .zip only, not {name}"
        ))),
    }
}

/// The reader's own words, in the status bar and — since the bar shows one
/// line and forgets it on the next key — in the console transcript too.
fn fail(app: &mut App, archive: &Path, error: &std::io::Error) {
    let text = format!("{}: {error}", name_of(archive));
    app.console.record_error(&text);
    app.set_status(Status::Error(text));
}

fn name_of(archive: &Path) -> String {
    archive.file_name().map_or_else(
        || archive.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    )
}
