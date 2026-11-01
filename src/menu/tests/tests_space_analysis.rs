//! Purpose: Space analysis is a Commands row that runs its dialog. Its label
//! holds no free letter, so the row prints no hotkey.

use super::*;
use crate::action::Action;
use crate::menu::MenuCommand;

#[test]
fn space_analysis_is_a_commands_row_that_runs_its_dialog() {
    let menu = Menu::open(Heading::Commands);
    let row = menu
        .entries()
        .iter()
        .position(|entry| label_of(entry) == "Space analysis")
        .expect("the row is listed under Commands");
    assert!(matches!(
        menu.entries()[row],
        Entry::Item {
            action: Action::Menu(MenuCommand::SpaceAnalysis),
            ..
        }
    ));
}

/// Every letter of the label is already another Commands row's hotkey, and a
/// second row on one letter is refused, so the label carries no `&`.
#[test]
fn its_label_carries_no_hotkey() {
    assert_eq!(hotkey_of("Space analysis"), None);
}
