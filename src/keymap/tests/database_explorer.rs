//! Purpose: The Commands menu row and the keymap row of the database explorer
//! both name the same command, so either one opens the explorer.

use crate::action::Action;
use crate::keymap::tables;
use crate::menu::{Entry, Heading, Menu, MenuCommand};

#[test]
fn the_commands_row_and_its_keymap_row_both_open_the_explorer() {
    let explorer = Action::Menu(MenuCommand::DatabaseExplorer);
    let menu_rows: Vec<Action> = Menu::open(Heading::Commands)
        .entries()
        .iter()
        .filter_map(|entry| match entry {
            Entry::Item { label, action, .. } if *label == "Data&base explorer" => Some(*action),
            _ => None,
        })
        .collect();
    assert_eq!(menu_rows, [explorer]);

    let keymap_rows: Vec<Action> = tables::PANELS
        .rows()
        .filter(|row| row.name.as_str() == "database_explorer")
        .map(|row| row.command)
        .collect();
    assert_eq!(keymap_rows, [explorer]);
}
