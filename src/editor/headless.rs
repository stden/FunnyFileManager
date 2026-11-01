//! Purpose: External editing through private terminal I/O without GUI probing.

use crate::{models::App, status::Status, terminal::headless};
use std::{io, process::Command};

pub(super) fn edit(app: &mut App) -> io::Result<()> {
    let Some(entry) = app.get_active_panel_ref().get_selected_entry() else {
        return Ok(());
    };
    if entry.kind.is_dir() {
        app.set_status(Status::EditDirUnsupported);
        return Ok(());
    }
    let Some(path) = app.get_active_panel_ref().selected_path() else {
        return Ok(());
    };
    let path = std::path::absolute(path)?;
    app.view_history.touch(&path);
    let command = std::env::var("EDITOR").unwrap_or_else(|_| super::DEFAULT_EDITOR.to_owned());
    let (editor, args) = super::split_editor_command(&command);
    let result = headless::run(Command::new(editor).args(args).arg(&path));
    let status = outcome(app, result, &path);
    app.set_status(status);
    crate::utils::refresh_panels(app)
}

fn outcome(
    app: &mut App,
    result: io::Result<headless::ExternalOutcome>,
    path: &std::path::Path,
) -> Status {
    match result {
        Ok(outcome) => {
            app.console
                .record_output(&String::from_utf8_lossy(&outcome.output));
            if outcome.code == 0 {
                Status::Edited(path.display().to_string())
            } else {
                Status::EditorExited {
                    status: format!("exit code {}", outcome.code),
                }
            }
        }
        Err(error) => Status::LaunchEditorFailed {
            error: error.to_string(),
        },
    }
}

#[cfg(test)]
mod tests;
