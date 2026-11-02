//! Purpose: modified keys in the task list — a shifted letter is its capital, Ctrl is Ctrl with or without Shift, no modified F-key signals — and the keys that mean nothing.

use super::*;

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
        (KeyCode::F(13), NONE),
        (KeyCode::Delete, ALT),
        (KeyCode::Delete, SHIFT),
        (KeyCode::Tab, CTRL),
        (KeyCode::Tab, SHIFT),
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
