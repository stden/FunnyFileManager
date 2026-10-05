//! Purpose: the task list's PID jump — digits jump, any other key ends it — and a terminal paste into the open bar.

use super::*;

#[test]
fn digits_jump_to_that_pid_and_backspace_steps_back() {
    let mut tasks = TaskApp::open(
        vec![named(2, "a"), named(20, "b"), named(200, "c")],
        Privilege::User,
    );
    press(&mut tasks, KeyCode::End);
    type_text(&mut tasks, "20");
    assert_eq!(cursor_pid(&mut tasks), Some(20));
    press(&mut tasks, KeyCode::Char('0'));
    assert_eq!(cursor_pid(&mut tasks), Some(200));
    press(&mut tasks, KeyCode::Char('7'));
    assert_eq!(cursor_pid(&mut tasks), Some(200), "2007 is not shown");
    press(&mut tasks, KeyCode::Backspace);
    press(&mut tasks, KeyCode::Backspace);
    press(&mut tasks, KeyCode::Backspace);
    assert_eq!(cursor_pid(&mut tasks), Some(2));
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
}

#[test]
fn any_other_key_ends_the_jump_and_then_does_what_it_does() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::Char('3'));
    press(&mut tasks, KeyCode::Char('0'));
    assert_eq!(cursor_pid(&mut tasks), Some(30));
    chord(&mut tasks, KeyCode::Char('5'), KeyModifiers::ALT);
    assert!(
        matches!(tasks.list().modal(), Modal::Search(_)),
        "Alt+5 searched"
    );
    press(&mut tasks, KeyCode::Esc);
    press(&mut tasks, KeyCode::Char('1'));
    press(&mut tasks, KeyCode::F(10));
    assert!(!tasks.app.overlay.is_active(), "F10 closed the list");
    let mut tasks = three();
    press(&mut tasks, KeyCode::Char('1'));
    press(&mut tasks, KeyCode::Esc);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert!(tasks.app.overlay.is_active(), "Esc only ended the jump");
}

#[test]
fn a_paste_reaches_the_open_bar_without_control_characters() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::F(4));
    paste_text(&mut tasks.app, "ng\tinx\n");
    assert_eq!(filter_line(&mut tasks), "nginx");
    assert_eq!(tasks.shown(), [30]);
    press(&mut tasks, KeyCode::Esc);
    press(&mut tasks, KeyCode::Char('/'));
    paste_text(&mut tasks.app, "python");
    assert_eq!(cursor_pid(&mut tasks), Some(20));
    press(&mut tasks, KeyCode::Esc);
    press(&mut tasks, KeyCode::Char('3'));
    paste_text(&mut tasks.app, "0x");
    assert_eq!(cursor_pid(&mut tasks), Some(30));
}

#[test]
fn a_paste_is_cut_at_its_limit_and_goes_nowhere_without_a_bar() {
    let mut tasks = three();
    paste_text(&mut tasks.app, "vim");
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(filter_line(&mut tasks), "");
    press(&mut tasks, KeyCode::F(4));
    paste_text(&mut tasks.app, &"a".repeat(300));
    assert_eq!(filter_line(&mut tasks).len(), 256);
}

#[test]
fn a_ctrl_digit_ends_the_jump_instead_of_typing() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::Char('2'));
    chord(&mut tasks, KeyCode::Char('0'), KeyModifiers::CONTROL);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(cursor_pid(&mut tasks), Some(10), "2 is not shown");
}
