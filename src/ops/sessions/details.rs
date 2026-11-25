//! Purpose: F3 on a stored session: its settings in the viewer, with no
//! password to show.

use super::{find, SessionError};
use crate::models::{App, Auth, Origin, Overlay, Session, SessionName, Viewer};

/// F3: the session's settings in the viewer. There is no password to show.
pub fn view(app: &mut App, name: &SessionName) -> Result<(), SessionError> {
    let lines = details(find(app, name)?);
    app.set_overlay(Overlay::Viewer(Viewer::report(
        format!("NetBox: {name}"),
        lines,
    )));
    Ok(())
}

/// The lines F3 shows.
fn details(session: &Session) -> Vec<String> {
    let user = session
        .user
        .as_ref()
        .map_or_else(|| "(none)".to_owned(), ToString::to_string);
    let login = match &session.auth {
        Auth::Password => "password, asked for when connecting".to_owned(),
        Auth::KeyFile(file) => format!("key file {file}"),
        Auth::Agent => "SSH agent".to_owned(),
    };
    let directory = session
        .remote_dir
        .as_ref()
        .map_or_else(|| "(login directory)".to_owned(), ToString::to_string);
    let origin = match &session.origin {
        Origin::Manual => "made or changed here".to_owned(),
        Origin::NetBoxFile { path } => format!("imported from {}", path.display()),
        Origin::Things { .. } => "imported from the Things catalog".to_owned(),
    };
    vec![
        format!("Name:      {}", session.name),
        format!("Protocol:  {}", session.protocol.label()),
        format!("Host:      {}", session.host),
        format!("Port:      {}", session.port),
        format!("User:      {user}"),
        format!("Login:     {login}"),
        format!("Directory: {directory}"),
        format!("Origin:    {origin}"),
        format!("Address:   {}", session.url()),
    ]
}
