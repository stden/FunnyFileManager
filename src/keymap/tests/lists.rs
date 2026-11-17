//! Purpose: the lists that move alike share their six moves, so the arrows,
//! the paging keys and Home and End mean the same on every list.

use super::*;
use crate::keymap::tables::{NETBOX_PANEL, TASK_LIST, TREE_PANEL, VAULT};

/// The six rows every list shares, in the order the shared group writes them.
const MOVES: [&str; 6] = [
    "cursor_up",
    "cursor_down",
    "page_up",
    "page_down",
    "cursor_home",
    "cursor_end",
];

/// The name and default keys of each shared move a table has, in table order.
fn moves<C>(table: &Table<C>) -> Vec<(&'static str, &'static [KeyChord])> {
    table
        .rows()
        .filter(|row| MOVES.contains(&row.name.as_str()))
        .map(|row| (row.name.as_str(), row.defaults))
        .collect()
}

#[test]
fn every_list_moves_with_the_same_six_keys() {
    let tree = moves(&TREE_PANEL);
    assert_eq!(tree.len(), 6, "the tree panel lost a shared move: {tree:?}");
    assert_eq!(
        moves(&NETBOX_PANEL),
        tree,
        "the NetBox list moves differently"
    );
    assert_eq!(moves(&TASK_LIST), tree, "the task list moves differently");
    assert_eq!(moves(&VAULT), tree, "the vault list moves differently");
}
