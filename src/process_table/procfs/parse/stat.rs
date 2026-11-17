//! Purpose: The parser of the `stat` line `/proc` keeps per process: its
//! name however it is spelled, and the fields through the 23rd the table
//! reads — state, parent, kind, CPU time, priority, threads and start.

use std::str::FromStr;
use std::time::Duration;

use crate::models::{
    KernelPriority, Niceness, Pid, Priority, ProcessKind, ProcessState, StartTime,
};

use super::{unsigned, ClockTicks};

/// `PF_KTHREAD`, the `stat` flag of a kernel thread (`include/linux/sched.h`).
const PF_KTHREAD: u64 = 0x0020_0000;

/// What one `stat` line says about a process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::process_table::procfs) struct Stat {
    /// The kernel's short name (`comm`), untrusted text.
    pub(in crate::process_table::procfs) name: String,
    pub(in crate::process_table::procfs) state: ProcessState,
    pub(in crate::process_table::procfs) parent: Option<Pid>,
    pub(in crate::process_table::procfs) kind: ProcessKind,
    /// User plus system time.
    pub(in crate::process_table::procfs) cpu_time: Duration,
    pub(in crate::process_table::procfs) priority: Priority,
    pub(in crate::process_table::procfs) threads: u32,
    /// Field 22, as the identity stamp.
    pub(in crate::process_table::procfs) start: StartTime,
    /// Field 22, as the time after boot the process started.
    pub(in crate::process_table::procfs) started: Duration,
}

/// The fields of a `stat` line after the name, numbered as proc(5) numbers
/// them: field `k` is the `k - 3`-th token after the closing parenthesis.
struct StatFields<'a>(Vec<&'a str>);

impl StatFields<'_> {
    /// Tokens a line must have: through field 23, `vsize`.
    const REQUIRED: usize = 21;
    /// The number of the first field after the name, the state.
    const FIRST: usize = 3;

    fn text(&self, field: usize) -> Option<&str> {
        self.0.get(field.checked_sub(Self::FIRST)?).copied()
    }

    fn unsigned<T: FromStr>(&self, field: usize) -> Option<T> {
        unsigned(self.text(field)?)
    }

    fn signed<T: FromStr>(&self, field: usize) -> Option<T> {
        self.text(field)?.parse().ok()
    }
}

/// The `stat` line of the process in directory `expected`, or `None` when it
/// is cut short, malformed, or about another process.
///
/// The name may hold spaces and parentheses (`((sd-pam))`, `next-server (v1`),
/// so the line is split at the *last* closing parenthesis.
pub(in crate::process_table::procfs) fn stat(
    text: &str,
    expected: Pid,
    ticks: ClockTicks,
) -> Option<Stat> {
    let (head, tail) = text.rsplit_once(')')?;
    let (pid, name) = head.split_once(" (")?;
    if Pid::new(unsigned(pid)?)? != expected {
        return None;
    }
    let fields = StatFields(tail.split_ascii_whitespace().collect());
    if fields.0.len() < StatFields::REQUIRED {
        return None;
    }
    let (utime, stime): (u64, u64) = (fields.unsigned(14)?, fields.unsigned(15)?);
    let start_ticks: u64 = fields.unsigned(22)?;
    let parent = match fields.unsigned(4)? {
        0 => None,
        raw => Some(Pid::new(raw)?),
    };
    Some(Stat {
        name: name.to_owned(),
        state: state(fields.text(3)?)?,
        parent,
        kind: kind(fields.unsigned(9)?),
        cpu_time: ticks.duration(utime.saturating_add(stime)),
        priority: priority(&fields)?,
        threads: fields.unsigned(20)?,
        start: StartTime::new(start_ticks),
        started: ticks.duration(start_ticks),
    })
}

/// The kernel's priority and the nice value; a nice value outside
/// -20..=19 makes the line malformed.
fn priority(fields: &StatFields<'_>) -> Option<Priority> {
    Some(Priority::Unix {
        kernel: KernelPriority::new(fields.signed(18)?),
        nice: Niceness::try_from(fields.signed::<i32>(19)?).ok()?,
    })
}

/// The state field: exactly one byte.
fn state(field: &str) -> Option<ProcessState> {
    match field.as_bytes() {
        [letter] => Some(ProcessState::from_letter(*letter)),
        _ => None,
    }
}

const fn kind(flags: u64) -> ProcessKind {
    if flags & PF_KTHREAD == 0 {
        ProcessKind::User
    } else {
        ProcessKind::KernelThread
    }
}
