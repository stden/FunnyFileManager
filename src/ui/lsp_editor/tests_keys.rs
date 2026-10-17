//! Purpose: Pin the Language servers dialog's keys on screen: where the F3
//! view is in its legend, and the form and the exit question naming only
//! their own keys over a blank key bar.

use super::*;

#[test]
fn f3_shows_where_the_view_is() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::F(3));
    let screen = buffer_string(&draw(&mut app, 80, 24));
    assert!(screen.contains(" Language servers: all "), "{screen}");
    assert!(screen.contains("Availability is what FFM would start; it is not a process list."));
    assert!(screen.contains("Lines 1-18 of "), "{screen}");
    assert!(screen.contains("3List"), "{screen}");
    assert!(screen.contains("10Back"), "{screen}");
    press(&mut app, KeyCode::End);
    let screen = buffer_string(&draw(&mut app, 80, 24));
    assert!(screen.contains("yaml  [enabled]"), "{screen}");
}

/// The key bar, the bottom row, has no label: no F-key acts.
fn assert_blank_key_bar(buffer: &Buffer) {
    let bar = buffer_string(buffer)
        .lines()
        .last()
        .unwrap_or_default()
        .to_owned();
    for label in ["Save", "View", "Edit", "Check", "Add", "Reset", "Quit"] {
        assert!(!bar.contains(label), "{label}: {bar}");
    }
}

/// Each mode names its own keys: no F-key acts in the form or the exit
/// question, so their key bar is blank, and the legend says what does act.
#[test]
fn the_form_and_the_exit_question_name_their_own_keys() {
    let machine = LspMachine::new();
    let mut app = machine.opened();
    press(&mut app, KeyCode::Enter);
    let buffer = draw(&mut app, 100, 30);
    assert!(buffer_string(&buffer).contains("Enter OK  Esc Cancel  Tab Next  Ctrl+Space List"));
    assert_blank_key_bar(&buffer);
    // After a space in Extensions every other extension is offered.
    for code in [KeyCode::Tab, KeyCode::Tab, KeyCode::End, KeyCode::Char(' ')] {
        press(&mut app, code);
    }
    press_with(&mut app, KeyCode::Char(' '), KeyModifiers::CONTROL);
    let screen = buffer_string(&draw(&mut app, 100, 30));
    assert!(
        screen.contains("Up Down Choose  Enter Insert  Esc Close list"),
        "{screen}"
    );
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Esc);
    press(&mut app, KeyCode::Char(' '));
    press(&mut app, KeyCode::Esc);
    let buffer = draw(&mut app, 100, 30);
    assert!(
        buffer_string(&buffer).contains("Y Yes  N No  Esc Cancel"),
        "{}",
        buffer_string(&buffer)
    );
    assert_blank_key_bar(&buffer);
}
