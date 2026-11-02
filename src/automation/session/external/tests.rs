//! Purpose: Preserve private-terminal input encoding and published lifecycle states.
use super::*;
use crossterm::event::{KeyCode, KeyEvent};

#[test]
fn mouse_events_use_sgr_coordinates_buttons_and_modifiers() {
    use MouseButton::{Left, Middle, Right};
    use MouseEventKind::{Down, Drag, Moved, ScrollDown, ScrollLeft, ScrollRight, ScrollUp, Up};
    let cases = [
        (Down(Left), "0;1;1M"),
        (Down(Middle), "1;1;1M"),
        (Down(Right), "2;1;1M"),
        (Up(Right), "2;1;1m"),
        (Drag(Middle), "33;1;1M"),
        (Moved, "35;1;1M"),
        (ScrollUp, "64;1;1M"),
        (ScrollDown, "65;1;1M"),
        (ScrollLeft, "66;1;1M"),
        (ScrollRight, "67;1;1M"),
    ];
    for (kind, expected) in cases {
        let mouse = MouseEvent {
            kind,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        assert_eq!(mouse_sequence(mouse), format!("\x1b[<{expected}"));
    }
    let mouse = MouseEvent {
        kind: Down(Left),
        column: u16::MAX,
        row: u16::MAX,
        modifiers: KeyModifiers::SHIFT | KeyModifiers::ALT | KeyModifiers::CONTROL,
    };
    assert_eq!(mouse_sequence(mouse), "\x1b[<28;65536;65536M");
}

#[test]
fn idle_input_rejects_payloads_but_ignores_focus_and_resize() {
    let host = HeadlessHost::new();
    input(
        &host,
        vec![Event::FocusGained, Event::Resize(80, 24), Event::FocusLost],
    )
    .unwrap();
    for event in [
        Event::Paste("private".into()),
        Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE)),
        Event::Mouse(MouseEvent {
            kind: MouseEventKind::Moved,
            column: 1,
            row: 2,
            modifiers: KeyModifiers::NONE,
        }),
    ] {
        assert_eq!(
            input(&host, vec![event]).unwrap_err().kind(),
            io::ErrorKind::NotConnected
        );
    }
    let state = snapshot(&host);
    assert_eq!(state["state"], "idle");
    assert!(state["code"].is_null());
    assert_eq!(state["output"], "");
    assert_eq!(state["dropped_bytes"], 0);
}

#[test]
fn failed_spawn_is_reported_without_a_fabricated_exit_code() {
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let missing = tempfile::TempDir::new().unwrap();
    let command = std::process::Command::new(missing.path().join("missing-ffm-program"));
    assert!(crate::terminal::headless::run(&command).is_err());
    assert_eq!(snapshot(&host)["state"], "failed");
    assert!(snapshot(&host)["code"].is_null());
}

#[cfg(unix)]
mod process;
