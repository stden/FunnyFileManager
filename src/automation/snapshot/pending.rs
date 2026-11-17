//! Purpose: Identify finite queued or running work without treating persistent samplers as jobs.

use crate::models::{
    temp_cleanup::TempCleanupState, things::ThingsState, token_usage::UsageState, App, ImageState,
    Overlay, PanelKind, PanelSide, Preview, ViewMode, WslActivity,
};

pub(super) fn is_pending(app: &App) -> bool {
    if !app.remote_requests.is_idle()
        || !app.wsl_requests.pending().is_empty()
        || images_pending(app)
    {
        return true;
    }
    match &app.overlay {
        Overlay::TokenUsage(panel) => {
            matches!(panel.state, UsageState::Queued | UsageState::Loading)
        }
        Overlay::Things(dialog) => {
            matches!(dialog.state, ThingsState::Queued(_) | ThingsState::Loading)
        }
        Overlay::Wsl(dialog) => matches!(dialog.activity, WslActivity::Busy { .. }),
        Overlay::TempCleanup(dialog) => matches!(
            dialog.state,
            TempCleanupState::QueuedScan(_)
                | TempCleanupState::Scanning
                | TempCleanupState::QueuedDelete(_)
                | TempCleanupState::Deleting
        ),
        _ => false,
    }
}

fn images_pending(app: &App) -> bool {
    if let Overlay::Viewer(viewer) = &app.overlay {
        if viewer.viewed_file().is_some()
            && viewer.image.as_ref().is_some_and(super::image::is_pending)
        {
            return true;
        }
    }
    [PanelSide::Left, PanelSide::Right]
        .into_iter()
        .any(|side| panel_image_pending(app, side))
}

fn panel_image_pending(app: &App, side: PanelSide) -> bool {
    let PanelKind::QuickView(view) = &app.panel(side).kind else {
        return false;
    };
    let Preview::Image(state) = &view.preview else {
        return false;
    };
    if view.shown.is_none() {
        return false;
    }
    match state {
        ImageState::Loading(_) => true,
        ImageState::Pending => {
            app.view == ViewMode::Panels
                && !matches!(app.overlay, Overlay::Viewer(_) | Overlay::Editor(_))
                && app.panels_on_screen().shows(side)
        }
        ImageState::Ready(_) | ImageState::Failed(_) => false,
    }
}

#[cfg(test)]
mod tests;
