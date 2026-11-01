//! Purpose: The Things read runs off the event loop, one at a time. A reply
//! for a dialog that was closed or reopened since it was asked for is dropped.

use std::sync::{mpsc, Arc};

use super::Redraw;
use crate::models::things::{ThingsAccess, ThingsState};
use crate::models::{App, Overlay};
use crate::things::{Config, Listing, ThingsError};

type Answer = Result<Listing, ThingsError>;

struct Pending {
    generation: Arc<()>,
    receiver: mpsc::Receiver<Answer>,
}

#[derive(Default)]
pub(super) struct ThingsJobs {
    pending: Option<Pending>,
}

impl ThingsJobs {
    /// Read the catalog when the live input allows it; otherwise answer the
    /// queued read with [`ThingsError::Disabled`] without touching the network.
    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        match app.live.things {
            ThingsAccess::Network => {
                self.pump_with(app, |config: Config| crate::things::fetch(&config))
            }
            ThingsAccess::Offline => {
                self.pump_with(app, |_config: Config| Err(ThingsError::Disabled))
            }
        }
    }

    /// Publish the answer that came back, then start the queued read with `fetch`.
    pub(super) fn pump_with(
        &mut self,
        app: &mut App,
        fetch: impl FnOnce(Config) -> Answer + Send + 'static,
    ) -> Redraw {
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
        let generation = Arc::clone(&dialog.generation);
        if spawn(sender, move || fetch(config)).is_ok() {
            self.pending = Some(Pending {
                generation,
                receiver,
            });
        } else {
            dialog.state = ThingsState::Failed(ThingsError::Worker.message().to_owned());
        }
        Redraw::Needed
    }

    fn collect(&mut self, app: &mut App) -> Redraw {
        let Some(pending) = &self.pending else {
            return Redraw::Unchanged;
        };
        let answer = match pending.receiver.try_recv() {
            Ok(answer) => answer,
            Err(mpsc::TryRecvError::Empty) => return Redraw::Unchanged,
            Err(mpsc::TryRecvError::Disconnected) => Err(ThingsError::Worker),
        };
        if let Some(pending) = self.pending.take() {
            crate::ops::things::publish(app, &pending.generation, answer);
        }
        Redraw::Needed
    }
}

fn spawn(
    sender: mpsc::Sender<Answer>,
    work: impl FnOnce() -> Answer + Send + 'static,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("ffm-things".to_owned())
        .spawn(move || {
            drop(sender.send(work())); // Closing the dialog may discard its answer.
        })
}

#[cfg(test)]
mod tests;
