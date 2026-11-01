//! Unit tests: the keys the completed F9 menu names, each resolving to its
//! own action.
//!
//! One row per key rather than one test per key: the point is the table, and
//! a table that lives in the test is the menu written a second time only
//! where the keyboard and the menu are meant to agree.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::{Action, FileCommand, PanelCommand, ToolCommand};
use crate::menu::MenuCommand;
use crate::models::{PanelSide, PanelView, SortMode};
use crate::test_support::panels_action as resolve;

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
        (KeyCode::F(12), CTRL, Action::Panel(PanelCommand::SortMenu)),
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
    let panel = Action::Panel;
    assert_bindings(&[
        (KeyCode::Char('t'), CTRL, panel(PanelCommand::Tree)),
        (KeyCode::Char('q'), CTRL, panel(PanelCommand::QuickView)),
        (KeyCode::Char('n'), CTRL, panel(PanelCommand::LongNames)),
        (
            KeyCode::F(1),
            CTRL,
            panel(PanelCommand::TogglePanel(PanelSide::Left)),
        ),
        (
            KeyCode::F(2),
            CTRL,
            panel(PanelCommand::TogglePanel(PanelSide::Right)),
        ),
    ]);
}

#[test]
fn the_files_menu_keys() {
    let file = Action::File;
    assert_bindings(&[
        (KeyCode::F(6), ALT, file(FileCommand::Link)),
        (KeyCode::Delete, ALT, file(FileCommand::Wipe)),
        (KeyCode::F(1), SHIFT, file(FileCommand::AddToArchive)),
        (KeyCode::F(2), SHIFT, file(FileCommand::Extract)),
        (KeyCode::F(3), SHIFT, file(FileCommand::ArchiveCommands)),
        (KeyCode::Char('a'), CTRL, file(FileCommand::Attributes)),
        (KeyCode::Char('g'), CTRL, file(FileCommand::ApplyCommand)),
        (KeyCode::Char('z'), CTRL, file(FileCommand::Describe)),
    ]);
}

#[test]
fn the_commands_menu_keys() {
    let tool = Action::Tool;
    assert_bindings(&[
        (KeyCode::F(7), ALT, tool(ToolCommand::FindFile)),
        (KeyCode::F(8), ALT, tool(ToolCommand::History)),
        (KeyCode::F(9), ALT, tool(ToolCommand::VideoMode)),
        (KeyCode::F(10), ALT, tool(ToolCommand::FindFolder)),
        (KeyCode::F(11), ALT, tool(ToolCommand::ViewHistory)),
        (KeyCode::F(12), ALT, tool(ToolCommand::FolderHistory)),
        (KeyCode::Char('i'), CTRL, tool(ToolCommand::Filter)),
        (
            KeyCode::F(12),
            KeyModifiers::NONE,
            tool(ToolCommand::Screens),
        ),
        (KeyCode::Char('w'), CTRL, tool(ToolCommand::Tasks)),
        (KeyCode::F(9), SHIFT, Action::Menu(MenuCommand::SaveSetup)),
    ]);
}

/// Exact matching: each chord is its own key. Ctrl+Alt+W is the WSL manager,
/// not Ctrl+W's task list; Shift+F5 and Alt+F3 are not Copy and View any more,
/// so they fall back.
#[test]
fn chords_beside_a_binding_are_other_keys() {
    assert_bindings(&[
        (KeyCode::Char('w'), CTRL.union(ALT), Action::WslManagement),
        (KeyCode::Char('o'), CTRL, Action::ToggleConsole),
        (KeyCode::F(5), SHIFT, Action::QuickSearchClear),
        (KeyCode::F(3), ALT, Action::QuickSearchClear),
        (
            KeyCode::Delete,
            KeyModifiers::NONE,
            Action::QuickSearchClear,
        ),
    ]);
}

/// The rows Far gives no key are built only by the menu: none of these keys
/// resolves to one.
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

/// The menu takes its own keys exactly: a chord with extra modifiers is not a
/// menu key, even where the main view would still answer it.
#[test]
fn the_menu_takes_only_its_exact_keys() {
    use super::is_menu_key;
    let panels = crate::keymap::tables::PANELS.defaults();

    assert!(is_menu_key(
        &panels,
        KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL)
    ));
    assert!(!is_menu_key(
        &panels,
        KeyEvent::new(
            KeyCode::Char('t'),
            KeyModifiers::CONTROL | KeyModifiers::ALT
        )
    ));
    assert!(!is_menu_key(
        &panels,
        KeyEvent::new(KeyCode::Delete, KeyModifiers::ALT | KeyModifiers::SHIFT)
    ));
}
