//! Purpose: the chord tests' shared helper, and the small types — modifiers,
//! named keys, function keys.

use crossterm::event::KeyCode;

use super::*;

pub(super) use crate::test_support::chord;

#[test]
fn modifiers_are_a_set_in_canonical_order() {
    let set = Modifiers::NONE.with(Modifier::Shift).with(Modifier::Ctrl);
    assert!(set.contains(Modifier::Ctrl));
    assert!(set.contains(Modifier::Shift));
    assert!(!set.contains(Modifier::Alt));
    assert!(!set.is_empty());
    assert!(set.ctrl_or_alt());
    assert_eq!(
        set.iter().collect::<Vec<_>>(),
        [Modifier::Ctrl, Modifier::Shift]
    );
    assert_eq!(set.without(Modifier::Ctrl), Modifiers::SHIFT);
    assert_eq!(set.without(Modifier::Shift), Modifiers::CTRL);
    assert_eq!(set.without(Modifier::Alt), set);
    assert_eq!(set.with(Modifier::Ctrl), set);
    assert!(Modifiers::NONE.is_empty());
    assert!(!Modifiers::SHIFT.ctrl_or_alt());
    assert!(Modifiers::ALT.ctrl_or_alt());
    assert_eq!(Modifiers::default(), Modifiers::NONE);
    assert_eq!(
        Modifiers::CTRL
            .with(Modifier::Alt)
            .with(Modifier::Shift)
            .iter()
            .collect::<Vec<_>>(),
        Modifier::ALL
    );
}

#[test]
fn modifier_names_parse_in_any_case() {
    for modifier in Modifier::ALL {
        assert_eq!(Modifier::parse(modifier.name()), Some(modifier));
        assert_eq!(
            Modifier::parse(&modifier.name().to_uppercase()),
            Some(modifier)
        );
        assert_eq!(modifier.to_string(), modifier.name());
    }
    assert_eq!(Modifier::parse("ctrl"), Some(Modifier::Ctrl));
    assert_eq!(Modifier::parse("Control"), None);
    assert_eq!(Modifier::parse("Meta"), None);
    assert_eq!(Modifier::parse(""), None);
}

#[test]
fn every_named_key_has_a_name_a_code_and_parses_back() {
    for key in NamedKey::ALL {
        assert_eq!(NamedKey::parse(key.name()), Some(key), "{key}");
        assert_eq!(
            NamedKey::parse(&key.name().to_lowercase()),
            Some(key),
            "{key}"
        );
        assert_eq!(NamedKey::of_code(key.code()), Some(key), "{key}");
        assert_eq!(key.to_string(), key.name());
    }
    assert_eq!(NamedKey::Space.code(), KeyCode::Char(' '));
    assert_eq!(NamedKey::PgDn.name(), "PgDn");
}

#[test]
fn named_key_aliases_are_read_and_never_written() {
    let aliases = [
        ("Insert", NamedKey::Ins),
        ("delete", NamedKey::Del),
        ("ESCAPE", NamedKey::Esc),
        ("PageUp", NamedKey::PgUp),
        ("pagedown", NamedKey::PgDn),
    ];
    for (alias, key) in aliases {
        assert_eq!(NamedKey::parse(alias), Some(key), "{alias}");
        assert_ne!(key.name(), alias);
    }
    assert_eq!(NamedKey::parse("Return"), None);
    assert_eq!(NamedKey::parse("Esc "), None);
    assert_eq!(NamedKey::parse(""), None);
}

#[test]
fn of_code_refuses_backtab_characters_and_unbindable_codes() {
    assert_eq!(NamedKey::of_code(KeyCode::BackTab), None);
    assert_eq!(NamedKey::of_code(KeyCode::Char('k')), None);
    assert_eq!(NamedKey::of_code(KeyCode::F(1)), None);
    assert_eq!(NamedKey::of_code(KeyCode::CapsLock), None);
    assert_eq!(NamedKey::of_code(KeyCode::Null), None);
}

#[test]
fn function_keys_are_f1_to_f24() {
    assert_eq!(FunctionKey::new(0), None);
    assert_eq!(FunctionKey::new(25), None);
    assert_eq!(FunctionKey::new(1).map(FunctionKey::number), Some(1));
    assert_eq!(FunctionKey::new(24).map(FunctionKey::number), Some(24));
    assert_eq!(
        FunctionKey::new(3).map(|key| key.to_string()).as_deref(),
        Some("F3")
    );
    assert_eq!(FunctionKey::LAST, 24);
}

#[test]
fn a_saturating_function_key_is_the_nearest_of_f1_to_f24() {
    for number in 1..=FunctionKey::LAST {
        assert_eq!(
            Some(FunctionKey::saturating(number)),
            FunctionKey::new(number)
        );
    }
    assert_eq!(FunctionKey::saturating(0).number(), 1);
    assert_eq!(FunctionKey::saturating(25).number(), FunctionKey::LAST);
    assert_eq!(FunctionKey::saturating(u8::MAX).number(), FunctionKey::LAST);
}

#[test]
fn a_key_alone_keeps_a_letters_case() {
    assert_eq!(Key::Char('k').to_string(), "k");
    assert_eq!(Key::Char('K').to_string(), "K");
    assert_eq!(Key::Named(NamedKey::Ins).to_string(), "Ins");
    assert_eq!(
        FunctionKey::new(12)
            .map(Key::F)
            .map(|key| key.to_string())
            .as_deref(),
        Some("F12")
    );
}

#[test]
fn case_mapping_covers_cased_letters_only() {
    assert_eq!(lowercase('K'), Some('k'));
    assert_eq!(lowercase('k'), Some('k'));
    assert_eq!(lowercase('Ж'), Some('ж'));
    assert_eq!(lowercase('ß'), Some('ß'));
    assert_eq!(lowercase('1'), None);
    assert_eq!(lowercase('+'), None);
    assert_eq!(lowercase('中'), None);
    assert_eq!(uppercase('k'), Some('K'));
    assert_eq!(uppercase('ж'), Some('Ж'));
    assert_eq!(uppercase('ß'), None);
    assert_eq!(uppercase('é'), Some('É'));
    assert_eq!(uppercase('€'), None);
}
