//! Purpose: When the task list samples next — live or held, a sample due, one
//! at a set time, one in flight, or none ever again — and how fresh the
//! table on screen is.
//!
//! The model only says what is due; the controller that owns the sampling
//! thread asks it on every loop pass, with the time passed in, so every
//! transition here is tested with synthetic instants. Applying a sample is
//! here too: whether a result is shown depends on the schedule it answers.

use std::time::{Duration, Instant};

use super::SampleError;

mod list;
mod refresh;

pub use refresh::Refresh;

/// Time between one sample completing and the next being taken: htop's
/// default of fifteen tenths.
pub const REFRESH_INTERVAL: Duration = Duration::from_millis(1_500);
/// The pause after a session's first snapshot, so the first CPU shares appear
/// quickly; at 100 clock ticks a second it still resolves 2 % per process.
pub const WARM_UP: Duration = Duration::from_millis(500);
/// A sample in flight this long leaves the table marked stale.
pub const OVERDUE_AFTER: Duration = Duration::from_secs(5);
/// A sample in flight this long is abandoned as not responding.
pub const STALL_LIMIT: Duration = Duration::from_secs(10);

/// Whether the table follows the system: `F11` and `Z` switch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Cadence {
    #[default]
    Live,
    Held,
}

/// Why a sample was asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request {
    /// The interval ran out.
    Automatic,
    /// The list opened, `Ctrl+R` was pressed, or a signal or renice finished:
    /// shown even while the table is held.
    Explicit,
}

/// What happens next.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Schedule {
    /// Sample as soon as the controller can: where a list that just opened
    /// starts.
    #[default]
    Due,
    /// Sample once this instant has come.
    At(Instant),
    /// A sample is in flight.
    Waiting { since: Instant, request: Request },
    /// A control made the sample in flight obsolete: discard it, then ask
    /// for one that can observe the control's result, even while held.
    Obsolete { since: Instant },
    /// Held: nothing until `F11`, `Z` or `Ctrl+R`.
    Held,
    /// The sampling worker is gone for this session; reopening the list
    /// starts a new one. Nothing is ever asked for again.
    Stopped,
}

/// Whether a result that arrived is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arrival {
    Apply,
    /// Held without an explicit request, or superseded by a control.
    Discard,
}

/// How fresh the table on screen is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Fresh,
    /// The sample in flight is late: ` · stale `.
    Overdue,
    /// The sample in flight was abandoned: ` · not responding `.
    Stalled,
    /// The last sample failed, and the table shows the one before it.
    Failed(SampleError),
}
