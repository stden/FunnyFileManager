//! Purpose: Queue bulk Git pulls from the active folder and show their console report.

use crate::{models::App, status::Status};
use std::{io, path::Path};

mod discover;
pub(in crate::ops) mod repositories;
mod run;

pub(super) const LABEL: &str = "Git pull for all repositories";

pub(super) fn queue(app: &mut App) -> io::Result<()> {
    let panel = app.get_active_panel_ref();
    if panel.kind.is_netbox() || !panel.path.is_dir() {
        return Err(io::Error::other("Git pull requires a local directory"));
    }
    app.git_pull_root = Some(panel.path.clone());
    app.set_status(Status::CommandOutput("Git pull queued".to_owned()));
    Ok(())
}

pub fn run_pending(app: &mut App) {
    run_with(app, execute);
}

fn execute(root: &Path, app: &mut App, cancelled: &dyn Fn() -> bool) -> io::Result<run::Summary> {
    run::execute(
        root,
        |line| {
            app.console.record_output(line);
            Ok(())
        },
        cancelled,
    )
}

fn run_with(
    app: &mut App,
    run: impl FnOnce(&Path, &mut App, &dyn Fn() -> bool) -> io::Result<run::Summary>,
) {
    let Some(root) = app.git_pull_root.take() else {
        return;
    };
    let result = run(&root, app, &|| {
        crate::progress::tick();
        crate::terminal::cancel_requested()
    });
    record_outcome(app, result);
    drop(crate::utils::refresh_panels(app));
}

fn record_outcome(app: &mut App, result: io::Result<run::Summary>) {
    let status = match result {
        Err(error) if error.kind() == io::ErrorKind::Interrupted => {
            Status::CommandOutput("Git pull cancelled".to_owned())
        }
        Ok(summary) if summary.failed > 0 || summary.scan_errors > 0 => {
            Status::Error(summary.to_string())
        }
        Ok(summary) => Status::CommandOutput(summary.to_string()),
        Err(error) => Status::Error(format!("Git pull failed: {error}")),
    };
    app.record_run_outcome(status);
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod fixtures;
