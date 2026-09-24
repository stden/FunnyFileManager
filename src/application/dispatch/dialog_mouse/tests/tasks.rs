//! Purpose: clicks on the task list — a row, a column title, a key-bar
//! cell, a menu line and a dialog's buttons — and that no click on the
//! list sends a signal without its confirmation.

use super::*;
use crate::models::{FormFocus, Modal, ReniceButton, SortKey};
use crate::prompt::Privilege;
use crate::test_support::processes::Control;
use crate::test_support::task_list::{key_of, named, TaskApp};

/// Bob's three processes, 10 under the cursor.
fn three() -> TaskApp {
    TaskApp::open(
        vec![named(10, "vim"), named(20, "python3"), named(30, "nginx")],
        Privilege::User,
    )
}

/// Draws `tasks`, then presses the left button at (`column`, `row`).
fn click(term: &mut Terminal<WritableTestBackend>, tasks: &mut TaskApp, column: u16, row: u16) {
    draw(&mut tasks.app, term);
    gesture(
        term,
        &mut tasks.app,
        MouseEventKind::Down(MouseButton::Left),
        (column, row),
    );
}

fn cursor_pid(tasks: &mut TaskApp) -> u32 {
    let list = tasks.list();
    let cursor = list.cursor().expect("a cursor");
    list.view()[cursor.index()].key().pid().get()
}

#[test]
fn a_click_on_a_row_puts_the_cursor_there() {
    let mut term = terminal();
    let mut tasks = three();
    // One core: the rows start under the titles on row 4.
    click(&mut term, &mut tasks, 20, 7);
    assert_eq!(cursor_pid(&mut tasks), 30);
    click(&mut term, &mut tasks, 20, 6);
    assert_eq!(cursor_pid(&mut tasks), 20);
    assert!(tasks.host.controls().is_empty());
}

#[test]
fn the_wheel_moves_the_cursor_as_up_and_down_do() {
    let mut term = terminal();
    let mut tasks = three();
    gesture(
        &mut term,
        &mut tasks.app,
        MouseEventKind::ScrollDown,
        (40, 10),
    );
    gesture(
        &mut term,
        &mut tasks.app,
        MouseEventKind::ScrollDown,
        (40, 10),
    );
    assert_eq!(cursor_pid(&mut tasks), 30);
    gesture(
        &mut term,
        &mut tasks.app,
        MouseEventKind::ScrollUp,
        (40, 10),
    );
    assert_eq!(cursor_pid(&mut tasks), 20);
}

#[test]
fn a_click_on_a_title_sorts_by_it_and_again_reverses() {
    let mut term = terminal();
    let mut tasks = three();
    click(&mut term, &mut tasks, 3, 4);
    assert_eq!(tasks.list().options().sort.key(), SortKey::Pid);
    assert_eq!(tasks.shown(), [10, 20, 30]);
    click(&mut term, &mut tasks, 3, 4);
    assert_eq!(tasks.shown(), [30, 20, 10]);
    click(&mut term, &mut tasks, 70, 4);
    assert_eq!(tasks.list().options().sort.key(), SortKey::Command);
}

#[test]
fn a_key_bar_click_presses_its_key() {
    let mut term = terminal();
    let mut tasks = three();
    // 80 columns: the first eight cells are 7 wide, the last four 6.
    click(&mut term, &mut tasks, 22, 23);
    assert!(matches!(tasks.list().modal(), Modal::Filter(_)), "4Filter");
    tasks.press(KeyCode::Esc);
    click(&mut term, &mut tasks, 50, 23);
    assert!(matches!(tasks.list().modal(), Modal::Confirm(_)), "8Kill");
    assert!(
        tasks.host.controls().is_empty(),
        "only the confirmation opened"
    );
    tasks.press(KeyCode::Esc);
    click(&mut term, &mut tasks, 64, 23);
    assert!(
        matches!(tasks.app.overlay, Overlay::None),
        "10Quit closes the list"
    );
}

#[test]
fn a_menu_line_click_chooses_it() {
    let mut term = terminal();
    let mut tasks = three();
    tasks.press(KeyCode::F(6));
    draw(&mut tasks.app, &mut term);
    click_text(&mut term, &mut tasks.app, "Start time");
    assert_eq!(tasks.list().options().sort.key(), SortKey::Started);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
}

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

/// Clicks twice on every cell of a `width` × `height` screen, each pair
/// from the table: the second lands on whatever the first opened.
fn double_click_everywhere(width: u16, height: u16) -> Vec<Control> {
    let mut term = Terminal::new(WritableTestBackend::new(width, height)).expect("terminal");
    let mut tasks = three();
    tasks.app.viewport = crate::models::Viewport::new(height);
    for row in 0..height {
        for column in 0..width {
            click(&mut term, &mut tasks, column, row);
            click(&mut term, &mut tasks, column, row);
            if tasks.app.task_list_mut().is_none() {
                tasks = three();
                tasks.app.viewport = crate::models::Viewport::new(height);
            }
            tasks.list().close_modal();
        }
    }
    tasks.host.controls()
}

#[test]
fn no_click_on_the_list_sends_a_signal_even_twice_in_one_place() {
    for (width, height) in [(80, 24), (40, 10), (24, 8)] {
        let sent = double_click_everywhere(width, height);
        assert!(sent.is_empty(), "{width}x{height}: {sent:?}");
    }
}
