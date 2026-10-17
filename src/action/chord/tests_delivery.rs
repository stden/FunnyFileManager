//! Purpose: `Delivery` — every class of chord the parity doc and §7.3 name,
//! and the exit-action test `on_legacy`.

use super::tests::chord;
use super::*;

fn delivery(text: &str) -> Delivery {
    chord(text).delivery()
}

fn legacy_as(text: &str) -> Delivery {
    Delivery::LegacyAs(chord(text))
}

#[test]
fn legacy_bytes_arrive_as_another_key() {
    let cases = [
        ("Ctrl+I", "Tab"),
        ("Ctrl+M", "Enter"),
        ("Ctrl+[", "Esc"),
        ("Ctrl+3", "Esc"),
        ("Ctrl+8", "Backspace"),
        ("Ctrl+1", "1"),
        ("Ctrl+9", "9"),
        ("Ctrl+0", "0"),
        ("Ctrl+2", "Ctrl+Space"),
        ("Ctrl+@", "Ctrl+Space"),
        ("Ctrl+4", "Ctrl+\\"),
        ("Ctrl+]", "Ctrl+5"),
        ("Ctrl+^", "Ctrl+6"),
        ("Ctrl+_", "Ctrl+7"),
        ("Ctrl+Alt+I", "Alt+Tab"),
        ("Ctrl+Alt+]", "Ctrl+Alt+5"),
        // Shift does not reach a control byte either.
        ("Ctrl+Shift+I", "Tab"),
        ("Ctrl+Shift+M", "Enter"),
        ("Ctrl+Alt+Shift+I", "Alt+Tab"),
    ];
    for (bound, sent) in cases {
        assert_eq!(delivery(bound), legacy_as(sent), "{bound}");
    }
}

#[test]
fn ctrl_shift_letter_loses_its_shift() {
    assert_eq!(delivery("Ctrl+Shift+K"), legacy_as("Ctrl+K"));
    assert_eq!(delivery("Ctrl+Alt+Shift+K"), legacy_as("Ctrl+Alt+K"));
    assert_eq!(delivery("Ctrl+Shift+Ж"), legacy_as("Ctrl+Ж"));
    // The terminal's own copy, paste and tab keys: the lost Shift outranks
    // the terminal taking them, because it makes Ctrl+Shift+C quit.
    assert_eq!(delivery("Ctrl+Shift+C"), legacy_as("Ctrl+C"));
    assert_eq!(delivery("Ctrl+Shift+W"), legacy_as("Ctrl+W"));
    assert_eq!(delivery("Ctrl+Shift+F5"), Delivery::Everywhere);
}

#[test]
fn a_modified_f3_and_f13_and_above_are_lost() {
    for lost in [
        "Shift+F3",
        "Ctrl+F3",
        "Alt+F3",
        "Ctrl+Shift+F3",
        "F13",
        "Shift+F13",
        "F24",
    ] {
        assert_eq!(delivery(lost), Delivery::LegacyLost, "{lost}");
    }
    assert_eq!(delivery("F3"), Delivery::Everywhere);
    assert_eq!(delivery("F12"), Delivery::Everywhere);
    assert_eq!(delivery("Shift+F12"), Delivery::Everywhere);
}

#[test]
fn keys_a_terminal_takes_first() {
    for taken in ["F10", "F11", "Alt+Enter", "Shift+PgUp", "Shift+PgDn"] {
        assert_eq!(
            delivery(taken),
            Delivery::TakenFirst(Taker::Terminal),
            "{taken}"
        );
    }
    assert_eq!(delivery("Shift+F10"), Delivery::Everywhere);
    assert_eq!(delivery("Ctrl+F11"), Delivery::Everywhere);
    assert_eq!(delivery("Ctrl+PgUp"), Delivery::Everywhere);
}

#[test]
fn keys_the_desktop_takes_first() {
    for taken in ["Alt+F4", "Alt+Space", "Ctrl+Alt+Del", "Ctrl+Space"] {
        assert_eq!(
            delivery(taken),
            Delivery::TakenFirst(Taker::Desktop),
            "{taken}"
        );
    }
    assert_eq!(delivery("Shift+F4"), Delivery::Everywhere);
    assert_eq!(delivery("Alt+Del"), Delivery::Everywhere);
}

#[test]
fn keys_macos_takes_first() {
    for taken in [
        "Ctrl+Up",
        "Ctrl+Down",
        "Ctrl+Left",
        "Ctrl+Right",
        "Alt+K",
        "Alt+Shift+K",
        "Alt++",
    ] {
        assert_eq!(
            delivery(taken),
            Delivery::TakenFirst(Taker::MacOs),
            "{taken}"
        );
    }
    assert_eq!(delivery("Ctrl+Alt+K"), Delivery::Everywhere);
    assert_eq!(delivery("Alt+F1"), Delivery::Everywhere);
    assert_eq!(delivery("Alt+Left"), Delivery::Everywhere);
    assert_eq!(delivery("Ctrl+Shift+Down"), Delivery::Everywhere);
}

#[test]
fn ctrl_alt_with_a_symbol_is_altgr_on_windows() {
    // Ctrl+Alt+2 is not here: as VT bytes it is Alt+NUL first.
    for altgr in ["Ctrl+Alt+=", "Ctrl+Alt+7", "Ctrl+Alt+Shift+=", "Ctrl+Alt+{"] {
        assert_eq!(delivery(altgr), Delivery::AltGr, "{altgr}");
    }
    assert_eq!(delivery("Ctrl+Alt+Q"), Delivery::Everywhere);
    assert_eq!(delivery("Ctrl+Alt+W"), Delivery::Everywhere);
}

#[test]
fn ctrl_with_a_shifted_symbol_is_reported_differently() {
    for shifted in [
        "Ctrl++",
        "Ctrl+Shift+=",
        "Ctrl+!",
        "Ctrl+Shift+1",
        "Ctrl+?",
        "Ctrl+Shift+;",
    ] {
        assert_eq!(delivery(shifted), Delivery::ShiftedSymbol, "{shifted}");
    }
    assert_eq!(delivery("Ctrl+="), Delivery::Everywhere);
    assert_eq!(delivery("Ctrl+;"), Delivery::Everywhere);
    assert_eq!(delivery("Ctrl+5"), Delivery::Everywhere);
    assert_eq!(delivery("Ctrl+Shift+Space"), Delivery::Everywhere);
}

#[test]
fn defaults_arrive_everywhere() {
    for default in [
        "F5",
        "Shift+F6",
        "Ctrl+\\",
        "Ctrl+R",
        "Alt+F7",
        "Ctrl+PgUp",
        "Ins",
        "Space",
        "+",
        "-",
        "*",
        "Enter",
        "Esc",
        "Tab",
        "Shift+Tab",
        "Ctrl+5",
        "Ctrl+6",
        "Ctrl+7",
        "Ctrl+H",
        "Ctrl+O",
        "Ctrl+Alt+W",
        "Backspace",
        "Ctrl+Shift+F9",
        "k",
        "K",
        ":",
    ] {
        assert_eq!(delivery(default), Delivery::Everywhere, "{default}");
    }
}

#[test]
fn on_legacy_is_the_exit_action_test() {
    assert!(Delivery::Everywhere.on_legacy());
    assert!(Delivery::TakenFirst(Taker::Terminal).on_legacy());
    assert!(Delivery::TakenFirst(Taker::MacOs).on_legacy());
    assert!(Delivery::AltGr.on_legacy());
    assert!(!legacy_as("Tab").on_legacy());
    assert!(!Delivery::LegacyLost.on_legacy());
    assert!(!Delivery::ShiftedSymbol.on_legacy());
    assert!(delivery("F10").on_legacy());
    assert!(delivery("Esc").on_legacy());
    assert!(!delivery("Ctrl+I").on_legacy());
}
