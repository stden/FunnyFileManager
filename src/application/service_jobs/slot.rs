//! Purpose: Worker slot primitive and thread spawn helper for off-UI service jobs.

use std::sync::{mpsc, Arc};
use std::thread;

use super::super::Redraw;
use crate::models::App;

#[cfg(test)]
thread_local! {
    pub(super) static FORCE_SPAWN_FAILURE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(super) fn spawn_thread<F: FnOnce() + Send + 'static>(
    name: &'static str,
    f: F,
) -> std::io::Result<thread::JoinHandle<()>> {
    #[cfg(test)]
    if FORCE_SPAWN_FAILURE.get() {
        return Err(std::io::Error::other("forced spawn failure"));
    }
    thread::Builder::new().name(name.into()).spawn(f)
}

pub(super) struct Pending<Id, Rep> {
    pub(super) generation: Arc<()>,
    pub(super) id: Id,
    pub(super) fallback: Rep,
    pub(super) receiver: mpsc::Receiver<Rep>,
}

pub(super) struct WorkerSlot<Id, Rep> {
    pub(super) pending: Option<Pending<Id, Rep>>,
}

impl<Id, Rep> Default for WorkerSlot<Id, Rep> {
    fn default() -> Self {
        Self { pending: None }
    }
}

pub(super) type SlotOps<Id, Req, Rep> = (
    fn(&mut App) -> Option<(Arc<()>, Id, Req)>,
    fn(&Req) -> Rep,
    fn(&mut App, &Arc<()>, Id, Rep),
);

impl<Id: Copy + Send + 'static, Rep: Send + 'static> WorkerSlot<Id, Rep> {
    pub(super) fn poll(
        &mut self,
        app: &mut App,
        apply: impl FnOnce(&mut App, &Arc<()>, Id, Rep),
    ) -> Redraw {
        let Some(pending) = self.pending.take() else {
            return Redraw::Unchanged;
        };
        let reply = match pending.receiver.try_recv() {
            Ok(reply) => reply,
            Err(mpsc::TryRecvError::Empty) => {
                self.pending = Some(pending);
                return Redraw::Unchanged;
            }
            Err(mpsc::TryRecvError::Disconnected) => pending.fallback,
        };
        apply(app, &pending.generation, pending.id, reply);
        Redraw::Needed
    }

    pub(super) fn pump_with<Req: Send + 'static>(
        &mut self,
        app: &mut App,
        name: &'static str,
        ops: SlotOps<Id, Req, Rep>,
        work: impl FnOnce(Req) -> Rep + Send + 'static,
    ) -> Redraw {
        let (take, fallback_fn, apply) = ops;
        let polled = self.poll(app, apply);
        if self.pending.is_some() {
            return polled;
        }
        let Some((generation, id, request)) = take(app) else {
            return polled;
        };
        let fallback = fallback_fn(&request);
        if let Err((g, i, rep)) = self.spawn_job(name, (generation, id, request, fallback), work) {
            apply(app, &g, i, rep);
        }
        Redraw::Needed
    }

    fn spawn_job<Req: Send + 'static>(
        &mut self,
        name: &'static str,
        task: (Arc<()>, Id, Req, Rep),
        work: impl FnOnce(Req) -> Rep + Send + 'static,
    ) -> Result<(), (Arc<()>, Id, Rep)> {
        let (generation, id, request, fallback) = task;
        let (sender, receiver) = mpsc::channel();
        match spawn_thread(name, move || drop(sender.send(work(request)))) {
            Ok(_) => {
                self.pending = Some(Pending {
                    generation,
                    id,
                    fallback,
                    receiver,
                });
                Ok(())
            }
            Err(_) => Err((generation, id, fallback)),
        }
    }
}
