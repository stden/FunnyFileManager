//! Purpose: Cloudflare DNS requests run off the event loop. The answer to the dialog's latest request
//! is applied while that same dialog is open; any other answer is dropped.

use super::Redraw;
use crate::models::cloudflare_dns::{
    CloudflareAccess, CloudflareDnsPanel, CloudflareError, Reply, Request, RequestId,
};
use crate::models::{App, Overlay};
use std::sync::{mpsc, Arc};

/// The one request in flight, and what its answer has to match to be applied.
struct Pending {
    generation: Arc<()>,
    id: RequestId,
    request: Request,
    receiver: mpsc::Receiver<Reply>,
}

#[derive(Default)]
pub(super) struct CloudflareJobs {
    pending: Option<Pending>,
}

impl CloudflareJobs {
    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        match app.live.cloudflare {
            CloudflareAccess::Network => self.pump_with(app, crate::cloudflare_dns::fetch),
            CloudflareAccess::Offline => self.pump_with(app, crate::cloudflare_dns::unavailable),
        }
    }

    /// Applies a finished answer, then starts the queued request when nothing is in flight.
    pub(super) fn pump_with(
        &mut self,
        app: &mut App,
        fetch: impl FnOnce(Request) -> Reply + Send + 'static,
    ) -> Redraw {
        let polled = self.poll(app);
        let Overlay::CloudflareDns(panel) = &mut app.overlay else {
            return polled;
        };
        if self.pending.is_some() {
            return polled;
        }
        let Some((id, request)) = panel.take_queued() else {
            return polled;
        };
        self.start(panel, id, request, fetch);
        Redraw::Needed
    }

    fn start(
        &mut self,
        panel: &mut CloudflareDnsPanel,
        id: RequestId,
        request: Request,
        fetch: impl FnOnce(Request) -> Reply + Send + 'static,
    ) {
        let (sender, receiver) = mpsc::channel();
        let pending = Pending {
            generation: Arc::clone(&panel.generation),
            id,
            request: request.clone(),
            receiver,
        };
        if spawn(sender, request, fetch).is_ok() {
            self.pending = Some(pending);
        } else {
            let failure = pending.request.fail(CloudflareError::Worker);
            crate::ops::cloudflare_dns::publish(panel, failure);
        }
    }

    fn poll(&mut self, app: &mut App) -> Redraw {
        let Some(pending) = self.pending.take() else {
            return Redraw::Unchanged;
        };
        let reply = match pending.receiver.try_recv() {
            Ok(reply) => reply,
            Err(mpsc::TryRecvError::Empty) => {
                self.pending = Some(pending);
                return Redraw::Unchanged;
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                pending.request.clone().fail(CloudflareError::Worker)
            }
        };
        if let Overlay::CloudflareDns(panel) = &mut app.overlay {
            if Arc::ptr_eq(&panel.generation, &pending.generation) && panel.awaits(pending.id) {
                crate::ops::cloudflare_dns::publish(panel, reply);
            }
        }
        Redraw::Needed
    }
}

fn spawn(
    sender: mpsc::Sender<Reply>,
    request: Request,
    fetch: impl FnOnce(Request) -> Reply + Send + 'static,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    #[cfg(test)]
    if REFUSE_SPAWN.with(std::cell::Cell::take) {
        return Err(std::io::Error::other("the thread was refused"));
    }
    std::thread::Builder::new()
        .name("ffm-cloudflare-dns".to_owned())
        .spawn(move || {
            drop(sender.send(fetch(request))); // Closing the dialog may discard its answer.
        })
}

#[cfg(test)]
thread_local! {
    pub(super) static REFUSE_SPAWN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
mod tests;
