//! Purpose: Unit tests for `record_priority` and `record_nice` when no snapshot is ready.

use super::super::{Niceness, Pid, PriorityChange, ProcessKey, Start};
use super::TaskList;

#[test]
fn priority_updates_are_ignored_while_task_list_is_not_ready() {
    let mut list = TaskList::default();
    let key = ProcessKey::new(Pid::new(1).expect("pid"), Start::Unknown);

    list.record_priority(key, PriorityChange::Nice(Niceness::NORMAL));
    list.record_nice(key, Niceness::NORMAL);

    assert_eq!(list.view(), []);
}
