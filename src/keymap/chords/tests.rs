//! Purpose: the tables' chord constructors write the chord their text
//! names, so a default reads in the file as it was written in the table.

use super::*;
use crate::action::chord::NamedKey::{Del, Enter, Ins, Space, Tab};
use crate::test_support::chord;

/// Written at run time, as the tables write them at compile time.
fn written() -> [(KeyChord, &'static str); 16] {
    [
        (f(1), "F1"),
        (f(12), "F12"),
        (f(24), "F24"),
        (ch('+'), "+"),
        (ch(':'), ":"),
        (ch('K'), "K"),
        (ch('\\'), "\\"),
        (named(Ins), "Ins"),
        (named(Space), "Space"),
        (shift(f(4)), "Shift+F4"),
        (shift(named(Tab)), "Shift+Tab"),
        (alt(named(Del)), "Alt+Del"),
        (alt(ch('b')), "Alt+B"),
        (ctrl(ch('\\')), "Ctrl+\\"),
        (ctrl(named(Enter)), "Ctrl+Enter"),
        (ctrl(alt(ch('w'))), "Ctrl+Alt+W"),
    ]
}

#[test]
fn a_constructor_writes_the_chord_its_text_names() {
    for (written, text) in written() {
        assert_eq!(written, chord(text), "{text}");
        assert_eq!(written.to_string(), text);
    }
}

#[test]
fn modifiers_wrap_in_any_order() {
    assert_eq!(ctrl(alt(shift(f(3)))), chord("Ctrl+Alt+Shift+F3"));
    assert_eq!(shift(alt(ctrl(f(3)))), chord("Ctrl+Alt+Shift+F3"));
    assert_eq!(ctrl(ctrl(f(3))), ctrl(f(3)));
}

#[test]
fn a_function_key_out_of_range_is_held_to_the_nearest() {
    assert_eq!(f(0), f(1));
    assert_eq!(f(25), f(24));
}
