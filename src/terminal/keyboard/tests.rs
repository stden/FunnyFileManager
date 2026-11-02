//! Unit tests: the bytes the protocol's push and pop write, and that a
//! legacy terminal is sent neither.

use super::*;

fn written(
    write: impl FnOnce(&mut Vec<u8>) -> io::Result<KeyboardProtocol>,
) -> (String, KeyboardProtocol) {
    let mut out = Vec::new();
    let protocol = write(&mut out).expect("write to a buffer");
    (
        String::from_utf8(out).expect("escape codes are ASCII"),
        protocol,
    )
}

/// `CSI > 5 u`: disambiguated escape codes (1) and alternate keys (4). On
/// Windows crossterm refuses both commands, and the host there is never
/// `Kitty`.
#[cfg(unix)]
#[test]
fn a_kitty_terminal_gets_the_flags_pushed_and_popped() {
    assert_eq!(
        written(|out| push(out, KeyboardProtocol::Kitty)),
        ("\u{1b}[>5u".to_owned(), KeyboardProtocol::Kitty)
    );
    assert_eq!(
        written(|out| pop(out, KeyboardProtocol::Kitty)),
        ("\u{1b}[<1u".to_owned(), KeyboardProtocol::Kitty)
    );
}

/// A terminal that did not answer the query is never sent the sequences: an
/// unknown `CSI u` could show up as garbage, and a pop could take flags
/// another program pushed.
#[test]
fn a_legacy_terminal_is_sent_nothing() {
    assert_eq!(
        written(|out| push(out, KeyboardProtocol::Legacy)),
        (String::new(), KeyboardProtocol::Legacy)
    );
    assert_eq!(
        written(|out| pop(out, KeyboardProtocol::Legacy)),
        (String::new(), KeyboardProtocol::Legacy)
    );
}

#[cfg(unix)]
fn ctrl(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::CONTROL)
}

/// 0x1C is Ctrl+\ as well as Ctrl+4; without the protocol it is taken as
/// Ctrl+\. Every other key, and every key under the protocol, passes as is.
#[cfg(unix)]
#[test]
fn a_legacy_terminals_file_separator_byte_is_ctrl_backslash() {
    assert_eq!(KeyboardProtocol::Legacy.decode(ctrl('4')), ctrl('\\'));
    let alt_ctrl_4 = KeyEvent::new(
        KeyCode::Char('4'),
        KeyModifiers::CONTROL | KeyModifiers::ALT,
    );
    assert_eq!(
        KeyboardProtocol::Legacy.decode(alt_ctrl_4).code,
        KeyCode::Char('\\')
    );
    for untouched in [
        ctrl('5'),
        ctrl('a'),
        KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE),
    ] {
        assert_eq!(KeyboardProtocol::Legacy.decode(untouched), untouched);
    }
}

#[cfg(unix)]
#[test]
fn a_kitty_terminals_keys_are_already_what_was_pressed() {
    for key in [ctrl('4'), ctrl('\\'), ctrl('5'), ctrl(']')] {
        assert_eq!(KeyboardProtocol::Kitty.decode(key), key);
    }
}

#[test]
fn events_other_than_keys_pass_through_decode_input() {
    let resize = Event::Resize(80, 24);
    assert_eq!(decode_input(resize.clone()), resize);
}
