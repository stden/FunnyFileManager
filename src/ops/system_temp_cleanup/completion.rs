//! Purpose: Publish completed cleanup and refresh affected local file panels.
use crate::models::{
    temp_cleanup::{TempOutcome, TempRoot},
    App, PanelKind, PanelSide,
};
use crate::status::Status;

pub fn finish(app: &mut App, outcome: &TempOutcome, root: &TempRoot) {
    let mut message = format!(
        "Removed {} in {} temporary files; {} skipped; {} failed",
        outcome.freed, outcome.removed, outcome.skipped, outcome.failures
    );
    for side in [PanelSide::Left, PanelSide::Right] {
        let panel = app.panel_mut(side);
        if matches!(panel.kind, PanelKind::Files)
            && std::fs::canonicalize(&panel.path).is_ok_and(|path| path.starts_with(root.path()))
            && panel.reload().is_err()
        {
            message.push_str("; panel refresh failed");
        }
    }
    app.set_status(Status::CommandOutput(message));
}
