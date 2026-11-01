//! Purpose: Search commands share task input feedback and reject other task modes.

use super::{
    invoke_search,
    tests::{cursor_pid, press, type_text},
};
use crate::action::tasks::search::TaskSearchCommand;
use crate::models::{ControlRefusal, Found, Modal, Notice};
use crate::prompt::Privilege;
use crate::test_support::task_list::{named, TaskApp};
use crossterm::event::KeyCode;

#[test]
fn typed_find_next_wraps_and_clears_notice_without_moving_the_dialog() {
    let mut tasks = TaskApp::open(
        vec![named(10, "vim"), named(20, "python3"), named(30, "python2")],
        Privilege::User,
    );
    press(&mut tasks, KeyCode::Char('/'));
    type_text(&mut tasks, "pyth");
    tasks
        .list()
        .set_notice(Notice::Refused(ControlRefusal::NoTarget));
    tasks.app.dialog_mouse.offset = (3, 2);
    assert!(invoke_search(&mut tasks.app, TaskSearchCommand::FindNext));
    assert_eq!(cursor_pid(&mut tasks), Some(30));
    assert_eq!(tasks.list().notice(), None);
    assert_eq!(tasks.app.dialog_mouse.offset, (3, 2));
    assert!(invoke_search(&mut tasks.app, TaskSearchCommand::FindNext));
    assert_eq!(cursor_pid(&mut tasks), Some(20));
    let Modal::Search(bar) = tasks.list().modal() else {
        panic!("search")
    };
    assert_eq!(bar.found(), Found::Match);
}

#[test]
fn typed_search_commands_reject_the_table_and_its_other_dialogs() {
    let mut tasks = TaskApp::open(vec![named(10, "vim")], Privilege::User);
    for modal_key in [None, Some(KeyCode::F(4)), Some(KeyCode::F(8))] {
        if let Some(key) = modal_key {
            press(&mut tasks, key);
        }
        tasks
            .list()
            .set_notice(Notice::Refused(ControlRefusal::NoTarget));
        let before = tasks.list().modal().clone();
        assert!(!invoke_search(&mut tasks.app, TaskSearchCommand::FindNext));
        assert_eq!(tasks.list().modal(), &before);
        assert!(tasks.list().notice().is_some());
        if modal_key.is_some() {
            press(&mut tasks, KeyCode::Esc);
        }
    }
    tasks.app.close_overlay();
    assert!(!invoke_search(&mut tasks.app, TaskSearchCommand::FindNext));
}
