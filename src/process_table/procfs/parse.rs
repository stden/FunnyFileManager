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

mod stat;
mod system;

use std::num::NonZeroU64;
use std::str::FromStr;
use std::time::Duration;

use crate::models::{MemoryBytes, MemoryUsage, Pid};

pub(super) use stat::{stat, Stat};
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
