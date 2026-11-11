//! Purpose: Tests for freezing targets: the refusals before any system call,
//! FFM's ancestors, a cyclic parent chain, and targets that survive a
//! refresh that moves every row.

use super::targets::refusal;
use super::tests_support::{child_of, key, listing, pid, process, resample, OWN};
use super::{
    ControlRefusal, Cursor, Niceness, Operation, ProcessKey, ProcessKind, ProcessSample, Relation,
    SortKey, Start, Target, TaskList,
};

mod tests_choice;
mod tests_refusals;
