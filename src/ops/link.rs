//! Purpose: Link (`Alt+F6`) — a symbolic or hard link to the entry under the
//! cursor, named in the other panel by default.
//!
//! The same dialog F7 uses, with a different question: the target is known —
//! it is what the cursor is on — and what is asked for is the link's name and
//! kind. Far's list also has a directory junction; that is a Windows idea, so
//! it is not offered. A hard link is offered for a file only, because no
//! kernel this runs on will hard-link a directory, and the dialog should not
//! hold out a choice that can only fail.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::models::{App, LinkType, NamePrompt, Overlay, PanelSide};
use crate::status::Status;
use crate::utils::refresh_panels;

/// `Alt+F6`: ask what to call a link to the selected entry.
///
/// The name field opens on the other panel's directory joined with the
/// entry's own name, as Far's does: the commonest link is a namesake in the
/// place the user has already navigated the other panel to.
pub fn open(app: &mut App) {
    let target = app
        .get_active_panel_ref()
        .get_selected_entry()
        .filter(|entry| entry.name != "..")
        .map(|entry| app.get_active_panel_ref().path.join(&entry.name));
    let Some(target) = target else {
        app.set_status(Status::CommandOutput("Nothing to link".to_owned()));
        return;
    };
    let suggested = target
        .file_name()
        .map(|name| app.get_inactive_panel_ref().path.join(name))
        .unwrap_or_default();
    let prompt = NamePrompt::link_to(&target, suggested.display().to_string());
    app.set_overlay(Overlay::NamePrompt(prompt));
}

/// Whether the kind the drop-down shows can be made for the target it names.
///
/// The only combination that cannot is a hard link to a directory. The
/// answer is read from the disk rather than from the prompt because the
/// Target field is free text: what matters is what it says now.
#[must_use]
pub fn is_possible(directory: &Path, prompt: &NamePrompt) -> bool {
    prompt.link() != LinkType::Hard
        || link_path(directory, prompt)
            .is_none_or(|link| !is_directory(&relative_target(prompt.link_target(), &link)))
}

/// A directory in its own right. `symlink_metadata`, so a link to a directory
/// is judged as the link it is — the decision stays on the entry itself.
fn is_directory(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_dir())
}

/// Enter on the Link dialog: make the link, then show it.
pub fn create(app: &mut App, prompt: &NamePrompt) {
    let Some(link) = link_path(&app.get_inactive_panel_ref().path, prompt) else {
        return;
    };
    // `symlink_metadata` rather than `exists`: a dangling link at that name
    // is still a name that is taken, and `exists` would say it is free.
    if fs::symlink_metadata(&link).is_ok() {
        let taken = link.display();
        app.set_status(Status::CommandOutput(format!("{taken} already exists")));
        return;
    }
    let target = prompt.link_target();
    match make(prompt.link(), target, &link) {
        Ok(()) => {
            settle_on(app, &link);
            let made = link.display();
            app.set_status(Status::CommandOutput(format!("Link {made} -> {target}")));
        }
        Err(error) => app.set_status(Status::CommandOutput(error.to_string())),
    }
}

/// Where the link goes: the name as typed, and a bare name is relative to
/// the other panel — the only reading under which "in the other panel by
/// default" survives editing the field down to a bare name.
fn link_path(directory: &Path, prompt: &NamePrompt) -> Option<PathBuf> {
    let name = prompt.name().trim();
    if name.is_empty() {
        return None;
    }
    let path = Path::new(name);
    if path.is_absolute() {
        return Some(path.to_path_buf());
    }
    Some(directory.join(path))
}

/// Relative targets are read from the link's own directory, including when
/// the user has moved the link name into a subdirectory of the other panel.
fn relative_target(target: &str, link: &Path) -> PathBuf {
    link.parent().unwrap_or_else(|| Path::new("")).join(target)
}

fn make(kind: LinkType, target: &str, link: &Path) -> io::Result<()> {
    let resolved = relative_target(target, link);
    match kind {
        LinkType::Symbolic => symlink(target, link),
        LinkType::Hard if is_directory(&resolved) => {
            Err(io::Error::other("a folder cannot be hard-linked"))
        }
        LinkType::Hard => fs::hard_link(resolved, link),
        // Alt+F6 never offers it — see `NamePurpose::link_choices`.
        LinkType::None => Err(io::Error::other("a link needs a kind")),
    }
}

/// A symbolic link to `target`, whatever `target` is.
///
/// Windows has one call for a directory link and another for a file link,
/// and refuses each for the other; Unix does not care. The target is not
/// required to exist — a dangling link is a thing people deliberately make.
#[cfg(unix)]
fn symlink(target: &str, link: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, link)
}

#[cfg(windows)]
fn symlink(target: &str, link: &Path) -> io::Result<()> {
    if relative_target(target, link).is_dir() {
        std::os::windows::fs::symlink_dir(target, link)
    } else {
        std::os::windows::fs::symlink_file(target, link)
    }
}

/// Re-read both panels and put the cursor on the new link in whichever of
/// them lists its directory. The reload is what makes the entry exist to
/// point at, so the two belong together.
fn settle_on(app: &mut App, link: &Path) {
    // The link exists either way; a listing that cannot be re-read keeps its
    // old rows, and the cursor then simply finds nothing to settle on.
    drop(refresh_panels(app));
    let (Some(parent), Some(name)) = (link.parent(), link.file_name()) else {
        return;
    };
    let name = name.to_string_lossy();
    for side in [PanelSide::Left, PanelSide::Right] {
        let panel = app.panel_mut(side);
        if panel.path == parent {
            panel.select_named(&name);
        }
    }
}

#[cfg(test)]
mod tests;
