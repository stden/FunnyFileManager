use crate::{
    models::{App, ChoiceTarget, MenuChoice, MenuDialog, Overlay},
    packages::{StepOutcome, UpdatePlan},
    status::Status,
};
use ratatui::{backend::Backend, Terminal};
use std::{
    io::{self, Write},
    process::{Command, ExitStatus, Stdio},
    time::Instant,
};

use super::shell::{collect_until, drain_echoing, DRAIN_GRACE};

mod headless;

pub fn open(app: &mut App) {
    open_with(app, crate::packages::detect());
}

fn open_with(app: &mut App, plan: Option<UpdatePlan>) {
    let Some(plan) = plan else {
        app.set_status(Status::Error(
            "No supported package manager found in PATH (choco, apt-get, brew)".to_owned(),
        ));
        return;
    };
    let rows = vec![
        MenuChoice {
            label: "Cancel".to_owned(),
            target: ChoiceTarget::Close,
        },
        MenuChoice {
            label: format!("Run: {}", plan.description()),
            target: ChoiceTarget::UpdatePackages(plan),
        },
    ];
    app.set_overlay(Overlay::MenuDialog(Box::new(MenuDialog::Choices {
        title: "Update all installed packages".to_owned(),
        rows,
        selected: 0,
    })));
}

pub(crate) fn run_pending<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> io::Result<()> {
    if app.live.terminal_mode == crate::models::TerminalMode::Headless {
        return headless::run(app);
    }
    run_with(terminal, app, &mut crate::terminal::CrosstermOps, execute)
}

/// Run every step, echoing each into the Ctrl+O transcript with what it wrote
/// to stderr, as a `:` command would be.
fn execute(plan: &UpdatePlan, app: &mut App) -> io::Result<String> {
    let cwd = app.get_active_panel_ref().path.clone();
    plan.execute_with(|path, args| {
        app.console.record_command(&cwd, &plan.command_line(args));
        let (status, stderr) = run_step(command(plan, path, args))?;
        app.console.record_error(&stderr);
        Ok(StepOutcome {
            exit: status.into(),
            reason: crate::console::first_drawable(stderr.lines().rev()),
        })
    })
}

/// Run one step on the terminal, keeping a copy of what it writes to stderr.
///
/// stdin and stdout stay the terminal's, so sudo can ask for the password and
/// the manager can draw its progress. stderr is shown as it arrives and also
/// kept: sudo's reason for refusing, or apt-get's `E:` line, is printed there,
/// on a screen the panels cover again the moment the step ends.
fn run_step(mut command: Command) -> io::Result<(ExitStatus, String)> {
    let program = command.get_program().to_string_lossy().into_owned();
    let mut child = command.stderr(Stdio::piped()).spawn().map_err(|error| {
        io::Error::new(error.kind(), format!("cannot start {program}: {error}"))
    })?;
    let stderr = drain_echoing(child.stderr.take(), io::stderr());
    let status = child.wait()?;
    // Bounded: a daemon a maintainer script started may hold the pipe open.
    let kept = collect_until(&stderr, Instant::now() + DRAIN_GRACE);
    Ok((status, String::from_utf8_lossy(&kept).into_owned()))
}

fn run_with<B: Backend + Write, O: crate::terminal::TerminalOps<B>>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    ops: &mut O,
    run: impl FnOnce(&UpdatePlan, &mut App) -> io::Result<String>,
) -> io::Result<()> {
    let Some(plan) = app.package_update.take() else {
        return Ok(());
    };
    let suspension = crate::terminal::TerminalSuspension::suspend(terminal, ops)?;
    let result = run(&plan, app);
    let interrupted = suspension.interrupted();
    let restored = suspension.resume();
    let status = outcome_status(result, interrupted);
    app.record_run_outcome(status);
    restored
}

fn outcome_status(result: io::Result<String>, interrupted: bool) -> Status {
    match result {
        // Whatever sudo or apt-get then exited with, the user stopped it.
        _ if interrupted => Status::PackageUpdateCancelled,
        Ok(message) => Status::CommandOutput(message),
        Err(error) => Status::Error(error.to_string()),
    }
}

#[cfg(test)]
mod tests;

#[cfg(all(test, unix))]
mod tests_interrupt;

#[cfg(all(test, unix))]
mod tests_sudo;

#[cfg(unix)]
fn command(plan: &UpdatePlan, path: &std::path::Path, args: &[&str]) -> Command {
    let mut command = match plan.elevation {
        crate::packages::Elevation::Sudo => {
            let mut command = Command::new("sudo");
            command.arg("--").arg(path);
            command
        }
        crate::packages::Elevation::CurrentUser => Command::new(path),
    };
    command.args(args);
    command
}

/// Keep the native UAC adapter as an embedded Python source file.
#[cfg(any(windows, test))]
const ELEVATED_UPGRADE_SCRIPT: &str = include_str!("packages/elevated_upgrade.py");

#[cfg(any(windows, test))]
fn elevated_command(path: &std::path::Path, args: &[&str]) -> Command {
    let mut command = Command::new("python.exe");
    command.args(["-I", "-X", "utf8", "-c", ELEVATED_UPGRADE_SCRIPT]);
    command.arg(path).args(args);
    command
}

#[cfg(windows)]
fn command(plan: &UpdatePlan, path: &std::path::Path, args: &[&str]) -> Command {
    if plan.elevation == crate::packages::Elevation::CurrentUser {
        let mut command = Command::new(path);
        command.args(args);
        return command;
    }
    elevated_command(path, args)
}

#[cfg(test)]
mod tests_steps;
