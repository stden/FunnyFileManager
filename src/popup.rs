//! Purpose: Handles modal overlay keyboard input and popup state transitions.
//!
use crate::models::EditState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::PathBuf;

use crate::models::{App, DriveEntry, Overlay, PanelSide};
use crate::ops::{handle_create_file, handle_delete};
use crate::status::Status;
use crate::utils::refresh_panels;

/// Handle keyboard input for popup overlays (help, viewer, editor, dialogs).
///
/// Overlays are modal windows that appear on top of the panels.
/// While an overlay is open, keys go here instead of the main handler.
/// Returns true if the key was consumed by an overlay.
pub fn process_popup_input(app: &mut App, key: KeyEvent) -> bool {
    let code = key.code;
    let modifiers = key.modifiers;

    match &mut app.overlay {
        Overlay::None => false,
        Overlay::Viewer(viewer) => {
            match code {
                KeyCode::Esc | KeyCode::F(3) | KeyCode::Enter => {
                    app.close_overlay();
                }
                KeyCode::Down => {
                    // The clamp lives on `ScrollTop`, so the guard the `if`
                    // used to carry is gone.
                    viewer.scroll = viewer.scroll.down(1, viewer.lines.len().saturating_sub(1));
                }
                KeyCode::Up => viewer.scroll = viewer.scroll.up(1),
                _ => {}
            }
            true
        }
        Overlay::Editor(_) => process_editor_input(app, code, modifiers),
        Overlay::Help => {
            if matches!(code, KeyCode::Esc | KeyCode::F(1)) {
                app.close_overlay();
            }
            true
        }
        Overlay::CommandLine(_) => command_line::process(app, key),
        Overlay::DriveSelection {
            side: target_panel,
            selected_index,
        } => {
            let drives = crate::models::App::get_all_drives();
            let drive_count = drives.len();
            let target_panel = *target_panel; // Copy it out

            match code {
                KeyCode::Esc => {
                    app.close_overlay();
                }
                KeyCode::Up => *selected_index = drive_index_up(*selected_index, drive_count),
                KeyCode::Down => *selected_index = drive_index_down(*selected_index, drive_count),
                KeyCode::Home => {
                    *selected_index = 0;
                }
                KeyCode::End if drive_count > 0 => {
                    *selected_index = drive_count - 1;
                }
                KeyCode::Enter if *selected_index < drive_count => {
                    // Cloning the entry ends the `&mut app.overlay` borrow the
                    // match bindings hold, so the helper can take the whole App.
                    let selected = drives[*selected_index].clone();
                    let status = select_drive(app, target_panel, selected);
                    app.set_status(status);
                    app.close_overlay();
                }
                KeyCode::Char(ch) if ch.is_ascii_alphabetic() => {
                    // Quick select by drive letter (Windows). On Unix `C:\` and
                    // friends never exist, so the key is simply swallowed.
                    let drive_letter = ch.to_ascii_uppercase();
                    open_drive_at_path(
                        app,
                        target_panel,
                        PathBuf::from(format!("{drive_letter}:\\")),
                    );
                }
                _ => {}
            }
            true
        }
        Overlay::DeleteConfirm(_) => {
            match code {
                KeyCode::Char('y' | 'Y') | KeyCode::Enter => {
                    // Confirmed. `handle_delete` never fails as a whole: it
                    // folds per-file errors into its own status summary, so
                    // there is nothing left to report here.
                    let _ = handle_delete(app);
                }
                KeyCode::Char('n' | 'N') | KeyCode::Esc => {
                    // Cancelled
                    app.close_overlay();
                    app.set_status(crate::status::Status::DeleteCancelled);
                }
                _ => {}
            }
            true
        }
        Overlay::NewFileInput(input) => {
            match code {
                KeyCode::Esc => {
                    app.close_overlay();
                }
                KeyCode::Enter => {
                    let filename = input.trim().to_string();
                    app.close_overlay();
                    if let Err(err) = handle_create_file(app, &filename) {
                        app.set_status(crate::status::Status::CreateFileFailed {
                            error: err.to_string(),
                        });
                    }
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(ch)
                    if !modifiers.contains(KeyModifiers::CONTROL)
                        && !modifiers.contains(KeyModifiers::ALT) =>
                {
                    input.push(ch);
                }
                _ => {}
            }
            true
        }
    }
}

/// Apply the drive picked in the Alt+F1 / Alt+F2 menu to `side`'s panel and
/// describe the outcome. An S3 profile is only acknowledged — the S3 browser is
/// not implemented yet, so the panel keeps showing its local directory.
fn select_drive(app: &mut App, side: PanelSide, drive: DriveEntry) -> Status {
    match drive {
        DriveEntry::Local(path) => match open_drive(app, side, path) {
            Ok(path) => Status::ChangedTo(path),
            Err(error) => Status::OpenDriveFailed { error },
        },
        DriveEntry::S3Profile(profile) => Status::S3ProfileSelected(profile),
    }
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

/// Open `drive_path` in `target_panel` when it exists; a path that does not
/// exist is silently ignored, which is what makes unknown drive letters a
/// harmless keypress instead of an error.
fn open_drive_at_path(app: &mut App, target_panel: PanelSide, drive_path: PathBuf) {
    if !drive_path.exists() {
        return;
    }
    let status = match open_drive(app, target_panel, drive_path) {
        Ok(path) => Status::ChangedToDrive(path),
        Err(error) => Status::OpenDriveFailed { error },
    };
    app.set_status(status);
    app.close_overlay();
}

/// Point `side`'s panel at `path` and reload its listing. Returns the panel's
/// new path on success, or the io error text when the drive cannot be read;
/// the wording is left to the caller, since the drive menu says "Changed to"
/// while quick-select by letter says "Changed to drive".
///
/// Both drive-menu paths (Enter on a row, quick-select by drive letter) come
/// through here, and it moves the panel with
/// [`crate::models::Panel::navigate_to`] — which drops the marks that named
/// entries of the directory being left and, on an unreadable drive, rolls the
/// panel back instead of parking it on a path it cannot list.
fn open_drive(app: &mut App, side: PanelSide, path: PathBuf) -> Result<PathBuf, String> {
    let panel = match side {
        PanelSide::Left => &mut app.left_panel,
        PanelSide::Right => &mut app.right_panel,
    };
    panel.navigate_to(path).map_err(|err| err.to_string())?;
    Ok(panel.path.clone())
}

/// Lines jumped by PgUp/PgDn in the built-in editor.
const EDITOR_PAGE: usize = 20;

/// Keyboard handling for the built-in editor overlay (F4).
///
/// Far Manager keys: F2 saves, Esc/F10 close; closing with unsaved changes
/// asks for confirmation (Y save, N discard, Esc continue editing).
fn process_editor_input(app: &mut App, code: KeyCode, modifiers: KeyModifiers) -> bool {
    let Some(editor) = app.get_editor_mut() else {
        return false;
    };

    if editor.state.is_confirming_exit() {
        match code {
            KeyCode::Char('y' | 'Y') | KeyCode::Enter => {
                let result = editor.save();
                let path = editor.path.clone();
                match result {
                    Ok(()) => {
                        app.close_overlay();
                        app.set_status(crate::status::Status::Saved(path));
                        let _ = refresh_panels(app);
                    }
                    Err(err) => {
                        if let Some(editor) = app.get_editor_mut() {
                            editor.state = EditState::Modified;
                        }
                        app.set_status(crate::status::Status::SaveFailed {
                            error: err.to_string(),
                        });
                    }
                }
            }
            KeyCode::Char('n' | 'N') => {
                app.close_overlay();
                app.set_status(crate::status::Status::ChangesDiscarded);
            }
            KeyCode::Esc => editor.state = EditState::Modified,
            _ => {}
        }
        return true;
    }

    match code {
        KeyCode::Esc | KeyCode::F(10) => {
            if editor.state.is_modified() {
                editor.state = EditState::ConfirmingExit;
            } else {
                app.close_overlay();
            }
        }
        KeyCode::F(2) => {
            let result = editor.save();
            let path = editor.path.clone();
            app.set_status(match result {
                Ok(()) => crate::status::Status::Saved(path),
                Err(err) => crate::status::Status::SaveFailed {
                    error: err.to_string(),
                },
            });
            let _ = refresh_panels(app);
        }
        KeyCode::Enter => editor.insert_newline(),
        KeyCode::Backspace => editor.backspace(),
        KeyCode::Delete => editor.delete_forward(),
        KeyCode::Left => editor.move_left(),
        KeyCode::Right => editor.move_right(),
        KeyCode::Up => editor.move_up(1),
        KeyCode::Down => editor.move_down(1),
        KeyCode::PageUp => editor.move_up(EDITOR_PAGE),
        KeyCode::PageDown => editor.move_down(EDITOR_PAGE),
        KeyCode::Home => editor.move_home(),
        KeyCode::End => editor.move_end(),
        KeyCode::Tab => {
            for _ in 0..4 {
                editor.insert_char(' ');
            }
        }
        KeyCode::Char(ch)
            if !modifiers.contains(KeyModifiers::CONTROL)
                && !modifiers.contains(KeyModifiers::ALT) =>
        {
            editor.insert_char(ch);
        }
        _ => {}
    }
    true
}

mod command_line;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_command_line_completion;
#[cfg(test)]
mod tests_command_line_editing;
#[cfg(test)]
mod tests_console_and_editor_modes;
#[cfg(test)]
mod tests_drive_selection;
