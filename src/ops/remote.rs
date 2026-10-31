//! Purpose: Browsing a remote host through a stored session — connecting,
//! listing a directory into the panel, entering one, and leaving.
//!
//! Everything goes through the [`RemoteFs`] the panel carries, so the same
//! code runs over isolated test hosts and native SFTP/FTP transports.
//! Network work runs in the background; failures reach the status bar.

use crate::models::{
    remote_panel::Shown, App, PanelKind, RemotePanel, RemotePath, Session, SessionName,
};
use crate::remote::{RemoteError, RemoteFs, RemoteLink};
use crate::status::Status;

pub(crate) mod actions;
pub(crate) mod completion;
mod context;
pub(crate) mod dialogs;
pub(crate) mod editor;
pub(crate) mod jobs;
mod read;
mod transfer;

/// Enter on a session: open its host in the active panel.
pub fn connect(app: &mut App, name: &SessionName) {
    let Some(session) = app.sessions.find(name).cloned() else {
        fail(app, &format!("no session named {name}"));
        return;
    };
    let login = crate::models::remote_dialog::RemoteLogin::new(session);
    app.set_overlay(crate::models::Overlay::Remote(Box::new(
        crate::models::RemoteOverlay(crate::models::remote_dialog::RemoteDialog::Login(login)),
    )));
}

pub(crate) fn enqueue(
    app: &mut App,
    target: crate::models::remote_work::RemoteTarget,
    task: crate::models::remote_work::RemoteTask,
) {
    match app.remote_requests.queue(target, task) {
        Ok(()) => app.set_status(Status::CommandOutput("Remote operation started".to_owned())),
        Err(error) => fail(app, &error.to_string()),
    }
}

/// Enter queues a directory listing or text download; the UI remains responsive.
pub fn enter(app: &mut App) {
    match target(app) {
        Some(Target::Leave) => leave(app),
        Some(Target::Directory(path)) => request_list(app, path),
        Some(Target::File) => {
            if let Err(error) = actions::open(app, false) {
                fail(app, &error.to_string());
            }
        }
        None => {}
    }
}

pub(crate) fn request_list(app: &mut App, path: RemotePath) {
    if let Ok(target) = context::operation_target(app) {
        if let crate::models::remote_work::RemoteTarget::Panel { link, .. } = &target {
            let task = crate::models::remote_work::RemoteTask::List {
                link: link.clone(),
                path,
            };
            enqueue(app, target, task);
        }
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
    File,
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
        return Some(Target::File);
    }
    panel.path().join(&entry.name).map(Target::Directory)
}

/// Show the directory `path` in the active remote panel.
pub fn list_into(app: &mut App, path: RemotePath) {
    if matches!(app.get_active_panel_ref().kind, PanelKind::Remote(_)) {
        request_list(app, path);
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
