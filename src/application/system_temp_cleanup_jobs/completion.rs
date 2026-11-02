//! Purpose: Publish completed cleanup even after closing, without reopening stale scan dialogs.
use super::Answer;
use crate::application::Redraw;
use crate::models::{
    temp_cleanup::{TempChoice, TempCleanupState, TempError},
    App, Overlay,
};
use std::sync::Arc;
pub(super) fn publish(
    app: &mut App,
    generation: &Arc<()>,
    answer: Result<Answer, TempError>,
) -> Redraw {
    let redraw = if let Ok(Answer::Deleted { outcome, root }) = &answer {
        crate::ops::system_temp_cleanup::finish(app, outcome, root);
        Redraw::Needed
    } else {
        Redraw::Unchanged
    };
    let Overlay::TempCleanup(dialog) = &mut app.overlay else {
        return redraw;
    };
    if !Arc::ptr_eq(&dialog.generation, generation) {
        return redraw;
    }
    dialog.state = match answer {
        Ok(Answer::Scanned(plan)) => TempCleanupState::Preview {
            plan,
            focus: TempChoice::Cancel,
        },
        Ok(Answer::Deleted { outcome, .. }) => TempCleanupState::Finished(outcome),
        Err(error) => TempCleanupState::Failed(error),
    };
    Redraw::Needed
}
