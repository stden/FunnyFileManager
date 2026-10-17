//! Purpose: Tests for the view: every sort key both ways with the PID
//! tie-break, the filter's terms, the user scope and the kernel-thread switch.

use std::time::Duration;

use super::tests_support::{counters, key, kib, measured, process, sample, snapshot, system};
use super::{
    build_view, Arrangement, CommandView, FilterTerms, KernelPriority, KernelThreads, MemoryUsage,
    Niceness, Owner, Priority, ProcessKind, ProcessSample, ProcessState, Snapshot, Sort,
    SortDirection, SortKey, TreeLine, UserScope, ViewOptions, ViewRow,
};
use crate::models::SortOrder;

mod tests_filter;
mod tests_sort;
mod tests_switches;

fn pids(snapshot: &Snapshot, view: &[ViewRow]) -> Vec<u32> {
    view.iter()
        .map(|shown| snapshot.rows()[shown.row()].sample().key.pid().get())
        .collect()
}

fn named(raw: u32, name: &str, command: &str) -> ProcessSample {
    ProcessSample {
        command: command.to_owned(),
        ..process(raw, name)
    }
}

fn browsers() -> Snapshot {
    snapshot(vec![
        named(1, "firefox", "/usr/bin/firefox"),
        named(2, "chromium", "/usr/bin/chromium --incognito"),
        named(3, "python3", "python3 /srv/x/train.py"),
    ])
}
