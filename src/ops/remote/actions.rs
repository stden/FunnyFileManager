//! Purpose: File-panel actions resolve remote operations before hidden local files can be touched.

use super::{context, enqueue};
use crate::action::{Action, DialogCommand, FileCommand, SelectionCommand};
use crate::lineedit::LineEditor;
use crate::models::remote_dialog::{RemoteDialog, RemoteInput};
use crate::models::remote_work::{RemoteTarget, RemoteTask};
use crate::models::{App, Overlay, PanelKind, RemoteOverlay};
use crate::remote::RemoteError;

pub fn handle(app: &mut App, action: Action) -> bool {
    let active_remote = matches!(app.get_active_panel_ref().kind, PanelKind::Remote(_));
    let transferring = matches!(action, Action::Copy | Action::Move)
        && matches!(app.get_inactive_panel_ref().kind, PanelKind::Remote(_));
    if !active_remote && !transferring {
        return false;
    }
    match apply(app, action) {
        Ok(handled) => handled,
        Err(error) => {
            super::fail(app, &error.to_string());
            true
        }
    }
}

fn apply(app: &mut App, action: Action) -> Result<bool, RemoteError> {
    match action {
        Action::Copy | Action::Move => transfer_prompt(app, action == Action::Move)?,
        Action::View | Action::Edit => open(app, action == Action::Edit)?,
        Action::Rename => rename_prompt(app)?,
        Action::DeleteConfirm => delete_prompt(app)?,
        Action::Dialog(DialogCommand::Mkdir) => {
            let base = context::directory(app.get_active_panel_ref())?;
            input(app, RemoteInput::Mkdir(base), "")?;
        }
        Action::File(FileCommand::Attributes) => permissions(app)?,
        Action::Refresh => super::completion::refresh(app),
        Action::Selection(command) => selection(app, command)?,
        _ => return Ok(false),
    }
    Ok(true)
}

fn transfer_prompt(app: &mut App, moving: bool) -> Result<(), RemoteError> {
    let sources = context::sources(app.get_active_panel_ref())?;
    if sources.is_empty() {
        return Ok(());
    }
    let directory = context::directory(app.get_inactive_panel_ref())?;
    let initial = directory.label();
    input(
        app,
        RemoteInput::Transfer {
            sources,
            directory,
            moving,
        },
        &initial,
    )
}

pub(super) fn open(app: &mut App, edit: bool) -> Result<(), RemoteError> {
    let Some(source) = context::selected(app.get_active_panel_ref())? else {
        return Ok(());
    };
    let target = context::operation_target(app)?;
    enqueue(app, target, RemoteTask::Open { source, edit });
    Ok(())
}

fn rename_prompt(app: &mut App) -> Result<(), RemoteError> {
    let Some(source) = context::selected(app.get_active_panel_ref())? else {
        return Ok(());
    };
    let name = source.name()?.to_owned();
    input(app, RemoteInput::Rename(source), &name)
}

fn delete_prompt(app: &mut App) -> Result<(), RemoteError> {
    let sources = context::sources(app.get_active_panel_ref())?;
    if sources.is_empty() {
        return Ok(());
    }
    let target = context::operation_target(app)?;
    app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(
        RemoteDialog::Delete {
            target,
            sources,
            answer: crate::models::Confirm::Yes,
        },
    ))));
    Ok(())
}

fn permissions(app: &mut App) -> Result<(), RemoteError> {
    let sources = context::sources(app.get_active_panel_ref())?;
    if sources.is_empty() {
        return Ok(());
    }
    let PanelKind::Remote(panel) = &app.get_active_panel_ref().kind else {
        return Ok(());
    };
    let initial = panel
        .selected()
        .map(|entry| format!("{:04o}", entry.attributes.bits() & 0o7777))
        .unwrap_or_default();
    input(app, RemoteInput::Chmod(sources), &initial)
}

fn selection(app: &mut App, command: SelectionCommand) -> Result<(), RemoteError> {
    let PanelKind::Remote(panel) = &mut app.get_active_panel().kind else {
        return Ok(());
    };
    match command {
        SelectionCommand::ToggleMark => panel.toggle_mark(),
        SelectionCommand::InvertSelection => panel.invert_marks(),
        SelectionCommand::RestoreSelection => panel.restore_marks(),
        SelectionCommand::SelectGroup | SelectionCommand::UnselectGroup => input(
            app,
            RemoteInput::Selection {
                select: command == SelectionCommand::SelectGroup,
            },
            "*",
        )?,
    }
    Ok(())
}

pub(super) fn input(app: &mut App, purpose: RemoteInput, initial: &str) -> Result<(), RemoteError> {
    let target = context::operation_target(app)?;
    show_input(app, target, purpose, initial);
    Ok(())
}

fn show_input(app: &mut App, target: RemoteTarget, purpose: RemoteInput, initial: &str) {
    app.set_overlay(Overlay::Remote(Box::new(RemoteOverlay(
        RemoteDialog::Input {
            target,
            purpose,
            input: LineEditor::new(initial),
            cancel: false,
            notice: String::new(),
        },
    ))));
}
