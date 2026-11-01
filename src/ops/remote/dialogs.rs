//! Purpose: Validated remote dialog answers become queued operations.

use super::{context, enqueue};
use crate::models::remote_dialog::{RemoteInput, RemoteLogin};
use crate::models::remote_work::{RemoteTarget, RemoteTask};
use crate::models::{App, PanelKind};
use crate::remote::{RemoteError, UnixMode};

pub fn login(app: &mut App, mut login: RemoteLogin) {
    login.user.text().clone_into(&mut login.credentials.user);
    let target = RemoteTarget::Connect {
        side: app.active_panel,
        session: login.session.name.clone(),
    };
    enqueue(
        app,
        target,
        RemoteTask::Connect {
            session: login.session,
            credentials: login.credentials,
        },
    );
}

pub fn input(
    app: &mut App,
    target: RemoteTarget,
    purpose: RemoteInput,
    typed: &str,
) -> Result<(), RemoteError> {
    if !context::current(app, &target) {
        return Err(RemoteError::Failed("the remote panel has changed"));
    }
    if let RemoteInput::Selection { select } = purpose {
        select_matching(app, &target, typed, select);
        return Ok(());
    }
    let task = task(purpose, typed)?;
    enqueue(app, target, task);
    Ok(())
}

fn task(purpose: RemoteInput, typed: &str) -> Result<RemoteTask, RemoteError> {
    Ok(match purpose {
        RemoteInput::Transfer {
            sources,
            directory,
            moving,
        } => RemoteTask::Transfer {
            sources,
            destination: directory.resolve(typed)?,
            moving,
        },
        RemoteInput::Rename(from) => {
            let to = from.parent()?.child(typed)?;
            RemoteTask::Rename { from, to }
        }
        RemoteInput::Mkdir(directory) => RemoteTask::Mkdir(directory.resolve(typed)?),
        RemoteInput::Chmod(targets) => RemoteTask::Chmod {
            targets,
            mode: UnixMode::parse(typed)?,
        },

        RemoteInput::Selection { .. } => {
            return Err(RemoteError::InvalidInput("selection is not a job"))
        }
    })
}

fn select_matching(app: &mut App, target: &RemoteTarget, typed: &str, select: bool) {
    let RemoteTarget::Panel { side, .. } = target else {
        return;
    };
    if let PanelKind::Remote(panel) = &mut app.panel_mut(*side).kind {
        panel.mark_matching(&crate::mask::Mask::parse(typed), select);
    }
}
