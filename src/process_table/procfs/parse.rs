//! Purpose: Pure parsers for the files `/proc` keeps per process — `stat`,
//! `statm`, `status` and `cmdline` — and the two conversion factors they
//! need: clock ticks per second and bytes per page.
//!
//! Every parser takes text already read (and cut at the reader's cap) and
//! decoded lossily, and answers `None` rather than guessing: a name chosen to
//! confuse or a number too large to hold never becomes a plausible wrong
//! value, and nothing panics. A number cut short is never read as a smaller
//! one: the kernel ends every line it writes with a newline, so the
//! line-oriented parsers read only up to the last one (`complete_lines`);
//! `stat` needs its fields through the 23rd instead, and `cmdline`, which
//! has no lines, is shown as far as it was read. They split with `split_once`
//! and `split_ascii_whitespace`, reach fields with `get`, and do checked or
//! saturating arithmetic only.

mod system;

use std::num::NonZeroU64;
use std::str::FromStr;
use std::time::Duration;

use crate::models::{
    KernelPriority, MemoryBytes, MemoryUsage, Niceness, Pid, Priority, ProcessKind, ProcessState,
    StartTime,
};

pub(super) use system::{cpu_times, load_average, memory_info, uptime};

/// Clock ticks per second (`USER_HZ`), the unit of every CPU time in `/proc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ClockTicks(NonZeroU64);

impl ClockTicks {
    /// The rate the platform reports, or `None` for zero, which no kernel
    /// reports and no time could be divided by.
    pub(super) fn new(per_second: u64) -> Option<Self> {
        NonZeroU64::new(per_second).map(Self)
    }

    /// `ticks` as a duration, exact to the nanosecond and never overflowing:
    /// the whole seconds and the ticks left over are converted apart.
    pub(super) fn duration(self, ticks: u64) -> Duration {
        let per_second = self.0.get();
        let seconds = Duration::from_secs(ticks / per_second);
        let left = u128::from(ticks % per_second) * NANOS_PER_SECOND / u128::from(per_second);
        // `left` is below a second's worth of nanoseconds, so it always fits.
        seconds.saturating_add(Duration::from_nanos(u64::try_from(left).unwrap_or(0)))
    }
}

const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// Bytes per memory page, the unit of `statm`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PageSize(NonZeroU64);

impl PageSize {
    /// The size the platform reports, or `None` for zero.
    pub(super) fn new(bytes: u64) -> Option<Self> {
        NonZeroU64::new(bytes).map(Self)
    }

    /// `pages` in bytes, or `None` when that does not fit.
    fn bytes(self, pages: u64) -> Option<MemoryBytes> {
        MemoryBytes::from_pages(pages, self.0.get())
    }
}

/// `PF_KTHREAD`, the `stat` flag of a kernel thread (`include/linux/sched.h`).
const PF_KTHREAD: u64 = 0x0020_0000;

/// What one `stat` line says about a process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Stat {
    /// The kernel's short name (`comm`), untrusted text.
    pub(super) name: String,
    pub(super) state: ProcessState,
    pub(super) parent: Option<Pid>,
    pub(super) kind: ProcessKind,
    /// User plus system time.
    pub(super) cpu_time: Duration,
    pub(super) priority: Priority,
    pub(super) threads: u32,
    /// Field 22, as the identity stamp.
    pub(super) start: StartTime,
    /// Field 22, as the time after boot the process started.
    pub(super) started: Duration,
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
pub(super) fn stat(text: &str, expected: Pid, ticks: ClockTicks) -> Option<Stat> {
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

/// A `statm` line: size, resident and shared memory, in pages. `None` when a
/// field is missing or a size does not fit in bytes.
pub(super) fn statm(text: &str, page: PageSize) -> Option<MemoryUsage> {
    let mut fields = complete_lines(text).split_ascii_whitespace();
    let mut next = || page.bytes(unsigned(fields.next()?)?);
    Some(MemoryUsage {
        virtual_size: next()?,
        resident: next()?,
        shared: Some(next()?),
    })
}

/// The effective uid in a `status` file — the second number of its `Uid:`
/// line — which, unlike the owner of the `/proc` directory, is the process's
/// own even when it is not dumpable.
pub(super) fn effective_uid(text: &str) -> Option<u32> {
    let line = complete_lines(text)
        .lines()
        .find_map(|line| line.strip_prefix("Uid:"))?;
    unsigned(line.split_ascii_whitespace().nth(1)?)
}

/// A `cmdline` file as one line: the arguments joined with spaces, or
/// `[name]`, as `ps` prints them, for a process with none (a kernel thread,
/// a zombie).
pub(super) fn command_line(text: &str, name: &str) -> String {
    let arguments = text.strip_suffix('\0').unwrap_or(text);
    let joined = arguments.replace('\0', " ");
    if joined.trim().is_empty() {
        format!("[{name}]")
    } else {
        joined
    }
}

/// The PID a `/proc` entry is named by, or `None` for every other entry
/// (`self`, `thread-self`, `sys`, `net`, …): decimal digits only, so no sign
/// or blank `parse` would forgive turns a name into a PID.
pub(super) fn directory_pid(name: &str) -> Option<Pid> {
    Pid::new(unsigned(name)?)
}

/// The lines of `text` the kernel finished writing: everything up to its
/// last newline. What follows was cut off by the reader's cap.
fn complete_lines(text: &str) -> &str {
    text.rfind('\n')
        .and_then(|end| text.get(..=end))
        .unwrap_or_default()
}

/// A number written in decimal digits only: no sign, no blank, nothing
/// `parse` would forgive.
fn unsigned<T: FromStr>(text: &str) -> Option<T> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}
