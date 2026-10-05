//! Purpose: clicks on the task list's dialog buttons — each is the key it stands for, and the form's and the confirmation's buttons take the focus they name.

use super::*;

#[test]
fn the_confirmation_buttons_press_what_they_name() {
    let terminate = Control::Signal(key_of(10), crate::models::Signal::Terminate);
    for (button, sent) in [("{ Terminate }", Some(terminate)), ("[ Cancel ]", None)] {
        let mut term = terminal();
        let mut tasks = three();
        tasks.press(KeyCode::F(8));
        click_text(&mut term, &mut tasks.app, button);
        assert_eq!(tasks.host.controls().first().copied(), sent, "{button}");
        assert_eq!(tasks.list().modal(), &Modal::Browse, "{button}");
    }
}

#[cfg(unix)]
#[test]
fn the_kill_button_kills_whatever_the_keyboard_focused() {
    let mut term = terminal();
    let mut tasks = three();
    tasks.press(KeyCode::F(8));
    click_text(&mut term, &mut tasks.app, "[ Kill ]");
    let kill = Control::Signal(key_of(10), crate::models::Signal::Kill);
    assert_eq!(tasks.host.controls(), [kill]);
}

#[cfg(unix)]
#[test]
fn the_send_button_sends_the_signal_chosen_in_the_menu() {
    let mut term = terminal();
    let mut tasks = three();
    tasks.press(KeyCode::F(9));
    tasks.press(KeyCode::Down);
    tasks.press(KeyCode::Down);
    tasks.press(KeyCode::Enter);
    assert!(
        tasks.host.controls().is_empty(),
        "the menu only opens the confirmation"
    );
    click_text(&mut term, &mut tasks.app, "{ Send }");
    let hangup = Control::Signal(key_of(10), crate::models::Signal::Hangup);
    assert_eq!(tasks.host.controls(), [hangup]);
}

#[test]
fn each_task_dialog_button_is_the_key_it_stands_for() {
    let mut tasks = three();
    tasks.press(KeyCode::F(8));
    let buttons: Vec<KeyCode> = (0..3)
        .filter_map(|index| dialog_button_key(&mut tasks.app, index))
        .map(|key| key.code)
        .collect();
    #[cfg(unix)]
    let expected = [KeyCode::Char('t'), KeyCode::Char('k'), KeyCode::Esc];
    #[cfg(not(unix))]
    let expected = [KeyCode::Char('t'), KeyCode::Esc];
    assert_eq!(buttons, expected);
    tasks.press(KeyCode::Esc);
    tasks.press(KeyCode::F(3));
    assert_eq!(code(&mut tasks.app, 0), KeyCode::Esc, "Info's OK");
    tasks.press(KeyCode::Esc);
    assert_eq!(dialog_button_key(&mut tasks.app, 0), None, "nothing open");
}

#[test]
fn the_form_and_the_bracket_confirmation_focus_the_clicked_button() {
    let mut tasks = three();
    tasks.press(KeyCode::F(7));
    for (index, focus) in [(1, FormFocus::Cancel), (0, FormFocus::Set)] {
        assert_eq!(code(&mut tasks.app, index), KeyCode::Enter);
        let Modal::Priority(form) = tasks.list().modal() else {
            panic!("the form");
        };
        assert_eq!(form.focus(), focus);
    }
    assert_eq!(dialog_button_key(&mut tasks.app, 2), None);
    tasks.press(KeyCode::Esc);
    tasks.press(KeyCode::Char('['));
    for (index, focus) in [(1, ReniceButton::Cancel), (0, ReniceButton::Renice)] {
        assert_eq!(code(&mut tasks.app, index), KeyCode::Enter);
        let Modal::Renice(confirm) = tasks.list().modal() else {
            panic!("the confirmation");
        };
        assert_eq!(confirm.focused(), focus);
    }
    assert_eq!(dialog_button_key(&mut tasks.app, 2), None);
    tasks.press(KeyCode::Esc);
    tasks.press(KeyCode::Char('+'));
    assert_eq!(code(&mut tasks.app, 1), KeyCode::Enter);
    let Modal::Mask(prompt) = tasks.list().modal() else {
        panic!("the mask dialog");
    };
    assert!(prompt.on_cancel());
}
