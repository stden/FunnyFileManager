//! Purpose: the text syntax of §5.4 — every name and alias, the case rules,
//! `+` as a key, and every error; Shift's place is `tests_syntax_shift`.

use super::tests::chord;
use super::*;

pub(super) fn error(text: &str) -> KeyError {
    text.parse::<KeyChord>()
        .expect_err(&format!("{text} should not parse"))
}

pub(super) fn character(character: char, modifiers: Modifiers) -> KeyChord {
    KeyChord {
        key: Key::Char(character),
        modifiers,
    }
}

pub(super) const CTRL_SHIFT: Modifiers = Modifiers::CTRL.with(Modifier::Shift);

#[test]
fn every_name_and_alias_parses_in_any_case() {
    for named in NamedKey::ALL {
        let expected = KeyChord {
            key: Key::Named(named),
            modifiers: Modifiers::NONE,
        };
        assert_eq!(chord(named.name()), expected);
        assert_eq!(chord(&named.name().to_uppercase()), expected);
        assert_eq!(chord(&named.name().to_lowercase()), expected);
    }
    assert_eq!(chord("Insert"), chord("Ins"));
    assert_eq!(chord("delete"), chord("Del"));
    assert_eq!(chord("Escape"), chord("Esc"));
    assert_eq!(chord("shift+pageup"), chord("Shift+PgUp"));
    assert_eq!(chord("PageDown"), chord("PgDn"));
}

#[test]
fn function_keys_run_from_f1_to_f24() {
    for number in 1..=24 {
        let expected = FunctionKey::new(number).map(Key::F);
        assert_eq!(Some(chord(&format!("F{number}")).key), expected);
        assert_eq!(Some(chord(&format!("f{number}")).key), expected);
    }
    assert_eq!(error("F0"), KeyError::FunctionKeyRange("F0".to_owned()));
    assert_eq!(error("F25"), KeyError::FunctionKeyRange("F25".to_owned()));
    assert_eq!(
        error("Ctrl+F99"),
        KeyError::FunctionKeyRange("F99".to_owned())
    );
    assert_eq!(error("F123"), KeyError::UnknownKey("F123".to_owned()));
    assert_eq!(error("Fx"), KeyError::UnknownKey("Fx".to_owned()));
    assert_eq!(chord("F"), character('F', Modifiers::NONE));
}

#[test]
fn modifiers_come_in_any_order_and_case() {
    let all = Modifiers::CTRL.with(Modifier::Alt).with(Modifier::Shift);
    assert_eq!(chord("shift+ctrl+alt+F5").modifiers, all);
    assert_eq!(chord("ALT+Shift+CTRL+F5"), chord("Ctrl+Alt+Shift+F5"));
    assert_eq!(chord("CTRL+k"), character('k', Modifiers::CTRL));
    assert_eq!(chord("alt+Left").modifiers, Modifiers::ALT);
}

#[test]
fn the_key_is_what_follows_the_last_separating_plus() {
    assert_eq!(chord("+"), character('+', Modifiers::NONE));
    assert_eq!(chord("Alt++"), character('+', Modifiers::ALT));
    assert_eq!(chord("Ctrl+Shift++"), character('+', CTRL_SHIFT));
    assert_eq!(chord("Ctrl+\\"), character('\\', Modifiers::CTRL));
    assert_eq!(error("Ctrl+"), KeyError::NoKey(Modifier::Ctrl));
    assert_eq!(error("Alt+"), KeyError::NoKey(Modifier::Alt));
    assert_eq!(error("Ctrl+Alt"), KeyError::NoKey(Modifier::Alt));
    assert_eq!(error("shift"), KeyError::NoKey(Modifier::Shift));
}

#[test]
fn a_repeated_modifier_is_an_error() {
    assert_eq!(error("Ctrl+Ctrl+X"), KeyError::Repeated(Modifier::Ctrl));
    assert_eq!(
        error("Shift+Alt+shift+F1"),
        KeyError::Repeated(Modifier::Shift)
    );
    assert_eq!(error("Alt+ALT+"), KeyError::Repeated(Modifier::Alt));
}

#[test]
fn empty_blank_and_unknown_texts_are_errors() {
    assert_eq!(error(""), KeyError::Empty);
    assert_eq!(error(" "), KeyError::NotPrintable(" ".to_owned()));
    assert_eq!(error("\t"), KeyError::NotPrintable("\t".to_owned()));
    assert_eq!(error("Ctrl+ "), KeyError::NotPrintable(" ".to_owned()));
    // Invisible format characters a copy from a browser or an editor pastes.
    for invisible in ["\u{200B}", "\u{FEFF}", "\u{AD}", "\u{200D}", "\u{2062}"] {
        assert_eq!(
            error(invisible),
            KeyError::NotPrintable(invisible.to_owned())
        );
    }
    assert_eq!(error("ab"), KeyError::UnknownKey("ab".to_owned()));
    assert_eq!(error("Foo"), KeyError::UnknownKey("Foo".to_owned()));
    assert_eq!(
        error("Ctrl+Return"),
        KeyError::UnknownKey("Return".to_owned())
    );
    assert_eq!(
        error("Ctrl + K"),
        KeyError::UnknownKey("Ctrl + K".to_owned())
    );
    assert_eq!(error("Ctrl+K "), KeyError::UnknownKey("K ".to_owned()));
}

#[test]
fn errors_read_as_sentences() {
    let cases = [
        (KeyError::Empty, "no key"),
        (KeyError::NoKey(Modifier::Ctrl), "no key after Ctrl+"),
        (KeyError::Repeated(Modifier::Alt), "Alt is given twice"),
        (
            KeyError::ShiftedCharacter(Key::Char('1')),
            "Shift+1: Shift is part of the character; write the character itself",
        ),
        (
            KeyError::ShiftedCharacter(Key::Named(NamedKey::Space)),
            "Shift+Space: Shift is part of the character; write the character itself",
        ),
        (
            KeyError::FunctionKeyRange("F25".to_owned()),
            "F25: function keys are F1 to F24",
        ),
        (
            KeyError::NotPrintable(" ".to_owned()),
            "' ' is not a printable key (the space bar is Space)",
        ),
        (KeyError::UnknownKey("Foo".to_owned()), "unknown key 'Foo'"),
    ];
    for (error, text) in cases {
        assert_eq!(error.to_string(), text);
    }
    let source: &dyn std::error::Error = &KeyError::Empty;
    assert!(source.source().is_none());
}
