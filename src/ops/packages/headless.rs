//! Purpose: Package updates in a private PTY, never a UAC or console window.
#![allow(
    clippy::unnecessary_wraps,
    reason = "headless package update module — return type kept for consistency with surrounding code"
)]

use crate::{
    models::App,
    packages::{StepExit, StepOutcome, UpdatePlan},
    status::Status,
    terminal::headless,
};
use std::{io, path::Path, process::Command};

pub(super) fn run(app: &mut App) -> io::Result<()> {
    let Some(plan) = app.package_update.take() else {
        return Ok(());
    };
    let cwd = app.get_active_panel_ref().path.clone();
    let result = plan.execute_with(|path, args| {
        app.console.record_command(&cwd, &plan.command_line(args));
        let outcome = headless::run(command(&plan, path, args).current_dir(&cwd))?;
        let output = String::from_utf8_lossy(&outcome.output);
        app.console.record_output(&output);
        Ok(StepOutcome {
            exit: StepExit::Code(i32::from_ne_bytes(outcome.code.to_ne_bytes())),
            reason: crate::console::first_drawable(output.lines().rev()),
        })
    });
    let status = match result {
        _ if headless::cancelled() == Some(true) => Status::PackageUpdateCancelled,
        Ok(message) => Status::CommandOutput(message),
        Err(error) => Status::Error(error.to_string()),
    };
    app.record_run_outcome(status);
    Ok(())
}

#[cfg(unix)]
fn command(plan: &UpdatePlan, path: &Path, args: &[&str]) -> Command {
    super::command(plan, path, args)
}

#[cfg(windows)]
fn command(_: &UpdatePlan, path: &Path, args: &[&str]) -> Command {
    // Automation uses its own account's privileges; it must not open a UAC UI.
    let mut command = Command::new(path);
    command.args(args);
    command
}
