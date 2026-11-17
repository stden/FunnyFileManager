//! Unit tests: the viewer's default keys resolve to their commands exactly
//! as written — a modifier more is another key — a key the keymap gives
//! another command runs that one, and a key-less cursor move reads as a
//! scroll.

use crossterm::event::{KeyCode, KeyModifiers};

use super::*;
use crate::keymap::tables::VIEWER;
use crate::keymap::{ActionName, Explicit};
use crate::test_support::chord;

const SHIFT: KeyModifiers = KeyModifiers::SHIFT;
const ALT: KeyModifiers = KeyModifiers::ALT;
const CTRL: KeyModifiers = KeyModifiers::CONTROL;

const CLOSING_KEYS: [KeyCode; 4] = [KeyCode::Esc, KeyCode::Enter, KeyCode::F(3), KeyCode::F(10)];

const SCROLL_KEYS: [(KeyCode, ViewerScroll); 8] = [
    (KeyCode::Up, ViewerScroll::LineUp),
    (KeyCode::Down, ViewerScroll::LineDown),
    (KeyCode::PageUp, ViewerScroll::PageUp),
    (KeyCode::PageDown, ViewerScroll::PageDown),
    (KeyCode::Home, ViewerScroll::Top),
    (KeyCode::End, ViewerScroll::Bottom),
    (KeyCode::Left, ViewerScroll::Left),
    (KeyCode::Right, ViewerScroll::Right),
];

const WIDE_KEYS: [(KeyCode, ViewerScroll); 2] = [
    (KeyCode::Left, ViewerScroll::LeftWide),
    (KeyCode::Right, ViewerScroll::RightWide),
];

/// Every way to hold Shift, Alt and Ctrl beside a key.
fn held() -> [KeyModifiers; 7] {
    [
        SHIFT,
        ALT,
        SHIFT | ALT,
        CTRL,
        CTRL | SHIFT,
        CTRL | ALT,
        CTRL | ALT | SHIFT,
    ]
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// What the default keys make of a key.
fn by_default(code: KeyCode, modifiers: KeyModifiers) -> Option<ViewerCommand> {
    resolve(&VIEWER.defaults(), KeyEvent::new(code, modifiers))
}

/// The keys `ffm.yml` gives one action, over the defaults of the others.
fn remapped(action: &'static str, keys: &[&str]) -> Bindings<ViewerCommand> {
    let chords = keys.iter().copied().map(chord).collect();
    let explicit = Explicit::from([(ActionName::new(action), chords)]);
    VIEWER.layered(&explicit).bindings
}

#[test]
fn the_closing_keys_close() {
    for code in CLOSING_KEYS {
        let asked = by_default(code, KeyModifiers::NONE);
        assert_eq!(asked, Some(ViewerCommand::Close), "{code:?}");
    }
}

#[test]
fn the_scroll_keys_scroll() {
    for (code, scroll) in SCROLL_KEYS {
        let asked = by_default(code, KeyModifiers::NONE);
        assert_eq!(asked, Some(ViewerCommand::Scroll(scroll)), "{code:?}");
    }
}

#[test]
fn ctrl_with_a_horizontal_arrow_is_the_wide_step() {
    for (code, wide) in WIDE_KEYS {
        let asked = by_default(code, CTRL);
        assert_eq!(asked, Some(ViewerCommand::Scroll(wide)), "{code:?}");
    }
}

/// Exact matching: Shift+F3 is not F3, Ctrl+Down is not Down, and
/// Ctrl+Shift+Left is not Ctrl+Left.
#[test]
fn a_modifier_more_is_another_key() {
    let bound: Vec<KeyCode> = CLOSING_KEYS
        .into_iter()
        .chain(SCROLL_KEYS.map(|(code, _)| code))
        .collect();
    for code in bound {
        for modifiers in held() {
            let wide = modifiers == CTRL && WIDE_KEYS.iter().any(|(key, _)| *key == code);
            if !wide {
                assert_eq!(by_default(code, modifiers), None, "{modifiers:?} {code:?}");
            }
        }
    }
}

/// A key the desktop passes with Super held is the key: FFM binds no Super.
#[test]
fn super_beside_a_key_is_not_a_modifier_of_the_chord() {
    let asked = by_default(KeyCode::Esc, KeyModifiers::SUPER);
    assert_eq!(asked, Some(ViewerCommand::Close));
}

#[test]
fn a_key_the_viewer_does_not_use_resolves_to_nothing() {
    for code in [
        KeyCode::Tab,
        KeyCode::F(1),
        KeyCode::F(5),
        KeyCode::Char('q'),
        KeyCode::Char(' '),
        KeyCode::Backspace,
        KeyCode::Insert,
        KeyCode::CapsLock,
    ] {
        assert_eq!(by_default(code, KeyModifiers::NONE), None, "{code:?}");
    }
    assert_eq!(by_default(KeyCode::Char('q'), CTRL), None);
}

#[test]
fn a_command_given_other_keys_follows_them_and_leaves_its_default() {
    let bindings = remapped("line_down", &["j", "Shift+Down"]);
    let line_down = Some(ViewerCommand::Scroll(ViewerScroll::LineDown));
    assert_eq!(resolve(&bindings, key(KeyCode::Char('j'))), line_down);
    let shift_down = KeyEvent::new(KeyCode::Down, SHIFT);
    assert_eq!(resolve(&bindings, shift_down), line_down);
    assert_eq!(resolve(&bindings, key(KeyCode::Down)), None);
    let line_up = Some(ViewerCommand::Scroll(ViewerScroll::LineUp));
    assert_eq!(resolve(&bindings, key(KeyCode::Up)), line_up);
}

#[test]
fn a_key_given_to_another_command_runs_that_one() {
    let bindings = remapped("top", &["PgUp"]);
    let top = Some(ViewerCommand::Scroll(ViewerScroll::Top));
    assert_eq!(resolve(&bindings, key(KeyCode::PageUp)), top);
    assert_eq!(resolve(&bindings, key(KeyCode::Home)), None);
}

#[test]
fn a_command_left_without_a_key_has_none() {
    let bindings = remapped("scroll_left_wide", &[]);
    assert_eq!(resolve(&bindings, KeyEvent::new(KeyCode::Left, CTRL)), None);
    let left = Some(ViewerCommand::Scroll(ViewerScroll::Left));
    assert_eq!(resolve(&bindings, key(KeyCode::Left)), left);
}

/// A cursor move that reaches the quick view without a key — the wheel —
/// scrolls as its key would; anything else falls through.
#[test]
fn a_key_less_cursor_move_reads_as_a_scroll() {
    for (action, scroll) in [
        (Action::NavUp, ViewerScroll::LineUp),
        (Action::NavDown, ViewerScroll::LineDown),
        (Action::PageUp, ViewerScroll::PageUp),
        (Action::PageDown, ViewerScroll::PageDown),
        (Action::First, ViewerScroll::Top),
        (Action::Last, ViewerScroll::Bottom),
    ] {
        assert_eq!(
            ViewerScroll::from_action(action),
            Some(scroll),
            "{action:?}"
        );
    }
    for action in [Action::View, Action::Open, Action::SwitchPanel] {
        assert_eq!(ViewerScroll::from_action(action), None, "{action:?}");
    }
}

/// Under the default keys a key-less source scrolls the preview as the key
/// it stands for does: the wheel is a line, as Up and Down are. Close is the
/// viewer's alone; the quick view leaves it to the file panel's actions.
#[test]
fn a_key_less_source_scrolls_as_its_default_key_does() {
    use crate::action::tests::keyed_sources::keyed_sources;
    for source in keyed_sources() {
        let by_key = match resolve(&VIEWER.defaults(), source.key) {
            Some(ViewerCommand::Scroll(scroll)) => Some(scroll),
            _ => None,
        };
        assert_eq!(
            ViewerScroll::from_action(source.action),
            by_key,
            "{}",
            source.shortcut
        );
    }
}
