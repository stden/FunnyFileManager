//! Launch panel programs with the host terminal available for interactive input.

use crate::{
    models::App,
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
    let suspension = TerminalSuspension::suspend(terminal, terminal_ops)?;
    let result = run(&path, &directory);
    let restored = suspension.resume();
    app.set_status(match result {
        Ok(status) => Status::CommandOutput(format!("{}: {status}", path.display())),
        Err(error) => Status::Error(format!("Cannot run {}: {error}", path.display())),
    });
    let refreshed = crate::utils::refresh_panels(app);
    crate::terminal::merge_run_and_cleanup(refreshed, restored)
}

#[cfg(windows)]
fn program_command(path: &Path) -> Command {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    if extension.eq_ignore_ascii_case("ps1") {
        let mut command = Command::new("powershell.exe");
        command.args(["-NoProfile", "-File"]).arg(path);
        command
    } else if extension.eq_ignore_ascii_case("msi") {
        let mut command = Command::new("msiexec.exe");
        command.arg("/i").arg(path);
        command
    } else {
        Command::new(path)
    }
}

#[cfg(not(windows))]
fn program_command(path: &Path) -> Command {
    Command::new(path)
}

#[cfg(test)]
mod tests;
