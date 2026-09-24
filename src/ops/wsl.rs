//! Canonical WSL request and completion transitions; native execution is injected.

use crate::models::{App, Overlay, WslActivity, WslDialog, WslOperation, WslOutput, WslRequestId};
use crate::status::Status;
use std::path::PathBuf;

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
    match result {
        Ok(WslOutput::Root(path)) => open_root(app, path),
        Ok(WslOutput::Distributions(distributions)) => {
            if let Overlay::Wsl(dialog) = &mut app.overlay {
                dialog.replace_distributions(distributions);
                dialog.activity = WslActivity::Ready;
            }
        }
        Ok(WslOutput::Updated(output) | WslOutput::PackagesUpdated(output)) => {
            show(
                app,
                WslActivity::Notice(output.clone()),
                Status::CommandOutput(output),
            );
        }
        Err(error) => fail(app, error),
    }
}

fn open_root(app: &mut App, path: PathBuf) {
    match app.get_active_panel().navigate_to(path.clone()) {
        Ok(()) => {
            app.close_overlay();
            app.set_status(Status::Opened(path));
        }
        Err(error) => fail(
            app,
            format!("Cannot open WSL root {}: {error}", path.display()),
        ),
    }
}

fn fail(app: &mut App, error: String) {
    show(
        app,
        WslActivity::Failed(error.clone()),
        Status::Error(error),
    );
}

/// Put `activity` in the dialog and `status` on the bar; a dialog that has
/// closed meanwhile takes neither.
fn show(app: &mut App, activity: WslActivity, status: Status) {
    let Overlay::Wsl(dialog) = &mut app.overlay else {
        return;
    };
    dialog.activity = activity;
    app.set_status(status);
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
