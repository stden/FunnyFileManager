//! Purpose: The task list (`Ctrl+W`): a live, htop-style view of every
//! process — the table as last sampled, what is shown and how, the cursor and
//! the marks, when the next sample is due, and what is open on top.
//!
//! State only: the worker that samples lives in `application`, the effects
//! of kill and renice in `ops`, the keys in `popup`, the drawing in `ui`.
//! Every transition is a method here, so no caller can leave the view out
//! of step with the table, the cursor off its process, or two dialogs open.
//!
//! No `Debug`: the rows hold command lines, which may hold secrets.

mod accessors;
mod errors;
mod header;
mod host;
mod identity;
mod meters;
mod modal;
mod notice;
mod priority;
mod renice;
mod sample;
mod schedule;
mod selection;
mod signal;
mod snapshot;
mod table;
mod targets;
mod tree;
mod units;
mod view;

use crate::prompt::Privilege;
use crate::scroll::ScrollLeft;

pub use errors::{ControlError, SampleError, SystemPart};
pub use header::{HeaderShape, BARS_PER_ROW};
pub use host::TaskHostSlot;
pub use identity::{Pid, ProcessKey, Start, StartTime, TaskSession};
pub use meters::{
    CoreGroup, CpuBar, CpuMeter, CpuMeters, MemoryMeter, Meters, SwapMeter, TaskCounts,
};
pub use modal::{
    ConfirmButton, FormFocus, Found, KillConfirm, MenuCursor, Modal, NiceConfirm, PidQuery,
    PriorityForm, ReniceButton, SearchBar, TaskMenu, TextBar,
};
pub use notice::{Attempt, Failure, Notice, Severity};
pub use priority::{
    KernelPriority, NiceOutOfRange, NiceStep, Niceness, Priority, PriorityChange, PriorityClass,
    SettableClass,
};
pub use renice::{NicePlan, PlannedChange, Renice};
pub use sample::{
    CoreSample, CpuCounters, MemorySample, MemoryUsage, Owner, ProcessHost, ProcessKind,
    ProcessReader, ProcessSample, ProcessState, Sample, StateLetter, SwapSample, SystemSample,
};
pub use schedule::{
    Arrival, Cadence, Health, Refresh, Request, Schedule, OVERDUE_AFTER, REFRESH_INTERVAL,
    STALL_LIMIT, WARM_UP,
};
pub use selection::{Cursor, ListMotion, SearchFrom};
pub use signal::{KillOffer, Signal, SignalEffect};
pub use snapshot::{Baselines, ProcessRow, Snapshot};
pub use table::Table;
pub use targets::{ControlRefusal, FrozenTargets, Operation, Relation, Skipped, Target};
pub use tree::{Sibling, TreeLine};
pub use units::{CpuUsage, Load, LoadAverage, MemoryBytes, Percent};
pub use view::{
    build_view, Arrangement, CommandView, FilterTerms, KernelThreads, Sort, SortDirection, SortKey,
    UserScope, ViewOptions, ViewRow,
};

use selection::Selection;

/// One opening of the task list.
#[derive(Clone)]
pub struct TaskList {
    session: TaskSession,
    /// FFM's own process, refused as a signal target; unknown to the default
    /// list, which has no host to signal through.
    own: Option<Pid>,
    /// Whether FFM runs as root, which decides whether `[` asks first.
    privilege: Privilege,
    table: Table,
    baselines: Baselines,
    options: ViewOptions,
    command_view: CommandView,
    /// The shown rows of the table, in order; rebuilt with every change.
    view: Vec<ViewRow>,
    selection: Selection,
    command_scroll: ScrollLeft,
    refresh: Refresh,
    modal: Modal,
    notice: Option<Notice>,
}

impl Default for TaskList {
    /// A list of session 0, which the host slot never mints, knowing no own
    /// process and assuming no root: what a placeholder or a test without a
    /// host holds.
    fn default() -> Self {
        Self::opened(TaskSession::new(0), None, Privilege::default())
    }
}

impl TaskList {
    /// The list for `session`, run by the process `own` with `privilege`: no
    /// table yet, a sample due at once.
    #[must_use]
    pub fn new(session: TaskSession, own: Pid, privilege: Privilege) -> Self {
        Self::opened(session, Some(own), privilege)
    }

    fn opened(session: TaskSession, own: Option<Pid>, privilege: Privilege) -> Self {
        Self {
            session,
            own,
            privilege,
            table: Table::Waiting,
            baselines: Baselines::default(),
            options: ViewOptions::default(),
            command_view: CommandView::default(),
            view: Vec::new(),
            selection: Selection::default(),
            command_scroll: ScrollLeft::LEFT,
            refresh: Refresh::default(),
            modal: Modal::Browse,
            notice: None,
        }
    }

    /// Rebuilds the shown rows from the table and the options, and finds the
    /// cursor's process in them.
    fn rebuild(&mut self) {
        self.view = build_view(self.rows(), &self.options);
        self.selection.follow(&self.view);
    }
}

#[cfg(test)]
mod tests_errors;
#[cfg(test)]
mod tests_header;
#[cfg(test)]
mod tests_host;
#[cfg(test)]
mod tests_identity;
#[cfg(test)]
mod tests_list;
#[cfg(test)]
mod tests_meters;
#[cfg(test)]
mod tests_modal;
#[cfg(test)]
mod tests_notice;
#[cfg(test)]
mod tests_priority;
#[cfg(test)]
mod tests_renice;
#[cfg(test)]
mod tests_sample;
#[cfg(test)]
mod tests_schedule;
#[cfg(test)]
mod tests_selection;
#[cfg(test)]
mod tests_signal;
#[cfg(test)]
mod tests_snapshot;
#[cfg(test)]
mod tests_support;
#[cfg(test)]
mod tests_targets;
#[cfg(test)]
mod tests_tree;
#[cfg(test)]
mod tests_units;
#[cfg(test)]
mod tests_view;
#[cfg(test)]
mod tests_view_priority;
