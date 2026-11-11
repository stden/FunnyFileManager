//! Purpose: A walk up the parent links from an unvisited row also stops safely
//! when the component is not a cycle: at a root, or before a reached row.

use super::*;
use crate::models::tasks::tests_support::{child_of, process};

#[test]
fn a_parent_walk_stops_at_a_root_or_an_already_reached_component() {
    let root = process(1, "parent");
    let child = child_of(1, process(2, "child"));
    let family = Family::of(&[&root, &child]);
    let mut visited = Visited::new(2);
    assert_eq!(family.cycle_entry(1, &visited), 0);
    visited.insert(0);
    assert_eq!(family.cycle_entry(1, &visited), 1);
}
