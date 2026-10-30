//! Purpose: The task list (`Ctrl+W`): a live view of processes and network
//! adapters, cursor/marks, sampling schedule, and modals.
//!
//! State only: the worker that samples lives in `application`, the effects
//! of kill and renice in `ops`, the keys in `popup`, the drawing in `ui`.
//!
//! No `Debug`: the rows hold command lines, which may hold secrets.

mod accessors;
mod errors;
mod header;
mod host;
mod identity;
mod meters;
mod modal;
mod network;
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
pub use network::{
    AdapterFilter, AdapterKind, AdapterRow, AdapterSample, BytesPerSec, InterfaceState, Ipv4,
    NetDiagnostics, NetHealth, NetMeter, NetworkSample, ProbeOutcome, ProbeRow, ProbeTarget,
    TaskTab, PROBE_INTERVAL,
};
pub use notice::{Attempt, Failure, Notice, Severity};
pub use priority::{
    KernelPriority, NiceOutOfRange, NiceStep, Niceness, Priority, PriorityChange, PriorityClass,
    SettableClass,
};
pub use renice::{NicePlan, PlannedChange, Renice};
pub use sample::{
    CoreSample, CpuCounters, MemorySample, MemoryUsage, NetProber, Owner, ProcessHost, ProcessKind,
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

use network::NetworkView;
use selection::Selection;

/// One opening of the task list.
#[derive(Clone)]
pub struct TaskList {
    session: TaskSession,
    own: Option<Pid>,
    privilege: Privilege,
    table: Table,
    baselines: Baselines,
    options: ViewOptions,
    command_view: CommandView,
    view: Vec<ViewRow>,
    selection: Selection,
    command_scroll: ScrollLeft,
    refresh: Refresh,
    network: NetworkView,
    modal: Modal,
    notice: Option<Notice>,
}

impl Default for TaskList {
    fn default() -> Self {
        Self::opened(TaskSession::new(0), None, Privilege::default())
    }
}

impl TaskList {
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
            network: NetworkView::default(),
            modal: Modal::Browse,
            notice: None,
        }
    }

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
mod tests_network;
#[cfg(test)]
mod tests_network_probes;
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
