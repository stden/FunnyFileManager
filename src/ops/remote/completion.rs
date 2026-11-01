//! Purpose: Publish worker results only to the panel or document that requested them.

use super::{context, enqueue};
use crate::models::remote_work::{
    RemoteListing, RemoteOutcome, RemoteRefresh, RemoteRequest, RemoteTarget, RemoteTask,
};
use crate::models::{App, PanelKind, PanelSide, RemotePanel};
use crate::remote::RemoteError;
use crate::status::Status;

mod failure;

pub fn apply(app: &mut App, request: RemoteRequest, answer: Result<RemoteOutcome, RemoteError>) {
    match answer {
        Ok(RemoteOutcome::Saved) => super::editor::saved(app, &request.task),
        Ok(RemoteOutcome::Done) => {
            app.set_status(Status::CommandOutput(
                "Remote operation completed".to_owned(),
            ));
            refresh(app);
        }
        Ok(RemoteOutcome::Refreshed(entries)) => {
            for entry in entries {
                apply_refresh(app, entry);
            }
        }
        Ok(outcome) if context::current(app, &request.target) => {
            publish(app, &request.target, outcome);
        }
        Err(error) => failure::apply(app, request, &error),
        Ok(_) => {} // A late reply never changes a panel the user has left.
    }
}

fn apply_refresh(app: &mut App, refresh: RemoteRefresh) {
    let RemoteListing { side, link, path } = refresh.listing;
    let PanelKind::Remote(panel) = &mut app.panel_mut(side).kind else {
        return;
    };
    if panel.link() != &link || panel.path() != &path {
        return;
    }
    match refresh.result {
        Ok(entries) => {
            let shown = panel.show(path, entries);
            if shown.unusable_names > 0 {
                super::fail(app, "Some remote names cannot be represented");
            }
        }
        Err(error) => super::fail(app, &error.to_string()),
    }
}

fn publish(app: &mut App, target: &RemoteTarget, outcome: RemoteOutcome) {
    match outcome {
        RemoteOutcome::Listed {
            link,
            path,
            entries,
        } => {
            listed(app, target, link, path, entries);
        }
        RemoteOutcome::Opened {
            source,
            bytes,
            edit,
        } if !app.overlay.is_active() => {
            if let Err(error) = super::editor::open(app, source, &bytes, edit) {
                super::fail(app, &error.to_string());
            }
        }
        _ => {}
    }
}

pub fn refresh(app: &mut App) {
    let mut listings = Vec::new();
    for side in [PanelSide::Left, PanelSide::Right] {
        match &mut app.panel_mut(side).kind {
            PanelKind::Remote(remote) => listings.push(RemoteListing {
                side,
                link: remote.link().clone(),
                path: remote.path().clone(),
            }),
            PanelKind::Files => {
                if let Err(error) = app.panel_mut(side).reload() {
                    super::fail(app, &error.to_string());
                }
            }
            _ => {}
        }
    }
    if let Ok(target) = context::operation_target(app) {
        if !listings.is_empty() {
            enqueue(app, target, RemoteTask::Refresh(listings));
        }
    }
}

fn listed(
    app: &mut App,
    target: &RemoteTarget,
    link: crate::remote::RemoteLink,
    path: crate::models::RemotePath,
    entries: Vec<crate::models::DirEntry>,
) {
    let side = match target {
        RemoteTarget::Connect { side, session } => {
            app.panel_mut(*side).kind =
                PanelKind::Remote(Box::new(RemotePanel::new(session.clone(), link)));
            *side
        }
        RemoteTarget::Panel { side, .. } => *side,
    };
    if let PanelKind::Remote(panel) = &mut app.panel_mut(side).kind {
        let shown = panel.show(path, entries);
        let title = format!("NetBox: {}", panel.title());
        super::announce(app, shown, title);
    }
}
