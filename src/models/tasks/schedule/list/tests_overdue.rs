//! Purpose: Clearing an overdue mark leaves a list still waiting for its first
//! sample alone.

use super::*;

#[test]
fn clearing_overdue_leaves_a_list_waiting_for_its_first_sample_alone() {
    let mut list = TaskList::default();
    list.clear_overdue();
    assert!(matches!(list.table, Table::Waiting));
}
