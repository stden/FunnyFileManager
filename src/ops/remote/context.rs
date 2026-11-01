//! Purpose: Resolve visible panel locations and match asynchronous replies to their origin.

use crate::models::remote_work::{FileLocation, RemoteTarget};
use crate::models::{App, Panel, PanelKind, PanelSide};
use crate::remote::RemoteError;

pub(super) fn directory(panel: &Panel) -> Result<FileLocation, RemoteError> {
    match &panel.kind {
        PanelKind::Remote(remote) => Ok(FileLocation::Remote {
            link: remote.link().clone(),
            path: remote.path().clone(),
        }),
        PanelKind::Files => Ok(FileLocation::Local(panel.path.clone())),
        _ => Err(RemoteError::Unsupported("transfer to this panel kind")),
    }
}

pub(super) fn sources(panel: &Panel) -> Result<Vec<FileLocation>, RemoteError> {
    match &panel.kind {
        PanelKind::Remote(remote) => {
            let base = directory(panel)?;
            remote
                .action_names()
                .iter()
                .map(|name| base.child(name))
                .collect()
        }
        PanelKind::Files => Ok(panel
            .action_paths()
            .into_iter()
            .map(FileLocation::Local)
            .collect()),
        _ => Err(RemoteError::Unsupported("transfer from this panel kind")),
    }
}

pub(super) fn selected(panel: &Panel) -> Result<Option<FileLocation>, RemoteError> {
    match &panel.kind {
        PanelKind::Remote(remote) => remote
            .selected()
            .filter(|entry| entry.name != "..")
            .map(|entry| directory(panel)?.child(&entry.name))
            .transpose(),
        PanelKind::Files => Ok(panel.selected_path().map(FileLocation::Local)),
        _ => Err(RemoteError::Unsupported("opening an entry in this panel")),
    }
}

pub(super) fn operation_target(app: &App) -> Result<RemoteTarget, RemoteError> {
    for side in [app.active_panel, other(app.active_panel)] {
        if let PanelKind::Remote(remote) = &app.panel(side).kind {
            return Ok(RemoteTarget::Panel {
                side,
                link: remote.link().clone(),
                path: remote.path().clone(),
            });
        }
    }
    Err(RemoteError::Failed("no remote panel is open"))
}

pub(super) fn current(app: &App, target: &RemoteTarget) -> bool {
    match target {
        RemoteTarget::Connect { side, session } => matches!(&app.panel(*side).kind,
            PanelKind::Sessions(list) if list.selected_name() == Some(session)),
        RemoteTarget::Panel { side, link, path } => matches!(&app.panel(*side).kind,
            PanelKind::Remote(remote) if remote.link() == link && remote.path() == path),
    }
}

pub(super) const fn other(side: PanelSide) -> PanelSide {
    match side {
        PanelSide::Left => PanelSide::Right,
        PanelSide::Right => PanelSide::Left,
    }
}
