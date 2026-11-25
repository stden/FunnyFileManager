//! Purpose: Things read and patch requests run off the event loop, one at a time.
//! A reply for a dialog that was closed or reopened since is dropped.

mod spawn;

use std::sync::{mpsc, Arc};

use super::Redraw;
use crate::models::things::{ThingsAccess, ThingsDialog, ThingsState};
use crate::models::{App, Overlay};
use crate::things::{Config, Listing, PatchPlan, PatchReport, ThingsError};
use spawn::{spawn_patch, spawn_read};

type Answer = Result<Listing, ThingsError>;
type PatchAnswer = Result<PatchReport, ThingsError>;

pub(super) enum JobAnswer {
    Read(Answer),
    Patch(PatchAnswer),
}

struct Pending {
    generation: Arc<()>,
    receiver: mpsc::Receiver<JobAnswer>,
}

#[derive(Default)]
pub(super) struct ThingsJobs {
    pending: Option<Pending>,
}

impl ThingsJobs {
    /// Run a queued Things catalog read or patch job as allowed by live input.
    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        let is_patch = matches!(
            &app.overlay,
            Overlay::Things(dialog) if matches!(dialog.state, ThingsState::PatchQueued { .. })
        );
        match (is_patch, app.live.things) {
            (true, ThingsAccess::Network) => {
                self.pump_patch_with(app, |cfg, plan| crate::things::execute_plan(&cfg, &plan))
            }
            (true, ThingsAccess::Offline) => {
                self.pump_patch_with(app, |_cfg, _plan| Err(ThingsError::Disabled))
            }
            (false, ThingsAccess::Network) => self.pump_with(app, |cfg| crate::things::fetch(&cfg)),
            (false, ThingsAccess::Offline) => {
                self.pump_with(app, |_cfg| Err(ThingsError::Disabled))
            }
        }
    }

    /// Publish any finished reply, then start a queued read with `fetch`.
    pub(super) fn pump_with(
        &mut self,
        app: &mut App,
        fetch: impl FnOnce(Config) -> Answer + Send + 'static,
    ) -> Redraw {
        self.pump_using(app, fetch, spawn_read)
    }

    /// Publish any finished reply, then start a queued patch with `run`.
    pub(super) fn pump_patch_with(
        &mut self,
        app: &mut App,
        run: impl FnOnce(Config, PatchPlan) -> PatchAnswer + Send + 'static,
    ) -> Redraw {
        let collected = self.collect(app);
        if self.pending.is_some() {
            return collected;
        }
        let Overlay::Things(dialog) = &mut app.overlay else {
            return collected;
        };
        let Some((config, plan)) = dialog.start_patch() else {
            return collected;
        };
        let (sender, receiver) = mpsc::channel();
        let started = spawn_patch(sender, config, plan, run);
        self.record_pending(dialog, receiver, &started)
    }

    fn pump_using<F>(
        &mut self,
        app: &mut App,
        fetch: F,
        launch: impl FnOnce(
            mpsc::Sender<JobAnswer>,
            Config,
            F,
        ) -> std::io::Result<std::thread::JoinHandle<()>>,
    ) -> Redraw
    where
        F: FnOnce(Config) -> Answer + Send + 'static,
    {
        let collected = self.collect(app);
        if self.pending.is_some() {
            return collected;
        }
        let Overlay::Things(dialog) = &mut app.overlay else {
            return collected;
        };
        let Some(config) = dialog.start() else {
            return collected;
        };
        let (sender, receiver) = mpsc::channel();
        let started = launch(sender, config, fetch);
        self.record_pending(dialog, receiver, &started)
    }

    fn record_pending(
        &mut self,
        dialog: &mut ThingsDialog,
        receiver: mpsc::Receiver<JobAnswer>,
        started: &std::io::Result<std::thread::JoinHandle<()>>,
    ) -> Redraw {
        if started.is_ok() {
            self.pending = Some(Pending {
                generation: Arc::clone(&dialog.generation),
                receiver,
            });
        } else {
            dialog.state = ThingsState::Failed(ThingsError::Worker.message().to_owned());
        }
        Redraw::Needed
    }

    fn collect(&mut self, app: &mut App) -> Redraw {
        let Some(pending) = self.pending.take() else {
            return Redraw::Unchanged;
        };
        match pending.receiver.try_recv() {
            Ok(JobAnswer::Read(answer)) => {
                crate::ops::things::publish(app, &pending.generation, answer);
            }
            Ok(JobAnswer::Patch(answer)) => {
                crate::ops::things::publish_patch(app, &pending.generation, answer);
            }
            Err(mpsc::TryRecvError::Empty) => {
                self.pending = Some(pending);
                return Redraw::Unchanged;
            }
            Err(mpsc::TryRecvError::Disconnected) => publish_disconnected(app, &pending.generation),
        }
        Redraw::Needed
    }
}

fn publish_disconnected(app: &mut App, generation: &Arc<()>) {
    if matches!(&app.overlay, Overlay::Things(dialog) if matches!(dialog.state, ThingsState::Patching))
    {
        crate::ops::things::publish_patch(app, generation, Err(ThingsError::Worker));
    } else {
        crate::ops::things::publish(app, generation, Err(ThingsError::Worker));
    }
}

#[cfg(test)]
mod tests;
