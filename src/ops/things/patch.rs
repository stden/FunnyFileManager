//! Purpose: Patch confirmation and workflow reporting for the Things catalog dialog.

use std::sync::Arc;

use crate::models::things::ThingsState;
use crate::models::{App, Overlay};
use crate::status::Status;
use crate::things::{build_plan, PatchReport, PatchScope, ThingsError};

/// Build a patch plan from the chosen machines and open the confirmation prompt.
pub fn request_patch(app: &mut App) {
    let Overlay::Things(dialog) = &mut app.overlay else {
        return;
    };
    let prev = std::mem::replace(&mut dialog.state, ThingsState::Patching);
    let ThingsState::Ready(catalog) = prev else {
        dialog.state = prev;
        return;
    };
    let plan = build_plan(
        catalog.chosen(),
        PatchScope::All,
        true,
        "Patch selected hosts via FunnyFileManager",
    );
    if plan.waves.is_empty() {
        dialog.state = ThingsState::Ready(catalog);
        app.set_status(Status::Error(
            "Things: no patchable machines selected".to_owned(),
        ));
        return;
    }
    dialog.state = ThingsState::ConfirmPatch { catalog, plan };
}

/// Return from the patch confirmation prompt back to the machine catalog.
pub fn cancel_patch(app: &mut App) {
    let Overlay::Things(dialog) = &mut app.overlay else {
        return;
    };
    let prev = std::mem::replace(&mut dialog.state, ThingsState::Patching);
    if let ThingsState::ConfirmPatch { catalog, .. } = prev {
        dialog.state = ThingsState::Ready(catalog);
    } else {
        dialog.state = prev;
    }
}

/// Confirm the patch plan and queue it for background execution.
pub fn confirm_patch(app: &mut App) {
    let Overlay::Things(dialog) = &mut app.overlay else {
        return;
    };
    let prev = std::mem::replace(&mut dialog.state, ThingsState::Patching);
    let ThingsState::ConfirmPatch { plan, .. } = prev else {
        dialog.state = prev;
        return;
    };
    match dialog.config.clone() {
        Some(config) => dialog.state = ThingsState::PatchQueued { config, plan },
        None => {
            dialog.state = ThingsState::Failed("Things configuration is unavailable".to_owned());
        }
    }
}

/// Publish the result of a background patch execution back into the UI.
pub fn publish_patch(
    app: &mut App,
    generation: &Arc<()>,
    answer: Result<PatchReport, ThingsError>,
) {
    let Overlay::Things(dialog) = &mut app.overlay else {
        return;
    };
    if !Arc::ptr_eq(&dialog.generation, generation) {
        return;
    }
    match answer {
        Ok(report) => {
            for (idx, group) in report.groups.iter().enumerate() {
                app.console.record_output(&format!(
                    "Things patch wave {}: group={} status={}",
                    idx + 1,
                    group.id,
                    group.status
                ));
            }
            let summary = report.summary();
            app.close_overlay();
            app.set_status(Status::CommandOutput(format!(
                "Things patch started: {summary}"
            )));
        }
        Err(error) => dialog.state = ThingsState::Failed(error.message().to_owned()),
    }
}
