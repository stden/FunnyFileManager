//! Purpose: Run a selected program inside the automation session's private PTY.

use crate::{models::App, status::Status, terminal::headless};
use std::io;

pub(super) fn launch(app: &mut App) -> io::Result<()> {
    let Some(path) = app.get_active_panel().selected_path() else {
        return Ok(());
    };
    let path = std::path::absolute(path)?;
    let directory = std::path::absolute(&app.get_active_panel_ref().path)?;
    app.console
        .record_command(&directory, &path.display().to_string());
    let result = headless::run(super::program_command(&path).current_dir(&directory));
    let status = match result {
        Ok(outcome) => {
            app.console
                .record_output(&String::from_utf8_lossy(&outcome.output));
            Status::CommandOutput(format!(
                "{} exited with code {}",
                path.display(),
                outcome.code
            ))
        }
        Err(error) => Status::CannotRun {
            program: path,
            error: error.to_string(),
        },
    };
    app.record_run_outcome(status);
    crate::utils::refresh_panels(app)
}

#[cfg(all(test, unix))]
mod tests;
