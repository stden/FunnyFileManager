//! Canonical WSL request and completion transitions; native execution is injected.

use crate::models::{App, Overlay, WslActivity, WslDialog, WslOperation, WslOutput, WslRequestId};
use crate::status::Status;

pub fn open_manager(app: &mut App) {
    app.set_overlay(Overlay::Wsl(Box::new(WslDialog::new(Vec::new()))));
    refresh(app);
}

pub fn refresh(app: &mut App) {
    enqueue(app, WslOperation::Refresh);
}

pub fn update(app: &mut App) {
    enqueue(app, WslOperation::Update);
}

pub fn update_packages(app: &mut App) {
    selected_operation(app, WslOperation::UpdatePackages);
}

pub fn open_selected(app: &mut App) {
    selected_operation(app, WslOperation::Open);
}

fn selected_operation(
    app: &mut App,
    operation: impl FnOnce(crate::models::DistributionName) -> WslOperation,
) {
    let Overlay::Wsl(dialog) = &mut app.overlay else {
        return;
    };
    if matches!(dialog.activity, WslActivity::Busy { .. }) {
        return;
    }
    let Some(name) = dialog.selected_distribution().cloned() else {
        dialog.activity = WslActivity::Failed("No WSL distribution selected".to_owned());
        return;
    };
    enqueue(app, operation(name));
}

fn enqueue(app: &mut App, operation: WslOperation) {
    let Overlay::Wsl(dialog) = &mut app.overlay else {
        return;
    };
    if matches!(dialog.activity, WslActivity::Busy { .. }) {
        return;
    }
    let id = app.wsl_requests.enqueue(operation.clone());
    dialog.activity = WslActivity::Busy { id, operation };
}

/// Ignore stale dialog results while still reporting background maintenance outcomes.
pub(crate) fn complete(app: &mut App, id: WslRequestId, result: Result<WslOutput, String>) {
    let current = matches!(&app.overlay, Overlay::Wsl(dialog)
        if matches!(dialog.activity, WslActivity::Busy { id: active, .. } if active == id));
    if !current {
        report_background_result(app, result);
        return;
    }

    apply_current_result(app, result);
}

fn apply_current_result(app: &mut App, result: Result<WslOutput, String>) {
    let result = match result {
        Ok(WslOutput::Root(path)) => match app.get_active_panel().navigate_to(path.clone()) {
            Ok(()) => {
                app.close_overlay();
                app.set_status(Status::Opened(path));
                return;
            }
            Err(error) => Err(format!("Cannot open WSL root {}: {error}", path.display())),
        },
        other => other,
    };
    let Overlay::Wsl(dialog) = &mut app.overlay else {
        return;
    };
    match result {
        Ok(WslOutput::Distributions(distributions)) => {
            dialog.replace_distributions(distributions);
            dialog.activity = WslActivity::Ready;
        }
        Ok(WslOutput::Updated(output) | WslOutput::PackagesUpdated(output)) => {
            dialog.activity = WslActivity::Notice(output.clone());
            app.set_status(Status::CommandOutput(output));
        }
        Err(error) => {
            dialog.activity = WslActivity::Failed(error.clone());
            app.set_status(Status::Error(error));
        }
        Ok(WslOutput::Root(_)) => unreachable!("root navigation handled above"),
    }
}

fn report_background_result(app: &mut App, result: Result<WslOutput, String>) {
    match result {
        Ok(WslOutput::Updated(output) | WslOutput::PackagesUpdated(output)) => {
            app.set_status(Status::CommandOutput(output));
        }
        Err(error) => app.set_status(Status::Error(error)),
        _ => {}
    }
}

#[cfg(test)]
mod tests;
