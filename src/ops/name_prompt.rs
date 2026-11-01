//! Filesystem effects accepted by the name prompt.

use crate::models::{App, LinkType, NamePrompt, NamePurpose};
use crate::status::Status;
use crate::utils::refresh_panels;

/// Carry out whatever the name prompt was asking for.
pub fn apply(app: &mut App, prompt: &NamePrompt) {
    match prompt.purpose() {
        NamePurpose::Rename => apply_rename(app, prompt),
        NamePurpose::MakeFolder => apply_make_folder(app, prompt),
        NamePurpose::Link => crate::ops::link::create(app, prompt),
    }
}

/// Shift+F6: one entry, one new name.
fn apply_rename(app: &mut App, prompt: &NamePrompt) {
    let Some(target) = prompt.target() else {
        return;
    };
    let directory = app.get_active_panel().path.clone();
    let destination = directory.join(target);

    // Refuse rather than overwrite. Far asks; this at least does not act.
    // A link that points nowhere holds its name too.
    if crate::utils::name_is_taken(&destination) {
        app.set_status(Status::AlreadyExists(target.into()));
        return;
    }
    match std::fs::rename(directory.join(prompt.original()), &destination) {
        Ok(()) => {
            settle_on(app, target);
            app.set_status(Status::CommandOutput(format!("Renamed to {target}")));
        }
        Err(err) => app.set_status(Status::CommandOutput(err.to_string())),
    }
}

/// F7: one directory, or a link standing for one, or a `;`-separated list of
/// either.
///
/// The first failure stops the run and is reported. Carrying on past it would
/// leave the user reading a status line about the last name while an earlier
/// one silently did not appear.
fn apply_make_folder(app: &mut App, prompt: &NamePrompt) {
    let names = prompt.names();
    let Some(first) = names.first().copied() else {
        return;
    };
    let directory = app.get_active_panel().path.clone();

    let mut made = 0_usize;
    let mut failure = None;
    for name in &names {
        let outcome = create_folder(&directory, name, prompt);
        if let Err(err) = outcome {
            failure = Some(err);
            break;
        }
        made += 1;
    }

    if made > 0 {
        // The cursor goes to what was just created, as Far's does: the new
        // directory is what the user is about to enter, and hunting for it in
        // a long listing is the work the dialog was meant to save. For a path
        // that is its first folder, the one this panel shows.
        settle_on(app, first_normal_component(&directory, first));
    }
    app.set_status(match (failure, made) {
        (Some(failed), _) => failed,
        (None, 1) => Status::CreatedDir(directory.join(first).display().to_string()),
        (None, count) => Status::CreatedDirs { count, directory },
    });
}

fn first_normal_component<'a>(directory: &std::path::Path, first: &'a str) -> &'a str {
    let rel = std::path::Path::new(first);
    rel.strip_prefix(directory)
        .unwrap_or(rel)
        .components()
        .find_map(|c| match c {
            std::path::Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .unwrap_or(first)
}

fn create_folder(
    directory: &std::path::Path,
    name: &str,
    prompt: &NamePrompt,
) -> Result<(), Status> {
    let destination = directory.join(name);
    if crate::utils::name_is_taken(&destination) {
        return Err(Status::AlreadyExists(name.into()));
    }
    // The folders a path names on the way are made as Far makes them; the
    // name itself is claimed exclusively, as it was just found free.
    let parents = destination.parent().map_or(Ok(()), std::fs::create_dir_all);
    match prompt.link() {
        LinkType::None => parents.and_then(|()| std::fs::create_dir(&destination)),
        LinkType::Symbolic => {
            parents.and_then(|()| crate::utils::symlink_dir(prompt.link_target(), &destination))
        }
        // F7 never offers a hard link — see `NamePurpose::link_choices`.
        LinkType::Hard => Err(std::io::Error::other("a folder cannot be hard-linked")),
    }
    .map_err(|error| Status::CommandOutput(error.to_string()))
}

/// Re-read both panels and leave the cursor on `name`.
///
/// The reload is what makes the entry exist to point at, so the two belong
/// together: a caller that did one without the other would put the cursor on
/// the row the name held *before* the listing changed.
fn settle_on(app: &mut App, name: &str) {
    // The entry exists either way; a listing that cannot be re-read keeps its
    // old rows, and the cursor then simply finds nothing to settle on.
    drop(refresh_panels(app));
    app.get_active_panel().select_named(name);
}
