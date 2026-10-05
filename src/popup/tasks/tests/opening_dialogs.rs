//! Purpose: the Browse keys that open a dialog on the task list — each opens its own, Information on an empty list, and Gray+ offering the cursor's name as the mask.

use super::*;

/// Whether the right dialog is open.
type Opened = fn(&Modal) -> bool;

#[test]
fn each_dialog_key_opens_its_dialog() {
    let cases: [(KeyCode, Opened); 8] = [
        (KeyCode::F(1), |modal| matches!(modal, Modal::Help(_))),
        (KeyCode::Enter, |modal| matches!(modal, Modal::Info(_))),
        (KeyCode::F(4), |modal| matches!(modal, Modal::Filter(_))),
        (KeyCode::Char('/'), |modal| {
            matches!(modal, Modal::Search(_))
        }),
        (KeyCode::Char('4'), |modal| {
            matches!(modal, Modal::PidJump(_))
        }),
        (KeyCode::F(6), |modal| {
            matches!(modal, Modal::Menu(TaskMenu::Sort(_)))
        }),
        (KeyCode::Char('u'), |modal| {
            matches!(modal, Modal::Menu(TaskMenu::User { .. }))
        }),
        (KeyCode::F(7), |modal| matches!(modal, Modal::Priority(_))),
    ];
    for (code, opened) in cases {
        let mut tasks = three();
        press(&mut tasks, code);
        assert!(opened(tasks.list().modal()), "{code:?}");
        press(&mut tasks, KeyCode::Esc);
        assert_eq!(tasks.list().modal(), &Modal::Browse, "{code:?} closes");
        assert!(tasks.app.overlay.is_active());
    }
}

#[test]
fn information_on_an_empty_list_says_nothing_is_selected() {
    let mut tasks = TaskApp::open(Vec::new(), Privilege::User);
    press(&mut tasks, KeyCode::Enter);
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(
        tasks.list().notice(),
        Some(Notice::Refused(ControlRefusal::NoTarget))
    );
}

#[test]
fn gray_plus_offers_the_cursor_process_name_as_the_mask() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::Char('+'));
    let Modal::Mask(prompt) = tasks.list().modal() else {
        panic!("the mask dialog is open");
    };
    assert_eq!(prompt.mask(), "vim");
    let mut empty = TaskApp::open(Vec::new(), Privilege::User);
    press(&mut empty, KeyCode::Char('-'));
    let Modal::Mask(prompt) = empty.list().modal() else {
        panic!("the mask dialog is open");
    };
    assert_eq!(prompt.mask(), "*");
}
