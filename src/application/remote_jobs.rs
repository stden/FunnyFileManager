//! Purpose: Run bounded remote operations on workers and publish results on the event loop.

use super::Redraw;
use crate::models::remote_work::{RemoteOutcome, RemoteRequest};
use crate::models::App;
use crate::remote::RemoteError;
use std::sync::mpsc::{self, Receiver, TryRecvError};

type Answer = (RemoteRequest, Result<RemoteOutcome, RemoteError>);

struct Pending {
    id: u64,
    receiver: Receiver<Answer>,
}

#[derive(Default)]
pub(super) struct RemoteJobs {
    pending: Option<Pending>,
}

impl RemoteJobs {
    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        let redraw = self.collect(app);
        if self.pending.is_some() {
            return redraw;
        }
        if app.remote_requests.is_idle() {
            return redraw;
        }
        let Some(request) = app.remote_requests.take() else {
            return redraw;
        };
        let id = request.id;
        let (sender, receiver) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("ffm-remote".to_owned())
            .spawn(move || {
                let answer = crate::ops::remote::jobs::execute(&request.task);
                drop(sender.send((request, answer))); // The app can close before a connection times out.
            });
        if spawned.is_err() {
            app.remote_requests.finish(id);
            app.set_status(crate::status::Status::Error(
                "Cannot start remote worker".to_owned(),
            ));
            return Redraw::Needed;
        }
        self.pending = Some(Pending { id, receiver });
        Redraw::Needed
    }

    fn collect(&mut self, app: &mut App) -> Redraw {
        let Some(pending) = &self.pending else {
            return Redraw::Unchanged;
        };
        match pending.receiver.try_recv() {
            Ok((request, answer)) => {
                self.pending = None;
                if app.remote_requests.finish(request.id) {
                    crate::ops::remote::completion::apply(app, request, answer);
                }
                Redraw::Needed
            }
            Err(TryRecvError::Empty) => Redraw::Unchanged,
            Err(TryRecvError::Disconnected) => {
                if let Some(pending) = self.pending.take() {
                    app.remote_requests.finish(pending.id);
                }
                app.set_status(crate::status::Status::Error(
                    "Remote worker stopped unexpectedly".to_owned(),
                ));
                Redraw::Needed
            }
        }
    }
}

#[cfg(test)]
mod tests;
