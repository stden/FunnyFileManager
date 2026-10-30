//! Launch panel programs with the host terminal available for interactive input.

use crate::{
    models::{App, ProgramPause},
    status::Status,
    terminal::{CrosstermOps, TerminalOps, TerminalSuspension},
};
use ratatui::{backend::Backend, Terminal};
use std::{
    io::{self, Write},
    path::Path,
    process::{Command, ExitStatus},
};

pub fn launch<B: Backend + Write>(terminal: &mut Terminal<B>, app: &mut App) -> io::Result<()> {
    launch_with(terminal, app, &mut CrosstermOps, |path, directory| {
        program_command(path).current_dir(directory).status()
    })
}

fn launch_with<B, O, F>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    terminal_ops: &mut O,
    run: F,
) -> io::Result<()>
where
    B: Backend + Write,
    O: TerminalOps<B>,
    F: FnOnce(&Path, &Path) -> io::Result<ExitStatus>,
{
    let Some(path) = app.get_active_panel().selected_path() else {
        return Ok(());
    };
    let path = std::path::absolute(path)?;
    let directory = std::path::absolute(&app.get_active_panel_ref().path)?;
    app.console
        .record_command(&directory, &path.display().to_string());
    let suspension = TerminalSuspension::suspend(terminal, terminal_ops)?;
    let result = run(&path, &directory);
    let restored = match (&result, app.live.program_pause) {
        // What the program printed is on the terminal's own screen, which
        // the panels are about to cover; it stays readable until a key.
        (Ok(status), ProgramPause::UntilKey) => suspension.resume_after_key(&pause_prompt(*status)),
        _ => suspension.resume(),
    };
    app.record_run_outcome(match result {
        Ok(status) => Status::ProgramExited {
            program: path,
            status,
        },
        Err(error) => Status::CannotRun {
            program: path,
            error: error.to_string(),
        },
    });
    let refreshed = crate::utils::refresh_panels(app);
    crate::terminal::merge_run_and_cleanup(refreshed, restored)
}

/// The line under a finished program's output: how it ended, and the way back.
fn pause_prompt(status: ExitStatus) -> String {
    format!("[{status}] Press any key to return to FFM")
}

fn runner_command(program: &str, flags: &[&str], path: &Path) -> Command {
    let mut command = Command::new(program);
    command.args(flags).arg(path);
    command
}

#[cfg(windows)]
fn program_command(path: &Path) -> Command {
    let ext = path.extension().and_then(|v| v.to_str()).unwrap_or("");
    if ext.eq_ignore_ascii_case("ps1") {
        runner_command("powershell.exe", &["-NoProfile", "-File"], path)
    } else if ext.eq_ignore_ascii_case("msi") {
        runner_command("msiexec.exe", &["/i"], path)
    } else if ext.eq_ignore_ascii_case("py") {
        runner_command("python", &[], path)
    } else if ext.eq_ignore_ascii_case("sh") {
        runner_command("sh", &[], path)
    } else {
        Command::new(path)
    }
}

#[cfg(not(windows))]
fn program_command(path: &Path) -> Command {
    use std::os::unix::fs::PermissionsExt;

    if std::fs::metadata(path).is_ok_and(|meta| meta.permissions().mode() & 0o111 != 0) {
        return Command::new(path);
    }
    let ext = path.extension().and_then(|v| v.to_str()).unwrap_or("");
    if ext.eq_ignore_ascii_case("py") {
        runner_command("python3", &[], path)
    } else if ext.eq_ignore_ascii_case("sh") {
        runner_command("sh", &[], path)
    } else if ext.eq_ignore_ascii_case("cmd") {
        runner_command("cmd.exe", &["/c"], path)
    } else {
        Command::new(path)
    }
}

#[cfg(test)]
mod tests;
