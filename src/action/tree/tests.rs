//! Unit tests: the tree's keys resolve to its commands, a typed character
//! searches, and the keys that are not the tree's resolve to nothing.

use super::*;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn alt(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::ALT)
}

#[test]
fn the_cursor_keys_move_open_and_close() {
    for (code, command) in [
        (KeyCode::Up, TreeCommand::Move(ListMotion::Up)),
        (KeyCode::Down, TreeCommand::Move(ListMotion::Down)),
        (KeyCode::Home, TreeCommand::Move(ListMotion::Home)),
        (KeyCode::End, TreeCommand::Move(ListMotion::End)),
        (KeyCode::PageUp, TreeCommand::Move(ListMotion::PageUp)),
        (KeyCode::PageDown, TreeCommand::Move(ListMotion::PageDown)),
        (KeyCode::Right, TreeCommand::Expand),
        (KeyCode::Left, TreeCommand::Collapse),
        (KeyCode::Enter, TreeCommand::GoToFolder),
        (KeyCode::Char('+'), TreeCommand::NextSibling),
        (KeyCode::Char('-'), TreeCommand::PreviousSibling),
    ] {
        assert_eq!(resolve(key(code)), Some(command), "{code:?}");
        assert_eq!(
            resolve(KeyEvent::new(code, KeyModifiers::SHIFT)),
            Some(command),
            "Shift is ignored on {code:?}"
        );
    }
}

#[test]
fn the_search_keys_and_the_typed_characters() {
    assert_eq!(resolve(key(KeyCode::Esc)), Some(TreeCommand::ClearSearch));
    assert_eq!(
        resolve(key(KeyCode::Backspace)),
        Some(TreeCommand::ShortenSearch)
    );
    assert_eq!(resolve(ctrl(KeyCode::Enter)), Some(TreeCommand::FindNext));
    assert_eq!(
        resolve(key(KeyCode::Char('g'))),
        Some(TreeCommand::Search('g'))
    );
    assert_eq!(
        resolve(alt(KeyCode::Char('b'))),
        Some(TreeCommand::Search('b')),
        "Alt+letter is Far's fast find"
    );
    assert_eq!(
        resolve(KeyEvent::new(KeyCode::Char('G'), KeyModifiers::SHIFT)),
        Some(TreeCommand::Search('G'))
    );
}

#[test]
fn ctrl_r_refreshes_in_either_case() {
    for ch in ['r', 'R'] {
        assert_eq!(
            resolve(ctrl(KeyCode::Char(ch))),
            Some(TreeCommand::RefreshTree)
        );
    }
}

#[test]
fn the_file_operation_keys_need_a_file_panel() {
    for n in 5..=8 {
        assert_eq!(
            resolve(key(KeyCode::F(n))),
            Some(TreeCommand::NeedsFilePanel)
        );
        assert_eq!(
            resolve(alt(KeyCode::F(n))),
            Some(TreeCommand::NeedsFilePanel)
        );
    }
}

#[test]
fn the_keys_the_tree_does_not_own_resolve_to_nothing() {
    for event in [
        ctrl(KeyCode::Char('t')),
        ctrl(KeyCode::Char('o')),
        ctrl(KeyCode::F(5)),
        ctrl(KeyCode::Up),
        key(KeyCode::Tab),
        key(KeyCode::Char(':')),
        key(KeyCode::F(1)),
        key(KeyCode::F(9)),
        key(KeyCode::F(10)),
        key(KeyCode::Insert),
        key(KeyCode::Delete),
    ] {
        assert_eq!(resolve(event), None, "{event:?}");
    }
}

/// The panels actions the tree reads as its own when they arrive without a
/// key — a menu row, a mouse gesture — and the command each becomes.
const READ_AS_THE_TREES: [(Action, TreeCommand); 15] = {
    [
        (Action::NavUp, TreeCommand::Move(ListMotion::Up)),
        (Action::NavDown, TreeCommand::Move(ListMotion::Down)),
        (Action::First, TreeCommand::Move(ListMotion::Home)),
        (Action::Last, TreeCommand::Move(ListMotion::End)),
        (Action::PageUp, TreeCommand::Move(ListMotion::PageUp)),
        (Action::PageDown, TreeCommand::Move(ListMotion::PageDown)),
        (Action::Open, TreeCommand::GoToFolder),
        (Action::Refresh, TreeCommand::RefreshTree),
        (Action::SelectGroup, TreeCommand::NextSibling),
        (Action::UnselectGroup, TreeCommand::PreviousSibling),
        (Action::Copy, TreeCommand::NeedsFilePanel),
        (Action::Move, TreeCommand::NeedsFilePanel),
        (Action::DeleteConfirm, TreeCommand::NeedsFilePanel),
        (
            Action::Dialog(DialogCommand::Mkdir),
            TreeCommand::NeedsFilePanel,
        ),
        (
            Action::Selection(SelectionCommand::InvertSelection),
            TreeCommand::NeedsFilePanel,
        ),
    ]
};

/// A panels action that reaches the tree without a key reads as the command
/// its key would; the file panel's own fall through.
#[test]
fn a_key_less_action_reads_as_the_trees_command() {
    for (action, command) in READ_AS_THE_TREES {
        assert_eq!(
            TreeCommand::from_action(action),
            Some(command),
            "{action:?}"
        );
    }
    for action in [Action::View, Action::SwitchPanel, Action::CommandLine] {
        assert_eq!(TreeCommand::from_action(action), None, "{action:?}");
    }
}

/// `from_action` and `resolve` are two tables of one meaning until step 5
/// deletes one: every key-less source must read as its key does. The
/// corners where they differ are these and no others: `*` searches the tree
/// while Invert selection needs a file panel (§14.3), and the key table
/// takes F5–F8 with any modifier (§2.3), so Alt+F6, Alt+F7 and Alt+F8 need
/// a file panel while Link, Find file and History are not the tree's.
#[test]
fn a_key_less_source_reads_as_its_key_does() {
    use crate::action::tests::keyed_sources::keyed_sources;
    let drift: Vec<&str> = keyed_sources()
        .into_iter()
        .filter(|source| TreeCommand::from_action(source.action) != resolve(source.key))
        .map(|source| source.shortcut)
        .collect();
    assert_eq!(drift, ["Alt+F6", "Gray *", "Alt+F7", "Alt+F8"]);
}
