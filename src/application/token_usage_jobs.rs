//! Purpose: Usage requests run off the event loop; generations reject late replies.
use super::Redraw;
use crate::models::token_usage::{Provider, ProviderUsage, UsageError, UsageState};
use crate::models::{App, Overlay};
use std::{
    sync::{mpsc, Arc},
    time::{Duration, Instant},
};

type Reports = Vec<ProviderUsage>;
struct Pending {
    generation: Arc<()>,
    receiver: mpsc::Receiver<Reports>,
}
#[derive(Default)]
pub(super) struct UsageJobs {
    pending: Option<Pending>,
    updated: Option<Instant>,
}
impl UsageJobs {
    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        match app.live.usage {
            crate::models::token_usage::UsageAccess::Network => {
                self.pump_with(app, crate::token_usage::collect)
            }
            crate::models::token_usage::UsageAccess::Offline => self.pump_with(app, disabled),
        }
    }
    pub(super) fn pump_with(
        &mut self,
        app: &mut App,
        fetch: impl FnOnce() -> Reports + Send + 'static,
    ) -> Redraw {
        let mut redraw = self.poll(app);
        let Overlay::TokenUsage(panel) = &mut app.overlay else {
            return redraw;
        };
        if self.pending.is_some() {
            return redraw;
        }
        let expired = self
            .updated
            .is_some_and(|time| time.elapsed() >= Duration::from_secs(60));
        if panel.state == UsageState::Queued || expired {
            let (sender, receiver) = mpsc::channel();
            let pending = Pending {
                generation: Arc::clone(&panel.generation),
                receiver,
            };
            panel.state = UsageState::Loading;
            let result = spawn(sender, fetch);
            if result.is_err() {
                crate::ops::token_usage::publish(panel, &failed());
            } else {
                self.pending = Some(pending);
            }
            self.updated = Some(Instant::now());
            redraw = Redraw::Needed;
        }
        redraw
    }
    fn poll(&mut self, app: &mut App) -> Redraw {
        let Some(pending) = &self.pending else {
            return Redraw::Unchanged;
        };
        let reports = match pending.receiver.try_recv() {
            Ok(reports) => reports,
            Err(mpsc::TryRecvError::Disconnected) => failed(),
            Err(mpsc::TryRecvError::Empty) => return Redraw::Unchanged,
        };
        if let Overlay::TokenUsage(panel) = &mut app.overlay {
            if Arc::ptr_eq(&panel.generation, &pending.generation) {
                crate::ops::token_usage::publish(panel, &reports);
            }
        }
        self.pending = None;
        self.updated = Some(Instant::now());
        Redraw::Needed
    }
}
fn failed() -> Reports {
    errors(UsageError::Worker)
}
fn disabled() -> Reports {
    errors(UsageError::Disabled)
}
fn errors(error: UsageError) -> Reports {
    Provider::ALL
        .into_iter()
        .map(|provider| ProviderUsage {
            provider,
            result: Err(error),
        })
        .collect()
}
#[cfg(test)]
mod tests;

fn spawn(
    sender: mpsc::Sender<Reports>,
    fetch: impl FnOnce() -> Reports + Send + 'static,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("ffm-token-usage".to_owned())
        .spawn(move || {
            drop(sender.send(fetch())); // Closing the panel may discard its result.
        })
}
