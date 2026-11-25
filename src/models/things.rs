//! Purpose: The state of the Things catalog dialog (F9 Commands > Things
//! catalog): what it waits for, the machines it shows, and the filter, cursor
//! and marks set on them.

mod catalog;

use std::sync::Arc;

pub use catalog::ThingsCatalog;

use crate::things::{Config, PatchPlan};

/// Whether this program may read the catalog. Only live input may: a test or a
/// replayed session reads nothing.
#[derive(Clone, Copy, Default)]
pub enum ThingsAccess {
    #[default]
    Offline,
    Network,
}

/// Where the dialog is.
#[derive(Clone)]
pub enum ThingsState {
    /// The settings are missing or unusable; the text says which.
    Setup(String),
    /// The settings are good; the read starts on the next turn of the loop.
    Queued(Config),
    /// The read is out.
    Loading,
    /// The machines, with the filter, cursor and marks set on them.
    Ready(ThingsCatalog),
    /// Waiting for the user to confirm starting patch workflows.
    ConfirmPatch {
        catalog: ThingsCatalog,
        plan: PatchPlan,
    },
    /// The patch run is queued to start on the worker thread.
    PatchQueued { config: Config, plan: PatchPlan },
    /// Patch workflow requests are in flight.
    Patching,
    /// The read or patch failed; the text says why.
    Failed(String),
}

/// The Things catalog dialog. `generation` tells the reply to a read apart from
/// the reply to a dialog that has since been closed or reopened.
#[derive(Clone)]
pub struct ThingsDialog {
    pub(crate) generation: Arc<()>,
    pub(crate) state: ThingsState,
    pub(crate) config: Option<Config>,
}

impl ThingsDialog {
    #[must_use]
    pub fn new(state: ThingsState) -> Self {
        Self {
            generation: Arc::new(()),
            state,
            config: None,
        }
    }

    #[must_use]
    pub fn with_config(state: ThingsState, config: Config) -> Self {
        Self {
            generation: Arc::new(()),
            state,
            config: Some(config),
        }
    }

    /// The settings of a read that is waiting, which marks the dialog as
    /// loading; `None` when no read is waiting.
    pub fn start(&mut self) -> Option<Config> {
        match std::mem::replace(&mut self.state, ThingsState::Loading) {
            ThingsState::Queued(config) => Some(config),
            other => {
                self.state = other;
                None
            }
        }
    }

    /// The settings and plan of a queued patch run, marking the dialog as patching.
    pub fn start_patch(&mut self) -> Option<(Config, PatchPlan)> {
        match std::mem::replace(&mut self.state, ThingsState::Patching) {
            ThingsState::PatchQueued { config, plan } => Some((config, plan)),
            other => {
                self.state = other;
                None
            }
        }
    }

    /// Whether the dialog is waiting on a background read or patch request.
    #[must_use]
    pub const fn is_busy(&self) -> bool {
        matches!(
            self.state,
            ThingsState::Queued(_)
                | ThingsState::Loading
                | ThingsState::PatchQueued { .. }
                | ThingsState::Patching
        )
    }
}

#[cfg(test)]
mod tests;
