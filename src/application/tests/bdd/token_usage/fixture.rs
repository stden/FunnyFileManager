//! Purpose: Per-scenario response fixtures and a gate for deterministic late replies.

use super::replies::{Edition, Reply};
use crate::application::token_usage_jobs::UsageJobs;
use crate::models::token_usage::{Provider, ProviderUsage, UsageError};
use std::sync::mpsc::{self, Receiver, Sender};

enum Gate {
    Open,
    Armed {
        release: Sender<()>,
        wait: Receiver<()>,
    },
    Blocked(Sender<()>),
}

pub struct Fixture {
    pub(super) jobs: UsageJobs,
    reply: Reply,
    gate: Gate,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            jobs: UsageJobs::default(),
            reply: Reply::Success(Edition::Original),
            gate: Gate::Open,
        }
    }
}

impl Fixture {
    pub(super) fn failing(name: &str) -> Self {
        let error = match name {
            "access denied" => UsageError::AccessDenied,
            "missing key" => UsageError::MissingKey,
            "rate limit" => UsageError::RateLimited,
            "network error" => UsageError::Network,
            _ => panic!("unknown fixture error {name}"),
        };
        Self {
            reply: Reply::Failure(error),
            ..Self::default()
        }
    }

    pub(super) fn delayed() -> Self {
        let (release, wait) = mpsc::channel();
        Self {
            gate: Gate::Armed { release, wait },
            ..Self::default()
        }
    }

    pub(super) fn refresh(&mut self) {
        self.reply = Reply::Success(Edition::Refreshed);
    }

    pub(super) fn fetch(&mut self) -> impl FnOnce() -> Vec<ProviderUsage> + Send + 'static {
        let wait = match std::mem::replace(&mut self.gate, Gate::Open) {
            Gate::Armed { release, wait } => {
                self.gate = Gate::Blocked(release);
                Some(wait)
            }
            gate => {
                self.gate = gate;
                None
            }
        };
        let reply = self.reply;
        move || {
            if wait.is_some_and(|receiver| receiver.recv().is_err()) {
                return failed_worker();
            }
            super::replies::reports(reply)
        }
    }

    pub(super) fn release(&mut self) {
        let Gate::Blocked(sender) = std::mem::replace(&mut self.gate, Gate::Open) else {
            panic!("a delayed usage request must start before being released");
        };
        self.refresh();
        sender.send(()).expect("fixture worker is still waiting");
    }
}

fn failed_worker() -> Vec<ProviderUsage> {
    Provider::ALL
        .into_iter()
        .map(|provider| ProviderUsage {
            provider,
            result: Err(UsageError::Worker),
        })
        .collect()
}
