//! Purpose: Quit, navigation, function keys, shortcut sequences and special key combinations.

use super::*;

#[test]
fn test_quit_keys() {
    // Test various quit key combinations
    let quit_events = vec![
        simulate_key_event(KeyCode::F(10), KeyModifiers::empty()),
        simulate_key_event(KeyCode::F(10), KeyModifiers::empty()),
        simulate_key_event(KeyCode::F(10), KeyModifiers::empty()),
        simulate_key_event(KeyCode::Char('c'), KeyModifiers::CONTROL),
    ];

    for event in quit_events {
        match event {
            Event::Key(key) => {
                assert!(
                    matches!(key.code, KeyCode::Char('q' | 'Q') | KeyCode::F(10))
                        || (key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL))
                );
            }
            _ => panic!("Expected key event"),
        }
    }
}

#[test]
fn test_navigation_keys() {
    // Test navigation key codes
    let nav_events = vec![
        simulate_key_event(KeyCode::Down, KeyModifiers::empty()),
        simulate_key_event(KeyCode::Up, KeyModifiers::empty()),
        simulate_key_event(KeyCode::Enter, KeyModifiers::empty()),
        simulate_key_event(KeyCode::Tab, KeyModifiers::empty()),
        simulate_key_event(KeyCode::Left, KeyModifiers::empty()),
        simulate_key_event(KeyCode::Right, KeyModifiers::empty()),
    ];

    for event in nav_events {
        match event {
            Event::Key(key) => {
                assert!(matches!(
                    key.code,
                    KeyCode::Down
                        | KeyCode::Up
                        | KeyCode::Enter
                        | KeyCode::Tab
                        | KeyCode::Left
                        | KeyCode::Right
                ));
            }
            _ => panic!("Expected key event"),
        }
    }
}

#[test]
fn test_function_keys() {
    // Test function key codes
    let function_keys = vec![
        KeyCode::F(1),  // Help
        KeyCode::F(3),  // View
        KeyCode::F(4),  // Edit
        KeyCode::F(5),  // Copy
        KeyCode::F(6),  // Move
        KeyCode::F(7),  // Mkdir
        KeyCode::F(8),  // Delete
        KeyCode::F(10), // Quit
    ];

    for key in function_keys {
        let event = simulate_key_event(key, KeyModifiers::empty());
        match event {
            Event::Key(k) => {
                assert!(matches!(k.code, KeyCode::F(_)));
            }
            _ => panic!("Expected key event"),
        }
    }
}

#[test]
fn test_keyboard_shortcut_sequences() {
    let key_sequences = vec![
        vec![KeyCode::Tab, KeyCode::Tab, KeyCode::Tab],
        vec![KeyCode::Down, KeyCode::Down, KeyCode::Up],
        vec![KeyCode::F(1), KeyCode::Esc],
        vec![KeyCode::Char('s'), KeyCode::Enter],
        vec![KeyCode::Char('c'), KeyCode::Backspace],
    ];

    for sequence in key_sequences {
        for key in sequence {
            let event = simulate_key_event(key, KeyModifiers::empty());
            match event {
                Event::Key(_) => {}
                _ => panic!("Expected key event"),
            }
        }
    }
}

#[test]
fn test_special_key_combinations() {
    let special_combinations = vec![
        (KeyCode::Char('a'), KeyModifiers::CONTROL),
        (KeyCode::Char('z'), KeyModifiers::CONTROL),
        (KeyCode::Char('A'), KeyModifiers::SHIFT),
        (KeyCode::Tab, KeyModifiers::SHIFT),
        (KeyCode::Enter, KeyModifiers::ALT),
    ];

    for (code, modifiers) in special_combinations {
        let event = simulate_key_event(code, modifiers);
        match event {
            Event::Key(key) => {
                assert_eq!(key.code, code);
                assert_eq!(key.modifiers, modifiers);
            }
            _ => panic!("Expected key event"),
        }
    }
}
