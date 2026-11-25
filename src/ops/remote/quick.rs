//! Purpose: Quick-connect to any FTP/FTPS server and persist its profile to ffm.yml.

use crate::models::remote_dialog::{RemoteDialog, RemoteLogin};
use crate::models::{App, InputPrompt, InputPurpose, Overlay, PanelSide, RemoteOverlay};
use crate::status::Status;
use std::io;

/// Open the one-line FTP/FTPS address prompt targeting `side`'s panel.
pub fn prompt_ftp(app: &mut App, side: PanelSide) {
    app.set_overlay(Overlay::InputPrompt(Box::new(InputPrompt::new(
        InputPurpose::FtpConnect(side),
        "",
    ))));
}

/// Dispatch `:ftp [target]` from the command line.
pub fn command(app: &mut App, target: Option<&str>) -> io::Result<()> {
    match target {
        None => {
            prompt_ftp(app, app.active_panel);
            Ok(())
        }
        Some(raw) => connect_ftp(app, app.active_panel, raw).map_err(io::Error::other),
    }
}

/// Handle confirmation of the [`InputPurpose::FtpConnect`] dialog.
pub fn submit(app: &mut App, side: PanelSide, typed: &str) {
    if let Err(message) = connect_ftp(app, side, typed) {
        app.set_status(Status::Error(message));
    }
}

fn connect_ftp(app: &mut App, side: PanelSide, raw: &str) -> Result<(), String> {
    let session =
        crate::values::session::parse_ftp_target(raw).map_err(|error| format!("FTP: {error}"))?;
    let name = session.name.clone();
    crate::ops::sessions::remember(app, session.clone())
        .map_err(|error| format!("FTP: {error}"))?;
    let login = RemoteLogin::new_on(session, side);
    app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(
        RemoteDialog::Login(login),
    ))));
    app.set_status(Status::CommandOutput(format!(
        "Saved FTP profile {name} to ffm.yml"
    )));
    Ok(())
}

#[cfg(test)]
mod tests;
