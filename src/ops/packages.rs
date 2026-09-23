use crate::{
    models::{App, ChoiceTarget, MenuChoice, MenuDialog, Overlay},
    packages::UpdatePlan,
    status::Status,
};
use ratatui::{backend::Backend, Terminal};
use std::{
    io::{self, Write},
    process::Command,
};

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
    run_with(terminal, app, &mut crate::terminal::CrosstermOps, execute)
}

fn execute(plan: &UpdatePlan) -> io::Result<String> {
    plan.execute_with(|path, args| {
        command(plan, path, args)
            .status()
            .map(|status| status.code().unwrap_or(-1))
    })
}

fn run_with<B: Backend + Write, O: crate::terminal::TerminalOps<B>>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    ops: &mut O,
    run: impl FnOnce(&UpdatePlan) -> io::Result<String>,
) -> io::Result<()> {
    let Some(plan) = app.package_update.take() else {
        return Ok(());
    };
    let suspension = crate::terminal::TerminalSuspension::suspend(terminal, ops)?;
    let result = run(&plan);
    let restored = suspension.resume();
    app.set_status(match result {
        Ok(message) => Status::CommandOutput(message),
        Err(error) => Status::Error(error.to_string()),
    });
    restored
}

#[cfg(test)]
mod tests;

#[cfg(unix)]
fn command(plan: &UpdatePlan, path: &std::path::Path, args: &[&str]) -> Command {
    // SAFETY: geteuid has no preconditions and does not mutate process state.
    let elevate = plan.manager == crate::packages::Manager::Apt && unsafe { libc::geteuid() } != 0;
    let mut command = if elevate {
        let mut command = Command::new("sudo");
        command.arg("--").arg(path);
        command
    } else {
        Command::new(path)
    };
    command.args(args);
    command
}

/// The elevation wrapper, kept as a real `.ps1` so it can be read, formatted
/// and syntax-checked like any other PowerShell.
#[cfg(windows)]
const ELEVATED_UPGRADE_SCRIPT: &str = include_str!("packages/elevated_upgrade.ps1");

#[cfg(windows)]
fn command(_: &UpdatePlan, path: &std::path::Path, _args: &[&str]) -> Command {
    // Fixed arguments; quote the discovered executable as a PowerShell literal.
    let path = path.to_string_lossy().replace('\'', "''");
    let script = ELEVATED_UPGRADE_SCRIPT.replace("{path}", &path);
    let mut command = Command::new("powershell.exe");
    command.args(["-NoProfile", "-Command", &script]);
    command
}
