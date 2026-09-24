//! Purpose: Tests for the task list's keymap: every binding of the spec's
//! Browse table, the shifted letters, and the keys left unbound.

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

use super::{normalize, resolve, Pan, PanStep, TaskCommand};
use crate::models::{ListMotion, MaskPurpose, NiceStep, SortKey};
use crate::scroll::WIDE_STEP;

const NONE: KeyModifiers = KeyModifiers::NONE;
const SHIFT: KeyModifiers = KeyModifiers::SHIFT;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;
const ALT: KeyModifiers = KeyModifiers::ALT;

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn ch(character: char) -> KeyCode {
    KeyCode::Char(character)
}

/// Every binding of the Browse table (spec 3.1), one row each.
pub(super) const BINDINGS: &[(KeyCode, KeyModifiers, TaskCommand)] = &[
    (KeyCode::Up, NONE, TaskCommand::Move(ListMotion::Up)),
    (KeyCode::Down, NONE, TaskCommand::Move(ListMotion::Down)),
    (KeyCode::PageUp, NONE, TaskCommand::Move(ListMotion::PageUp)),
    (
        KeyCode::PageDown,
        NONE,
        TaskCommand::Move(ListMotion::PageDown),
    ),
    (KeyCode::Home, NONE, TaskCommand::Move(ListMotion::Home)),
    (KeyCode::End, NONE, TaskCommand::Move(ListMotion::End)),
    (
        KeyCode::Left,
        NONE,
        TaskCommand::Pan(Pan::Left(PanStep::Cell)),
    ),
    (
        KeyCode::Right,
        NONE,
        TaskCommand::Pan(Pan::Right(PanStep::Cell)),
    ),
    (
        KeyCode::Left,
        CTRL,
        TaskCommand::Pan(Pan::Left(PanStep::Wide)),
    ),
    (
        KeyCode::Right,
        CTRL,
        TaskCommand::Pan(Pan::Right(PanStep::Wide)),
    ),
    (KeyCode::Char(' '), NONE, TaskCommand::ToggleMark),
    (KeyCode::Insert, NONE, TaskCommand::ToggleMark),
    (KeyCode::Char('c'), NONE, TaskCommand::MarkFamily),
    (KeyCode::Char('U'), NONE, TaskCommand::UnmarkAll),
    (KeyCode::Char('*'), NONE, TaskCommand::InvertMarks),
    (
        KeyCode::Char('+'),
        NONE,
        TaskCommand::MarkByMask(MaskPurpose::Select),
    ),
    (
        KeyCode::Char('-'),
        NONE,
        TaskCommand::MarkByMask(MaskPurpose::Unselect),
    ),
    (KeyCode::Enter, NONE, TaskCommand::Info),
    (KeyCode::F(3), NONE, TaskCommand::Info),
    (KeyCode::F(1), NONE, TaskCommand::Help),
    (KeyCode::Char('h'), NONE, TaskCommand::Help),
    (KeyCode::Char('?'), NONE, TaskCommand::Help),
    (KeyCode::F(2), NONE, TaskCommand::ToggleCommandView),
    (KeyCode::F(4), NONE, TaskCommand::Filter),
    (KeyCode::Char('\\'), NONE, TaskCommand::Filter),
    (KeyCode::Char('/'), NONE, TaskCommand::Search(None)),
    (KeyCode::Char('j'), ALT, TaskCommand::Search(Some('j'))),
    (KeyCode::Char('+'), ALT, TaskCommand::Search(Some('+'))),
    (KeyCode::Char('0'), NONE, TaskCommand::PidJump('0')),
    (KeyCode::Char('4'), NONE, TaskCommand::PidJump('4')),
    (KeyCode::Char('9'), NONE, TaskCommand::PidJump('9')),
    (KeyCode::F(5), NONE, TaskCommand::ToggleTree),
    (KeyCode::Char('t'), NONE, TaskCommand::ToggleTree),
    (KeyCode::Char('t'), CTRL, TaskCommand::ToggleTree),
    (KeyCode::F(6), NONE, TaskCommand::SortMenu),
    (KeyCode::F(12), CTRL, TaskCommand::SortMenu),
    (KeyCode::Char('<'), NONE, TaskCommand::SortMenu),
    (KeyCode::Char('>'), NONE, TaskCommand::SortMenu),
    (KeyCode::F(3), CTRL, TaskCommand::SortBy(SortKey::Name)),
    (KeyCode::F(6), CTRL, TaskCommand::SortBy(SortKey::Resident)),
    (KeyCode::F(7), CTRL, TaskCommand::SortBy(SortKey::Pid)),
    (KeyCode::F(8), CTRL, TaskCommand::SortBy(SortKey::Started)),
    (KeyCode::F(10), CTRL, TaskCommand::SortBy(SortKey::Command)),
    (KeyCode::F(11), CTRL, TaskCommand::SortBy(SortKey::User)),
    (KeyCode::Char('P'), NONE, TaskCommand::SortBy(SortKey::Cpu)),
    (
        KeyCode::Char('M'),
        NONE,
        TaskCommand::SortBy(SortKey::Resident),
    ),
    (
        KeyCode::Char('T'),
        NONE,
        TaskCommand::SortBy(SortKey::CpuTime),
    ),
    (KeyCode::Char('N'), NONE, TaskCommand::SortBy(SortKey::Pid)),
    (KeyCode::Char('I'), NONE, TaskCommand::InvertOrder),
    (KeyCode::Char('u'), NONE, TaskCommand::UserMenu),
    (KeyCode::Char('K'), NONE, TaskCommand::ToggleKernelThreads),
    (KeyCode::F(7), NONE, TaskCommand::Priority),
    (KeyCode::Char(']'), NONE, TaskCommand::Nice(NiceStep::Raise)),
    (KeyCode::Char('['), NONE, TaskCommand::Nice(NiceStep::Lower)),
    (KeyCode::F(8), NONE, TaskCommand::Terminate),
    (KeyCode::Delete, NONE, TaskCommand::Terminate),
    (KeyCode::F(9), NONE, TaskCommand::SignalMenu),
    (KeyCode::Char('k'), NONE, TaskCommand::SignalMenu),
    (KeyCode::F(11), NONE, TaskCommand::ToggleHold),
    (KeyCode::Char('Z'), NONE, TaskCommand::ToggleHold),
    (KeyCode::Char('r'), CTRL, TaskCommand::RefreshNow),
    (KeyCode::Char('l'), CTRL, TaskCommand::RefreshNow),
    (KeyCode::Esc, NONE, TaskCommand::Close),
    (KeyCode::F(10), NONE, TaskCommand::Close),
    (KeyCode::Char('q'), NONE, TaskCommand::Close),
    (KeyCode::Char('c'), CTRL, TaskCommand::Close),
    (KeyCode::Char('w'), CTRL, TaskCommand::Close),
];

#[test]
fn every_binding_of_the_browse_table_resolves_to_its_command() {
    for &(code, modifiers, command) in BINDINGS {
        assert_eq!(
            resolve(key(code, modifiers)),
            command,
            "{modifiers:?} {code:?}"
        );
    }
}

/// Terminals and the BDD harness send a capital either as the capital or as
/// the small letter with SHIFT; both are the capital, and the small letter
/// alone keeps its own meaning.
#[test]
fn a_shifted_letter_is_its_capital_however_it_arrives() {
    for modifiers in [NONE, SHIFT] {
        assert_eq!(
            resolve(key(ch('K'), modifiers)),
            TaskCommand::ToggleKernelThreads
        );
    }
    assert_eq!(
        resolve(key(ch('k'), SHIFT)),
        TaskCommand::ToggleKernelThreads
    );
    assert_eq!(resolve(key(ch('k'), NONE)), TaskCommand::SignalMenu);
    for (small, capital) in [('u', 'U'), ('i', 'I'), ('p', 'P'), ('z', 'Z')] {
        assert_eq!(
            resolve(key(ch(small), SHIFT)),
            resolve(key(ch(capital), NONE)),
            "{small}"
        );
    }
    assert_eq!(
        resolve(key(ch('w'), CTRL | SHIFT)),
        TaskCommand::Close,
        "Ctrl+Shift+w, which opens the list, closes it too"
    );
    assert_eq!(
        resolve(key(ch('j'), ALT | SHIFT)),
        TaskCommand::Search(Some('J'))
    );
}

#[test]
fn normalize_drops_shift_only_from_characters_and_keeps_the_rest_of_the_event() {
    let event = KeyEvent {
        code: ch('k'),
        modifiers: SHIFT | CTRL,
        kind: KeyEventKind::Repeat,
        state: KeyEventState::KEYPAD,
    };
    let normal = normalize(event);
    assert_eq!(normal.code, ch('K'));
    assert_eq!(normal.modifiers, CTRL);
    assert_eq!((normal.kind, normal.state), (event.kind, event.state));
    let tab = key(KeyCode::BackTab, SHIFT);
    assert_eq!(normalize(tab), tab, "a named key keeps its SHIFT");
    assert_eq!(normalize(key(ch('<'), SHIFT)), key(ch('<'), NONE));
}

#[test]
fn keys_without_a_process_meaning_are_consumed_and_do_nothing() {
    let unbound = [
        (KeyCode::F(4), CTRL),
        (KeyCode::F(5), CTRL),
        (KeyCode::F(9), CTRL),
        (KeyCode::F(3), SHIFT),
        (KeyCode::F(8), ALT),
        (KeyCode::F(12), NONE),
        (KeyCode::F(13), NONE),
        (KeyCode::Delete, ALT),
        (KeyCode::Delete, SHIFT),
        (KeyCode::Tab, NONE),
        (KeyCode::Backspace, NONE),
        (ch('x'), NONE),
        (ch('F'), NONE),
        (ch('a'), CTRL),
        (ch('!'), NONE),
    ];
    for (code, modifiers) in unbound {
        assert_eq!(
            resolve(key(code, modifiers)),
            TaskCommand::Ignore,
            "{modifiers:?} {code:?}"
        );
    }
}

/// htop's F7 and F8 renice and F3 searches; here they only open dialogs,
/// and the plain F-keys with any modifier do nothing at all.
#[test]
fn no_function_key_with_a_modifier_reaches_a_signal() {
    for number in 1..=12 {
        for modifiers in [SHIFT, ALT, CTRL | ALT, CTRL | SHIFT] {
            assert_eq!(
                resolve(key(KeyCode::F(number), modifiers)),
                TaskCommand::Ignore,
                "{modifiers:?} F{number}"
            );
        }
    }
}

#[test]
fn a_pan_step_is_one_cell_or_the_viewers_wide_step() {
    assert_eq!(PanStep::Cell.cells(), 1);
    assert_eq!(PanStep::Wide.cells(), WIDE_STEP);
}

/// Ctrl+Shift+letter arrives as the capital with Ctrl; it means what the
/// Ctrl+letter means.
#[test]
fn a_ctrl_letter_means_the_same_with_shift() {
    for letter in ['c', 'w', 't', 'r', 'l'] {
        assert_eq!(
            resolve(key(ch(letter), CTRL | SHIFT)),
            resolve(key(ch(letter), CTRL)),
            "{letter}"
        );
        assert_ne!(resolve(key(ch(letter), CTRL)), TaskCommand::Ignore);
    }
    assert_eq!(resolve(key(KeyCode::Delete, CTRL)), TaskCommand::Ignore);
}
