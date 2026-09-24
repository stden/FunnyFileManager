//! Purpose: The task list's values as text — shares, CPU time, uptime, load,
//! task counts, sizes and the meter captions — as pure functions, so each
//! format is pinned to the character at every unit boundary.
//!
//! Integer arithmetic only: a value is divided down, never converted to a
//! float, so no rounding mode or overflow depends on the platform.

use std::borrow::Cow;
use std::time::Duration;

use crate::models::{
    CpuMeter, CpuUsage, Load, LoadAverage, MemoryBytes, Owner, Percent, Priority, TaskCounts,
};

use super::super::panel::abbreviate_bytes;
use super::super::printable_name;

/// What a value the platform did not give shows as.
pub(super) const UNKNOWN: &str = "-";

const MINUTE: u64 = 60;
const HOUR: u64 = 60 * MINUTE;
const DAY: u64 = 24 * HOUR;
/// CPU time from this many hours on is counted in days.
const DAYS_FROM: u64 = 100 * HOUR;

/// The kernel priority a realtime process reports: `RT` in the PRI column.
const REALTIME_LABEL: &str = "RT";

/// A share, as CPU% and MEM% print it: one decimal below 100 (`38.4`,
/// `0.0`), a whole number from 100 on (`3200`). Rounded half up, before the
/// switch, so `99.96` reads `100` rather than `100.0`.
pub(super) fn percent(share: Percent) -> String {
    let hundredths = u64::from(share.hundredths());
    let tenths = (hundredths + 5) / 10;
    if tenths < 1_000 {
        format!("{}.{}", tenths / 10, tenths % 10)
    } else {
        ((hundredths + 50) / 100).to_string()
    }
}

/// CPU%: the share, or `-` until two samples measure it.
pub(super) fn cpu_usage(usage: CpuUsage) -> String {
    match usage {
        CpuUsage::Pending => UNKNOWN.to_owned(),
        CpuUsage::Measured(share) => percent(share),
    }
}

/// MEM%: the share, or `-` when the platform gave no sizes.
pub(super) fn memory_share(share: Option<Percent>) -> String {
    share.map_or_else(|| UNKNOWN.to_owned(), percent)
}

/// TIME+: `M:SS.hh` under an hour, `HhMM:SS` under a hundred hours, and
/// `DdHHh` from there on.
pub(super) fn cpu_time(time: Duration) -> String {
    let seconds = time.as_secs();
    if seconds < HOUR {
        let hundredths = time.subsec_millis() / 10;
        format!(
            "{}:{:02}.{hundredths:02}",
            seconds / MINUTE,
            seconds % MINUTE
        )
    } else if seconds < DAYS_FROM {
        format!(
            "{}h{:02}:{:02}",
            seconds / HOUR,
            seconds % HOUR / MINUTE,
            seconds % MINUTE
        )
    } else {
        format!("{}d{:02}h", seconds / DAY, seconds % DAY / HOUR)
    }
}

/// A span of wall-clock time: `05:52:06`, `1 day, 05:52:06`,
/// `12 days, 05:52:06`.
pub(super) fn span(time: Duration) -> String {
    let seconds = time.as_secs();
    let clock = format!(
        "{:02}:{:02}:{:02}",
        seconds % DAY / HOUR,
        seconds % HOUR / MINUTE,
        seconds % MINUTE
    );
    match seconds / DAY {
        0 => clock,
        1 => format!("1 day, {clock}"),
        days => format!("{days} days, {clock}"),
    }
}

/// The header's `Uptime:` field.
pub(super) fn uptime(time: Duration) -> String {
    format!("Uptime: {}", span(time))
}

/// The header's `Load average:` field.
pub(super) fn load(average: LoadAverage) -> String {
    format!(
        "Load average: {} {} {}",
        hundredths(average.one()),
        hundredths(average.five()),
        hundredths(average.fifteen())
    )
}

fn hundredths(load: Load) -> String {
    let value = load.hundredths();
    format!("{}.{:02}", value / 100, value % 100)
}

/// The header's `Tasks:` field; the runnable count only when it is known.
pub(super) fn tasks(counts: TaskCounts) -> String {
    let listed = format!(
        "Tasks: {}, {} thr, {} kthr",
        counts.processes(),
        counts.threads(),
        counts.kernel_threads()
    );
    match counts.running() {
        Some(running) => format!("{listed}; {running} running"),
        None => listed,
    }
}

/// A size as the panels print one: `381 K`, `22.4 M`, `3.1 G`.
pub(super) fn bytes(size: MemoryBytes) -> String {
    abbreviate_bytes(size.bytes())
}

/// A size column's cell: the size, or `-` where the platform gave none.
pub(super) fn size(size: Option<MemoryBytes>) -> String {
    size.map_or_else(|| UNKNOWN.to_owned(), bytes)
}

/// A memory or swap meter's caption: `38.2 G/125 G`.
pub(super) fn used_of(used: MemoryBytes, total: MemoryBytes) -> String {
    format!("{}/{}", bytes(used), bytes(total))
}

/// A CPU meter's caption: `34.2%`, or `-` until two samples measure it.
pub(super) fn cpu_meter(meter: CpuMeter) -> String {
    match meter {
        CpuMeter::Pending => UNKNOWN.to_owned(),
        CpuMeter::Measured(bar) => format!("{}%", percent(bar.busy())),
    }
}

/// The PRI and NI cells: the kernel's priority (`RT` for realtime) and the
/// nice value; `-` for both where the platform reports neither.
pub(super) fn priority(priority: Priority) -> (String, String) {
    match priority {
        Priority::Unix { kernel, nice } => {
            let pri = if kernel.is_realtime() {
                REALTIME_LABEL.to_owned()
            } else {
                kernel.get().to_string()
            };
            (pri, nice.get().to_string())
        }
        Priority::Windows { .. } | Priority::Unknown => (UNKNOWN.to_owned(), UNKNOWN.to_owned()),
    }
}

/// An owner as the User column and the footer name one: the user name,
/// made safe to draw; the bare uid when it has no name; `-` when unknown.
pub(super) fn owner(owner: &Owner) -> Cow<'_, str> {
    match owner {
        Owner::Named(name) => printable_name(name),
        Owner::Uid(uid) => Cow::Owned(uid.to_string()),
        Owner::Unknown => Cow::Borrowed(UNKNOWN),
    }
}
