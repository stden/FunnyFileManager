//! Purpose: Preview and explicitly confirmed cleanup of old temporary files only.
use crate::models::temp_cleanup::{TempChoice, TempCleanupState, TempError};
use crate::models::{App, Confirm, Overlay, TempCleanupDialog};
mod completion;
mod files;
pub use completion::finish;
pub use files::{remove, scan};
#[cfg(test)]
mod tests;

pub fn open(app: &mut App) {
    let source = app.live.temp_cleanup.clone();
    app.set_overlay(Overlay::TempCleanup(Box::new(TempCleanupDialog::new(
        source,
    ))));
}
pub fn cancel(app: &mut App) {
    app.close_overlay();
}
pub fn toggle(app: &mut App) {
    let Overlay::TempCleanup(dialog) = &mut app.overlay else {
        return;
    };
    match &mut dialog.state {
        TempCleanupState::Preview { focus, .. } => {
            *focus = match focus {
                TempChoice::Clean => TempChoice::Cancel,
                TempChoice::Cancel => TempChoice::Clean,
            }
        }
        TempCleanupState::Confirm { answer, .. } => *answer = answer.toggled(),
        _ => {}
    }
}
pub fn answer(app: &mut App, choice: Confirm) {
    if let Overlay::TempCleanup(dialog) = &mut app.overlay {
        if let TempCleanupState::Confirm { answer, .. } = &mut dialog.state {
            *answer = choice;
            accept(app);
        }
    }
}
pub fn accept(app: &mut App) {
    let Overlay::TempCleanup(dialog) = &mut app.overlay else {
        return;
    };
    let state = std::mem::replace(
        &mut dialog.state,
        TempCleanupState::Failed(TempError::Worker),
    );
    match next(state) {
        Next::State(state) => dialog.state = state,
        Next::Close => app.close_overlay(),
    }
}
enum Next {
    State(TempCleanupState),
    Close,
}
fn next(state: TempCleanupState) -> Next {
    use TempCleanupState::{Confirm as Question, Preview};
    let state = match state {
        Preview {
            plan,
            focus: TempChoice::Clean,
        } if !plan.files.is_empty() => Question {
            plan,
            answer: Confirm::No,
        },
        Preview { .. } | TempCleanupState::Finished(_) | TempCleanupState::Failed(_) => {
            return Next::Close
        }
        Question {
            plan,
            answer: Confirm::Yes,
        } => TempCleanupState::QueuedDelete(plan),
        Question { plan, .. } => Preview {
            plan,
            focus: TempChoice::Cancel,
        },
        other => other,
    };
    Next::State(state)
}
