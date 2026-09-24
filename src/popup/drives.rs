//! Drive-menu navigation and selection of cached drive rows.

use crate::models::{App, DriveEntry, Overlay, PanelSide};
use crossterm::event::KeyCode;

use crate::status::Status;
use std::path::PathBuf;

pub(super) fn process(app: &mut App, code: KeyCode) -> bool {
    let Overlay::DriveSelection {
        side: target_panel,
        selected_index,
        rows,
    } = &mut app.overlay
    else {
        return false;
    };
    // The rows were built when the dialog opened; a keypress must not
    // rescan the drives, which is a `statvfs` per drive.
    let drive_count = rows.len();
    let target_panel = *target_panel; // Copy it out

    match code {
        KeyCode::Esc | KeyCode::F(10) => app.close_overlay(),
        KeyCode::Up => *selected_index = drive_index_up(*selected_index, drive_count),
        KeyCode::Down => *selected_index = drive_index_down(*selected_index, drive_count),
        KeyCode::Home => *selected_index = 0,
        KeyCode::End if drive_count > 0 => *selected_index = drive_count - 1,
        KeyCode::Enter => {
            // Cloning the entry ends the `&mut app.overlay` borrow the
            // match bindings hold, so the helper can take the whole App.
            if let Some(row) = rows.get(*selected_index) {
                let selected = row.entry.clone();
                finish_selection(app, target_panel, selected);
            }
        }
        // Far's own shortcut: the letter of a drive, or the digit
        // beside a row under the rule, opens it without the cursor.
        // A key no row answers to is swallowed rather than guessed at,
        // which is what typing `Q` used to do — it built `Q:\` and
        // reported that the drive could not be opened.
        KeyCode::Char(ch) => {
            if let Some(entry) = crate::models::drive_by_key(rows, ch) {
                finish_selection(app, target_panel, entry);
            }
        }
        _ => {}
    }
    true
}

/// The menu closes before the row acts: a row that opens a window of its
/// own — the process list — must not have it closed again behind it.
fn finish_selection(app: &mut App, side: PanelSide, entry: DriveEntry) {
    app.close_overlay();
    let status = select_drive(app, side, entry);
    app.set_status(status);
}

/// Apply the drive picked in the Alt+F1 / Alt+F2 menu to `side`'s panel and
/// describe the outcome. An S3 profile is only acknowledged — the S3 browser is
/// not implemented yet, so the panel keeps showing its local directory.
/// `NetBox` turns the panel into the stored-sessions list; the process list
/// opens the task list, as Ctrl+W does, and says only why when it cannot.
fn select_drive(app: &mut App, side: PanelSide, drive: DriveEntry) -> Status {
    match drive {
        DriveEntry::Local(root) => {
            let path = drive_current_dir(app, root);
            match open_drive(app, side, path) {
                Ok(path) => Status::ChangedTo(path),
                Err(error) => Status::OpenDriveFailed { error },
            }
        }
        DriveEntry::S3Profile(profile) => Status::S3ProfileSelected(profile),
        DriveEntry::NetBox => crate::ops::sessions::open_on(app, side),
        DriveEntry::ProcessList => match crate::ops::tasks::open(app) {
            Ok(()) => Status::Cleared,
            Err(error) => Status::Error(error.to_string()),
        },
    }
}

/// Where a local drive opens. Far keeps a current directory per drive, so
/// picking the drive a panel is already on lands beside that panel rather than
/// at the root: the left panel deep in a project, Alt+F2, the same drive, and
/// the right panel shows the same folder. The active panel's folder counts
/// first — it is the one being worked in — then the other panel's; a drive
/// neither is on opens at `root`. A panel folder deleted since counts as the
/// nearest folder above it that still exists.
fn drive_current_dir(app: &App, root: PathBuf) -> PathBuf {
    [app.get_active_panel_ref(), app.get_inactive_panel_ref()]
        .into_iter()
        .map(|panel| &panel.path)
        .find(|path| path.starts_with(&root))
        .and_then(|path| crate::directory::nearest_existing(path))
        .map_or(root, std::path::Path::to_path_buf)
}

/// One step up the drive list, wrapping to the bottom at the top.
const fn drive_index_up(index: usize, count: usize) -> usize {
    if index > 0 {
        index - 1
    } else if count > 0 {
        count - 1
    } else {
        0
    }
}

/// One step down the drive list, wrapping to the top past the end.
const fn drive_index_down(index: usize, count: usize) -> usize {
    if index + 1 < count {
        index + 1
    } else {
        0
    }
}

/// Point `side`'s panel at `path` and reload its listing. Returns the panel's
/// new path on success, or the io error text when the drive cannot be read;
/// the wording is left to the caller, since the drive menu says "Changed to"
/// while quick-select by letter says "Changed to drive".
///
/// Both drive-menu paths (Enter on a row, the row's own hotkey) come
/// through here, and it moves the panel with
/// [`crate::models::Panel::navigate_to`] — which drops the marks that named
/// entries of the directory being left and, on an unreadable drive, rolls the
/// panel back instead of parking it on a path it cannot list.
fn open_drive(app: &mut App, side: PanelSide, path: PathBuf) -> Result<PathBuf, String> {
    // A local drive means local files: a sessions list or a host drawn over
    // the panel goes first, a remote panel's connection with it — so even a
    // drive that cannot be read leaves the panel showing its files.
    crate::ops::sessions::back_to_files_on(app, side);
    let panel = match side {
        PanelSide::Left => &mut app.left_panel,
        PanelSide::Right => &mut app.right_panel,
    };
    panel.navigate_to(path).map_err(|err| err.to_string())?;
    Ok(panel.path.clone())
}

#[cfg(test)]
mod tests;
