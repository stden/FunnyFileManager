//! Purpose: Persist discovered network mounts in the user's private ffm.yml and connect configured drives.
use crate::command::NetDrivesCommand;
use crate::models::{App, MaintenanceCommand, Overlay, Viewer};
use crate::status::Status;
use std::io;
use std::path::{Path, PathBuf};

pub mod connect;
mod discover;
mod storage;
pub use storage::save_to;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_connect;

const NO_CONFIG_DIR: &str = "No configuration directory: set HOME or APPDATA to use ffm.yml";

pub fn start() {
    start_with(
        |run| {
            std::thread::Builder::new()
                .name("ffm-network-drives".to_owned())
                .spawn(run)
                .map(|_| ())
        },
        refresh,
    );
}

fn start_with(
    spawn: impl FnOnce(Box<dyn FnOnce() + Send>) -> io::Result<()>,
    run_refresh: fn() -> io::Result<()>,
) {
    let result = spawn(Box::new(move || {
        if let Err(error) = run_refresh() {
            crate::diagnostics::log_error(&format!("network drives: {error}"));
        }
    }));
    if let Err(error) = result {
        crate::diagnostics::log_error(&format!("network drive worker: {error}"));
    }
}

fn refresh() -> io::Result<()> {
    refresh_at(crate::sessions::SessionStore::path())
}

fn refresh_at(path: Option<PathBuf>) -> io::Result<()> {
    let Some(path) = path else {
        return Ok(());
    };
    save_to(&path, &discover::read()?)
}

fn config_path() -> io::Result<PathBuf> {
    config_path_from(crate::sessions::SessionStore::path())
}

pub fn config_path_from(path: Option<PathBuf>) -> io::Result<PathBuf> {
    path.ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, NO_CONFIG_DIR))
}

/// Queue a background job that mounts every configured drive in `ffm.yml`.
pub fn queue_connect_all(app: &mut App) -> io::Result<Status> {
    let path = config_path()?;
    queue_connect_all_from(app, &path)
}

pub fn queue_connect_all_from(app: &mut App, path: &Path) -> io::Result<Status> {
    let drives = storage::load_from(path)?;
    if drives.is_empty() {
        return Ok(Status::CommandOutput(format!(
            "No network drives configured in {}",
            path.display()
        )));
    }
    let count = drives.len();
    app.maintenance_request = Some(MaintenanceCommand::ConnectNetworkDrives(drives));
    Ok(Status::CommandOutput(format!(
        "Connecting {count} network drive(s)..."
    )))
}

/// Queue one-press connection of all configured network drives and update the status bar.
pub fn connect_all(app: &mut App) -> io::Result<()> {
    let status = queue_connect_all(app)?;
    app.set_status(status);
    Ok(())
}

/// Open `ffm.yml` in the built-in editor, creating a starter template if absent.
pub fn edit_config(app: &mut App) -> io::Result<()> {
    let path = config_path()?;
    edit_config_at(app, &path)
}

pub fn edit_config_at(app: &mut App, path: &Path) -> io::Result<()> {
    storage::ensure_template(path)?;
    let bytes = crate::fileload::read_small_file(path, "ffm.yml")?.into_bytes();
    let editor = crate::editor::editor_for(app, path.to_path_buf(), &bytes)?;
    app.set_overlay(Overlay::Editor(editor));
    app.set_status(Status::Editing(path.display().to_string()));
    Ok(())
}

/// Show the configured `network_drives:` entries from `ffm.yml` in a read-only viewer.
pub fn list(app: &mut App) -> io::Result<()> {
    let path = config_path()?;
    list_from(app, &path)
}

pub fn list_from(app: &mut App, path: &Path) -> io::Result<()> {
    let drives = storage::load_from(path)?;
    if drives.is_empty() {
        app.set_status(Status::CommandOutput(format!(
            "No network drives configured in {}",
            path.display()
        )));
        return Ok(());
    }
    let lines: Vec<String> = drives
        .iter()
        .map(|d| {
            format!(
                "{}  {}  ({})",
                d.path.as_str(),
                d.remote.as_str(),
                d.protocol.label()
            )
        })
        .collect();
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "Network drives (ffm.yml)",
        lines,
    )));
    Ok(())
}

/// Execute `:netdrives ...`.
pub fn execute(app: &mut App, command: NetDrivesCommand) -> io::Result<()> {
    match command {
        NetDrivesCommand::Connect => connect_all(app),
        NetDrivesCommand::Edit => edit_config(app),
        NetDrivesCommand::List => list(app),
    }
}
