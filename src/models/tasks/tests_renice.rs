//! Purpose: Tests for planned renices: one step from each target's own value
//! and refused at a bound, one value for every target of the Priority form
//! with the direction each one moves, the value the form offers, the
//! confirmation `[` asks for, and `[`'s dependence on root.

use super::tests_support::{key, listing, process, OWN};
use super::{
    ControlRefusal, KernelPriority, NiceConfirm, NicePlan, NiceStep, Niceness, Operation, Priority,
    PriorityChange, PriorityClass, ProcessKey, ProcessSample, Renice, ReniceButton, SettableClass,
    Start, TaskList, TaskSession,
};
use crate::prompt::Privilege;

mod tests_confirm;
mod tests_set;
mod tests_step;

fn nice(value: i32) -> Niceness {
    Niceness::try_from(value).expect("a fixture nice value")
}

fn at_nice(raw: u32, name: &str, value: i32) -> ProcessSample {
    ProcessSample {
        priority: Priority::Unix {
            kernel: KernelPriority::new(20),
            nice: nice(value),
        },
        ..process(raw, name)
    }
}

fn with_priority(raw: u32, priority: Priority) -> ProcessSample {
    ProcessSample {
        priority,
        ..process(raw, "win")
    }
}

fn marked_all(processes: Vec<ProcessSample>) -> TaskList {
    let mut list = listing(processes);
    list.invert_marks();
    list
}
