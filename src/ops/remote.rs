//! Purpose: Browsing a remote host through a stored session — connecting,
//! listing a directory into the panel, entering one, and leaving.
//!
//! Everything goes through the [`RemoteFs`] the panel carries, so the same
//! code runs over the fake in tests and over a transport once this build has
//! one. Every failure reaches the status bar; none reaches the event loop.

use crate::models::{
    remote_panel::Shown, App, PanelKind, RemotePanel, RemotePath, Session, SessionName,
};
use crate::remote::{RemoteError, RemoteFs, RemoteLink};
use crate::status::Status;

/// Enter on a session: open its host in the active panel.
pub fn connect(app: &mut App, name: &SessionName) {
    let Some(session) = app.sessions.find(name).cloned() else {
        fail(app, &format!("no session named {name}"));
        return;
    };
    let opened = crate::remote::connect(&session).and_then(|fs| open_with(app, &session, fs));
    if let Err(error) = opened {
        fail(app, &format!("{name}: {error}"));
    }
}

/// Browse `session` through an already open `fs`: the panel shows the
/// session's start directory, or the root.
pub fn open_with(
    app: &mut App,
    session: &Session,
    fs: Box<dyn RemoteFs>,
) -> Result<(), RemoteError> {
    let link = RemoteLink::new(fs);
    let start = session.remote_dir.clone().unwrap_or_else(RemotePath::root);
    let listing = link.lock().list(&start)?;
    let mut panel = RemotePanel::new(session.name.clone(), link);
    let shown = panel.show(start, listing);
    app.get_active_panel().kind = PanelKind::Remote(Box::new(panel));
    announce(app, shown, format!("NetBox: {}", session.url()));
    Ok(())
}

/// What Enter does on the remote panel's cursor.
enum Target {
    /// `..` at the root: out of the host.
    Leave,
    Directory(RemotePath),
    File(String),
}

/// Enter: a directory is listed, `..` climbs or leaves, a file is refused.
pub fn enter(app: &mut App) {
    match target(app) {
        None => {}
        Some(Target::Leave) => leave(app),
        Some(Target::Directory(path)) => list_into(app, path),
        Some(Target::File(name)) => fail(
            app,
            &format!("{name}: opening a file on a host is not part of this build"),
        ),
    }
}

fn target(app: &App) -> Option<Target> {
    let PanelKind::Remote(panel) = &app.get_active_panel_ref().kind else {
        return None;
    };
    if panel.on_parent() {
        return Some(
            panel
                .path()
                .parent()
                .map_or(Target::Leave, Target::Directory),
        );
    }
    let entry = panel.selected()?;
    if !entry.kind.is_dir() {
        return Some(Target::File(entry.name.clone()));
    }
    panel.path().join(&entry.name).map(Target::Directory)
}

/// Show the directory `path` in the active remote panel.
pub fn list_into(app: &mut App, path: RemotePath) {
    let PanelKind::Remote(panel) = &mut app.get_active_panel().kind else {
        return;
    };
    let listed = panel.link().lock().list(&path);
    match listed {
        Ok(listing) => {
            let shown = panel.show(path, listing);
            let title = format!("NetBox: {}", panel.title());
            announce(app, shown, title);
        }
        Err(error) => fail(app, &error.to_string()),
    }
}

/// `..` at the root: back to local files. The connection goes with the panel.
pub fn leave(app: &mut App) {
    let PanelKind::Remote(panel) = &app.get_active_panel_ref().kind else {
        return;
    };
    let status = format!("NetBox: left {}", panel.session());
    super::sessions::back_to_files(app);
    app.set_status(Status::CommandOutput(status));
}

/// The directory now shown, and how many entries were left out for names
/// that cannot be a path part.
fn announce(app: &mut App, shown: Shown, shown_as: String) {
    if shown.unusable_names == 0 {
        app.set_status(Status::CommandOutput(shown_as));
        return;
    }
    fail(
        app,
        &format!(
            "{} entries left out: their names cannot be part of a path",
            shown.unusable_names
        ),
    );
}

fn fail(app: &mut App, text: &str) {
    app.set_status(Status::Error(format!("NetBox: {text}")));
}

#[cfg(test)]
mod tests;
