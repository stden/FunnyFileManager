//! One serial WSL worker; App and rendering contain no threads or receivers.

use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

use super::Redraw;
use crate::models::{App, WslOutput, WslRequest, WslRequestId};
use crate::ops::wsl::complete;
use crate::wsl::WslBackend;

type Completion = (WslRequestId, Result<WslOutput, String>);

struct Worker {
    requests: mpsc::Sender<WslRequest>,
    results: mpsc::Receiver<Completion>,
    pending: Vec<Pending>,
}

struct Pending {
    id: WslRequestId,
    action: &'static str,
    started: Instant,
}

impl Pending {
    fn new(request: &WslRequest) -> Self {
        use crate::models::WslOperation;
        let action = match request.operation {
            WslOperation::Refresh => "Background/WSL/Refresh",
            WslOperation::Update => "Background/WSL/Update",
            WslOperation::UpdatePackages(_) => "Background/WSL/UpdatePackages",
            WslOperation::Open(_) => "Background/WSL/Open",
        };
        Self {
            id: request.id,
            action,
            started: Instant::now(),
        }
    }

    fn finish(self, app: &mut App) {
        app.performance
            .record(self.action, self.started.elapsed(), Duration::ZERO);
    }
}

pub(super) struct WslJobs {
    backend: Arc<dyn WslBackend>,
    worker: Option<Worker>,
}

impl WslJobs {
    pub(super) fn new(backend: Arc<dyn WslBackend>) -> Self {
        Self {
            backend,
            worker: None,
        }
    }

    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        let mut redraw = Redraw::Unchanged;
        for request in app.wsl_requests.take() {
            let id = request.id;
            let builder = std::thread::Builder::new().name("ffm-wsl".to_owned());
            if let Err(error) = self.submit_request(request, builder) {
                complete(app, id, Err(error));
                redraw = Redraw::Needed;
            }
        }
        let Some(worker) = &mut self.worker else {
            return redraw;
        };
        if worker.receive(app, &mut redraw) == Connection::Stopped {
            worker.fail_pending(app, &mut redraw);
            self.worker = None;
        }
        redraw
    }

    fn submit_request(
        &mut self,
        request: WslRequest,
        builder: std::thread::Builder,
    ) -> Result<(), String> {
        let mut worker = match self.worker.take() {
            Some(worker) => worker,
            None => spawn_with(Arc::clone(&self.backend), builder)
                .map_err(|error| format!("Cannot start WSL worker: {error}"))?,
        };
        let result = worker.submit(request);
        self.worker = Some(worker);
        result
    }
}

#[derive(PartialEq, Eq)]
enum Connection {
    Active,
    Stopped,
}

impl Worker {
    #[expect(
        clippy::map_err_ignore,
        reason = "SendError only hands back the unsent request; a stopped worker is the whole cause"
    )]
    fn submit(&mut self, request: WslRequest) -> Result<(), String> {
        let pending = Pending::new(&request);
        self.requests
            .send(request)
            .map_err(|_| "WSL worker stopped".to_owned())?;
        self.pending.push(pending);
        Ok(())
    }

    fn receive(&mut self, app: &mut App, redraw: &mut Redraw) -> Connection {
        loop {
            match self.results.try_recv() {
                Ok((id, result)) => {
                    self.finish(app, id);
                    complete(app, id, result);
                    *redraw = Redraw::Needed;
                }
                Err(mpsc::TryRecvError::Empty) => return Connection::Active,
                Err(mpsc::TryRecvError::Disconnected) => return Connection::Stopped,
            }
        }
    }

    fn fail_pending(&mut self, app: &mut App, redraw: &mut Redraw) {
        for pending in self.pending.drain(..) {
            let id = pending.id;
            pending.finish(app);
            complete(app, id, Err("WSL worker stopped".to_owned()));
            *redraw = Redraw::Needed;
        }
    }

    fn finish(&mut self, app: &mut App, id: WslRequestId) {
        if let Some(index) = self.pending.iter().position(|pending| pending.id == id) {
            self.pending.remove(index).finish(app);
        }
    }
}

#[cfg(test)]
fn spawn(backend: Arc<dyn WslBackend>) -> std::io::Result<Worker> {
    spawn_with(
        backend,
        std::thread::Builder::new().name("ffm-wsl".to_owned()),
    )
}

fn spawn_with(
    backend: Arc<dyn WslBackend>,
    builder: std::thread::Builder,
) -> std::io::Result<Worker> {
    let (requests, incoming) = mpsc::channel::<WslRequest>();
    let (outgoing, results) = mpsc::channel();
    builder.spawn(move || {
        while let Ok(request) = incoming.recv() {
            let result = backend.execute(&request.operation);
            if outgoing.send((request.id, result)).is_err() {
                break;
            }
        }
    })?;
    Ok(Worker {
        requests,
        results,
        pending: Vec::new(),
    })
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_abandoned;
