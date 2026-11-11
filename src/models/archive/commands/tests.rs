//! Purpose: Unit tests for archive menu home/end cursor navigation.

use super::{ArchiveCommand, ArchiveMenu};

#[test]
fn archive_menu_move_home_returns_to_first_command() {
    let mut menu = ArchiveMenu::default();
    menu.move_end();
    assert_eq!(menu.command(), ArchiveCommand::AddSelected);
    menu.move_up();
    assert_eq!(menu.command(), ArchiveCommand::ExtractToOtherPanel);
    menu.move_home();
    assert_eq!(menu.command(), ArchiveCommand::Test);
}
