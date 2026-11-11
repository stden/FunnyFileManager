//! Purpose: The state of the Things catalog dialog (F9 Commands > Things
//! catalog): what it waits for, the machines it shows, and the filter, cursor
//! and marks set on them.

mod catalog;

use std::sync::Arc;

pub use catalog::ThingsCatalog;

use crate::things::Config;

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
    /// The read failed; the text says why.
    Failed(String),
}

/// The Things catalog dialog. `generation` tells the reply to a read apart from
/// the reply to a dialog that has since been closed or reopened.
#[derive(Clone)]
pub struct ThingsDialog {
    pub(crate) generation: Arc<()>,
    pub(crate) state: ThingsState,
}

impl ThingsDialog {
    #[must_use]
    pub fn new(state: ThingsState) -> Self {
        Self {
            generation: Arc::new(()),
            state,
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
}

#[cfg(test)]
mod tests;
