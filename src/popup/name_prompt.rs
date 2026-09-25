//! Name prompt controls, acceptance, and what Enter makes: a rename, a
//! directory, or a link.

use crate::models::{App, LinkType, NameField, NamePrompt, NamePurpose, Overlay};
use crossterm::event::{KeyCode, KeyModifiers};

use crate::status::Status;
use crate::utils::refresh_panels;

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let directory = app.get_inactive_panel_ref().path.clone();
    let Overlay::NamePrompt(prompt) = &mut app.overlay else {
        return false;
    };
    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Tab | KeyCode::Down => prompt.advance(true),
        KeyCode::BackTab | KeyCode::Up => prompt.advance(false),
        // The arrows belong to the control under the cursor, and the
        // only one that answers them is the drop-down.
        KeyCode::Right => step_link(&directory, prompt, true),
        KeyCode::Left => step_link(&directory, prompt, false),
        KeyCode::Backspace => prompt.backspace(),
        KeyCode::Enter => {
            let cancelled = prompt.field() == NameField::Cancel;
            let settings = prompt.clone();
            app.close_overlay();
            // Nothing to do is the usual case, not an error: a blank
            // name, or a rename to the name it already has. Closing
            // quietly is the right answer to all of them.
            if !cancelled {
                apply_name(app, &settings);
            }
        }
        // A space is a space in a text field and a toggle everywhere
        // else, which is the only reading that leaves both usable.
        KeyCode::Char(' ') if !prompt.field().is_text() => prompt.toggle(),
        KeyCode::Char(ch)
            if !modifiers.contains(KeyModifiers::CONTROL)
                && !modifiers.contains(KeyModifiers::ALT) =>
        {
            prompt.push(ch);
        }
        _ => {}
    }
    true
}

/// Step the drop-down, passing over a kind the target cannot have.
///
/// Far's dialog lists a hard link for a file and not for a folder. The list
/// here is fixed per purpose, so the skip happens at the arrow instead: a
/// choice the arrows never stop on is a choice that is not offered.
fn step_link(directory: &std::path::Path, prompt: &mut NamePrompt, forward: bool) {
    for _ in 0..prompt.purpose().link_choices().len() {
        prompt.cycle(forward);
        if crate::ops::link::is_possible(directory, prompt) {
            return;
        }
    }
}

/// Carry out whatever the name prompt was asking for.
fn apply_name(app: &mut App, prompt: &NamePrompt) {
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
        let shown = first.split(std::path::is_separator).next().unwrap_or(first);
        settle_on(app, shown);
    }
    app.set_status(match (failure, made) {
        (Some(failed), _) => failed,
        (None, 1) => Status::CreatedDir(directory.join(first).display().to_string()),
        (None, count) => Status::CreatedDirs { count, directory },
    });
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

#[cfg(test)]
mod tests;
