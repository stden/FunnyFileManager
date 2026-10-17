//! Unit tests: the keys the completed F9 menu names, each resolving to its
//! own action.
//!
//! One row per key rather than one test per key: the point is the table, and
//! a table that lives in the test is the menu written a second time only
//! where the keyboard and the menu are meant to agree.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{resolve, Action, PanelCommand};
use crate::menu::MenuCommand;
use crate::models::{PanelSide, PanelView, SortMode};

const CTRL: KeyModifiers = KeyModifiers::CONTROL;
const ALT: KeyModifiers = KeyModifiers::ALT;
const SHIFT: KeyModifiers = KeyModifiers::SHIFT;

fn assert_bindings(cases: &[(KeyCode, KeyModifiers, Action)]) {
    for (code, modifiers, expected) in cases {
        assert_eq!(
            resolve(KeyEvent::new(*code, *modifiers)),
            *expected,
            "{modifiers:?}+{code:?}"
        );
    }
}

#[test]
fn ctrl_digits_are_the_ten_view_modes_in_fars_order() {
    let digits = ['1', '2', '3', '4', '5', '6', '7', '8', '9', '0'];
    for (digit, view) in digits.into_iter().zip(PanelView::ALL) {
        assert_eq!(
            resolve(KeyEvent::new(KeyCode::Char(digit), CTRL)),
            Action::PanelView(view),
            "Ctrl+{digit}"
        );
    }
}

#[test]
fn the_left_right_menu_sort_keys() {
    let sort = |mode| Action::Panel(PanelCommand::Sort(mode));
    assert_bindings(&[
        (KeyCode::F(12), CTRL, Action::MenuCommand(MenuCommand::Sort)),
        (KeyCode::F(3), CTRL, sort(SortMode::Name)),
        (KeyCode::F(4), CTRL, sort(SortMode::Extension)),
        (KeyCode::F(5), CTRL, sort(SortMode::ModificationTime)),
        (KeyCode::F(6), CTRL, sort(SortMode::Size)),
        (KeyCode::F(7), CTRL, sort(SortMode::Unsorted)),
        (KeyCode::F(8), CTRL, sort(SortMode::CreationTime)),
        (KeyCode::F(9), CTRL, sort(SortMode::AccessTime)),
        (KeyCode::F(10), CTRL, sort(SortMode::Descriptions)),
        (KeyCode::F(11), CTRL, sort(SortMode::Owner)),
    ]);
}

#[test]
fn the_left_right_menu_toggle_keys() {
    let toggle = |side| Action::MenuCommand(MenuCommand::TogglePanel(side));
    let command = Action::MenuCommand;
    assert_bindings(&[
        (KeyCode::Char('t'), CTRL, command(MenuCommand::Tree)),
        (KeyCode::Char('q'), CTRL, command(MenuCommand::QuickView)),
        (KeyCode::Char('n'), CTRL, command(MenuCommand::LongNames)),
        (KeyCode::F(1), CTRL, toggle(PanelSide::Left)),
        (KeyCode::F(2), CTRL, toggle(PanelSide::Right)),
    ]);
}

#[test]
fn the_files_menu_keys() {
    let command = Action::MenuCommand;
    assert_bindings(&[
        (KeyCode::F(6), ALT, command(MenuCommand::Link)),
        (KeyCode::Delete, ALT, command(MenuCommand::Wipe)),
        (KeyCode::F(1), SHIFT, command(MenuCommand::Archive)),
        (KeyCode::F(2), SHIFT, command(MenuCommand::Extract)),
        (KeyCode::F(3), SHIFT, command(MenuCommand::ArchiveCommands)),
        (KeyCode::Char('a'), CTRL, command(MenuCommand::Attributes)),
        (KeyCode::Char('g'), CTRL, command(MenuCommand::ApplyCommand)),
        (KeyCode::Char('z'), CTRL, command(MenuCommand::Describe)),
    ]);
}

#[test]
fn the_commands_menu_keys() {
    let command = Action::MenuCommand;
    assert_bindings(&[
        (KeyCode::F(7), ALT, command(MenuCommand::FindFile)),
        (KeyCode::F(8), ALT, command(MenuCommand::History)),
        (KeyCode::F(9), ALT, command(MenuCommand::VideoMode)),
        (KeyCode::F(10), ALT, command(MenuCommand::FindFolder)),
        (KeyCode::F(11), ALT, command(MenuCommand::ViewHistory)),
        (KeyCode::F(12), ALT, command(MenuCommand::FolderHistory)),
        (KeyCode::Char('i'), CTRL, command(MenuCommand::Filter)),
        (
            KeyCode::F(12),
            KeyModifiers::NONE,
            command(MenuCommand::Screens),
        ),
        (KeyCode::Char('w'), CTRL, command(MenuCommand::Tasks)),
        (KeyCode::F(9), SHIFT, command(MenuCommand::SaveSetup)),
    ]);
}

/// The keys that were already bound keep their meaning beside the new ones:
/// `Ctrl+Alt+W` is still WSL, not the task list, and a modified F-key the
/// tables do not name still falls through to the bare key.
#[test]
fn the_new_bindings_do_not_shadow_the_old_ones() {
    assert_bindings(&[
        (KeyCode::Char('w'), CTRL.union(ALT), Action::WslManagement),
        (KeyCode::Char('o'), CTRL, Action::ToggleConsole),
        (KeyCode::F(5), SHIFT, Action::Copy),
        (KeyCode::F(3), ALT, Action::View),
        (
            KeyCode::Delete,
            KeyModifiers::NONE,
            Action::QuickSearchClear,
        ),
    ]);
}

/// A menu row without a key is the one action no key produces.
#[test]
fn no_key_resolves_to_a_menu_command() {
    for code in [KeyCode::F(12), KeyCode::Char('c'), KeyCode::Enter] {
        for modifiers in [KeyModifiers::NONE, CTRL, ALT, SHIFT] {
            assert!(!matches!(
                resolve(KeyEvent::new(code, modifiers)),
                Action::Menu(_)
            ));
        }
    }
}
