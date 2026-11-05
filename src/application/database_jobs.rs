//! Purpose: Run the explorer's database requests on one worker thread, off the
//! UI thread, and deliver each answer to the dialog that asked for it.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use super::Redraw;
use crate::models::database::{Reply, Request, RequestId};
use crate::models::App;
use crate::ops::database::DbError;
use crate::status::Status;

type Answer = (RequestId, Result<Reply, DbError>);

struct Running {
    id: RequestId,
    receiver: Receiver<Answer>,
}

#[derive(Default)]
pub(super) struct DatabaseJobs {
    running: Option<Running>,
}

impl DatabaseJobs {
    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        let redraw = self.collect(app);
        if self.running.is_some() {
            return redraw;
        }
        let Some(request) = app.database_requests.take() else {
            return redraw;
        };
        self.start(app, request);
        Redraw::Needed
    }

    fn start(&mut self, app: &mut App, request: Request) {
        let id = request.id;
        let (sender, receiver) = mpsc::channel();
        let spawned = std::thread::Builder::new()
            .name("ffm-database".to_owned())
            .spawn(move || {
                let answer = crate::ops::database::execute(&request.task);
                drop(sender.send((request.id, answer))); // The dialog may close before the answer.
            });
        if spawned.is_err() {
            app.database_requests.finish(id);
            app.set_status(Status::Error("Cannot start database worker".to_owned()));
            return;
        }
        self.running = Some(Running { id, receiver });
    }

    fn collect(&mut self, app: &mut App) -> Redraw {
        let Some(running) = &self.running else {
            return Redraw::Unchanged;
        };
        match running.receiver.try_recv() {
            Ok((id, answer)) => {
                self.running = None;
                if app.database_requests.finish(id) {
                    crate::ops::database::apply(app, id, answer);
                }
                Redraw::Needed
            }
            Err(TryRecvError::Empty) => Redraw::Unchanged,
            Err(TryRecvError::Disconnected) => {
                if let Some(running) = self.running.take() {
                    app.database_requests.finish(running.id);
                }
                app.set_status(Status::Error(
                    "Database worker stopped unexpectedly".to_owned(),
                ));
                Redraw::Needed
            }
        }
    }
}

#[cfg(test)]
mod tests;
