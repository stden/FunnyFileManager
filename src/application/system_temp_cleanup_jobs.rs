//! Purpose: Scan/delete temporary files off the UI thread and reject stale scan replies.
use super::Redraw;
use crate::models::{
    temp_cleanup::{TempCleanupState, TempError, TempOutcome, TempPlan, TempSource},
    App, Overlay,
};
use std::{
    sync::{
        mpsc::{self, Receiver, TryRecvError},
        Arc,
    },
    time::SystemTime,
};

enum Work {
    Scan(TempSource),
    Delete(TempPlan),
}
enum Answer {
    Scanned(TempPlan),
    Deleted {
        outcome: TempOutcome,
        root: crate::models::temp_cleanup::TempRoot,
    },
}
struct Pending {
    generation: Arc<()>,
    receiver: Receiver<Result<Answer, TempError>>,
}
#[derive(Default)]
pub(super) struct TempCleanupJobs {
    pending: Option<Pending>,
}
impl TempCleanupJobs {
    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        let mut redraw = self.collect(app);
        if self.pending.is_some() {
            return redraw;
        }
        let Overlay::TempCleanup(dialog) = &mut app.overlay else {
            return redraw;
        };
        let Some(work) = take_work(&mut dialog.state) else {
            return redraw;
        };
        let (sender, receiver) = mpsc::channel();
        let generation = Arc::clone(&dialog.generation);
        let result = std::thread::Builder::new()
            .name("ffm-temp-cleanup".to_owned())
            .spawn(move || {
                drop(sender.send(execute(work))); // A scan can finish after its dialog closes.
            });
        if result.is_err() {
            dialog.state = TempCleanupState::Failed(TempError::Worker);
        } else {
            self.pending = Some(Pending {
                generation,
                receiver,
            });
        }
        redraw = Redraw::Needed;
        redraw
    }
    fn collect(&mut self, app: &mut App) -> Redraw {
        let Some(pending) = &self.pending else {
            return Redraw::Unchanged;
        };
        let answer = match pending.receiver.try_recv() {
            Ok(answer) => answer,
            Err(TryRecvError::Empty) => return Redraw::Unchanged,
            Err(TryRecvError::Disconnected) => Err(TempError::Worker),
        };
        let Some(pending) = self.pending.take() else {
            return Redraw::Unchanged;
        };
        completion::publish(app, &pending.generation, answer)
    }
}
fn take_work(state: &mut TempCleanupState) -> Option<Work> {
    let previous = std::mem::replace(state, TempCleanupState::Scanning);
    match previous {
        TempCleanupState::QueuedScan(source) => Some(Work::Scan(source)),
        TempCleanupState::QueuedDelete(plan) => {
            *state = TempCleanupState::Deleting;
            Some(Work::Delete(plan))
        }
        other => {
            *state = other;
            None
        }
    }
}
fn execute(work: Work) -> Result<Answer, TempError> {
    match work {
        Work::Scan(source) => {
            crate::ops::system_temp_cleanup::scan(&source, SystemTime::now()).map(Answer::Scanned)
        }
        Work::Delete(plan) => Ok(Answer::Deleted {
            outcome: crate::ops::system_temp_cleanup::remove(&plan),
            root: plan.root,
        }),
    }
}

mod completion;
#[cfg(test)]
mod tests;
