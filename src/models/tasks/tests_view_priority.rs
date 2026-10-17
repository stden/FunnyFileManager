//! Purpose: Tests for how the Priority sort key ranks processes: the kernel's
//! priority before nice, a Windows class after every Unix priority in the
//! order the classes run, and an unknown priority last.

use super::tests_support::{process, snapshot};
use super::{
    build_view, KernelPriority, Niceness, Priority, PriorityClass, ProcessSample, Sort, SortKey,
    ViewOptions,
};
use crate::models::SortOrder;

fn with(raw: u32, priority: Priority) -> ProcessSample {
    ProcessSample {
        priority,
        ..process(raw, "worker")
    }
}

fn unix(kernel: i32, nice: i32) -> Priority {
    Priority::Unix {
        kernel: KernelPriority::new(kernel),
        nice: Niceness::try_from(nice).expect("a fixture nice value"),
    }
}

const fn windows(class: PriorityClass) -> Priority {
    Priority::Windows { class }
}

#[test]
fn priority_sorts_what_runs_first_first_and_the_unknown_last() {
    // PIDs run against the expected order wherever a wrong rank could hide
    // behind the PID tie-break.
    let snapshot = snapshot(vec![
        with(5, Priority::Unknown),
        with(41, windows(PriorityClass::Idle)),
        with(42, windows(PriorityClass::BelowNormal)),
        with(43, windows(PriorityClass::Normal)),
        with(44, windows(PriorityClass::AboveNormal)),
        with(45, windows(PriorityClass::High)),
        with(46, windows(PriorityClass::Realtime)),
        with(30, unix(20, 0)),
        with(20, unix(0, -20)),
        with(12, unix(-51, 0)),
        with(15, unix(-51, -5)),
        with(10, unix(-100, 0)),
    ]);
    let options = ViewOptions {
        sort: Sort::new(SortKey::Priority, SortOrder::Direct),
        ..ViewOptions::default()
    };
    let order: Vec<u32> = build_view(snapshot.rows(), &options)
        .iter()
        .map(|shown| shown.key().pid().get())
        .collect();
    assert_eq!(
        order,
        [10, 15, 12, 20, 30, 46, 45, 44, 43, 42, 41, 5],
        "realtime before nice -20, nice between equal kernel priorities, \
         Windows classes Realtime to Idle, unknown last"
    );
}
