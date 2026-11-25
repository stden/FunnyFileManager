//! Purpose: Tests for the task list as a whole: applying samples and
//! failures, the warm-up, health marks, the view switches, a renice
//! recorded, and the viewport clamp `App` applies on resize.

use std::path::PathBuf;
use std::time::Instant;

use super::tests_support::{child_of, key, listing, one_core, pid, process, resample, shown, OWN};
use super::{
    Arrangement, CommandView, ControlRefusal, Cursor, Health, KernelThreads, ListMotion, Modal,
    Niceness, Notice, Owner, Priority, ProcessKind, ProcessSample, SampleError, Schedule, SortKey,
    SystemPart, Table, TaskList, TaskSession, UserScope, OVERDUE_AFTER, REFRESH_INTERVAL,
    STALL_LIMIT, WARM_UP,
};
use crate::models::{App, Overlay, Panel, SortOrder, Viewport};
use crate::prompt::Privilege;

mod tests_feedback;
mod tests_refresh;
mod tests_view;
