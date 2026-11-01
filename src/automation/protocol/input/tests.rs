//! Purpose: Validate every structured mouse variant before any input reaches an app.
use super::*;

fn mouse(kind: &str, button: Option<&str>, modifiers: &[&str]) -> Mouse {
    Mouse {
        kind: kind.into(),
        button: button.map(str::to_owned),
        column: 7,
        row: 9,
        modifiers: modifiers.iter().map(|value| (*value).to_owned()).collect(),
    }
}

#[test]
fn mouse_variants_preserve_button_coordinates_and_modifiers() {
    let cases = [
        ("down", MouseEventKind::Down(MouseButton::Middle)),
        ("up", MouseEventKind::Up(MouseButton::Middle)),
        ("drag", MouseEventKind::Drag(MouseButton::Middle)),
        ("move", MouseEventKind::Moved),
        ("moved", MouseEventKind::Moved),
        ("scroll_up", MouseEventKind::ScrollUp),
        ("scroll_down", MouseEventKind::ScrollDown),
        ("scroll_left", MouseEventKind::ScrollLeft),
        ("scroll_right", MouseEventKind::ScrollRight),
    ];
    for (name, kind) in cases {
        let events = events(
            vec![],
            None,
            Some(mouse(name, Some("middle"), &["ctrl", "alt", "shift"])),
        )
        .unwrap();
        assert_eq!(
            events,
            vec![Event::Mouse(MouseEvent {
                kind,
                column: 7,
                row: 9,
                modifiers: KeyModifiers::CONTROL | KeyModifiers::ALT | KeyModifiers::SHIFT
            })]
        );
    }
}

#[test]
fn mouse_defaults_and_control_alias_are_supported() {
    assert_eq!(button(Some("right")).unwrap(), MouseButton::Right);
    assert_eq!(button(None).unwrap(), MouseButton::Left);
    assert_eq!(
        modifiers(&["control".into()]).unwrap(),
        KeyModifiers::CONTROL
    );
}

#[test]
fn malformed_input_batches_are_rejected_atomically() {
    for invalid in [
        mouse("unknown", None, &[]),
        mouse("down", Some("unknown"), &[]),
        mouse("down", None, &["unknown"]),
    ] {
        let error = events(
            vec!["Down".into()],
            Some("valid prefix".into()),
            Some(invalid),
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    }
    assert_eq!(
        events(vec!["unknown-key".into()], None, None)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        events(vec!["Down".into(); 513], None, None)
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
}

#[test]
fn accepted_batch_order_is_keys_then_paste_then_mouse() {
    let parsed = events(
        vec!["Down".into(); 512],
        Some("Unicode 文本".into()),
        Some(mouse("down", None, &[])),
    )
    .unwrap();
    assert_eq!(parsed.len(), 514);
    assert!(matches!(parsed[0], Event::Key(_)));
    assert_eq!(parsed[512], Event::Paste("Unicode 文本".into()));
    assert!(matches!(parsed[513], Event::Mouse(_)));
}
