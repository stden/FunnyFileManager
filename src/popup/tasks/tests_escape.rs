//! Purpose: Tests that every dialog, bar and menu of the task list closes
//! with Esc and leaves the list open, and for the keys that only move a
//! focus or the cursor while one is open.

use crossterm::event::KeyCode;

use crate::models::{
    ControlError, Failure, KillConfirm, KillOffer, MaskPrompt, MaskPurpose, Modal, NiceConfirm,
    NiceStep, Operation, PidQuery, PriorityForm, ReniceButton, SearchBar, Sort, TaskMenu, TextBar,
};
use crate::scroll::ScrollTop;
use crate::test_support::task_list::{key_of, nice, TaskApp};

use super::tests::{cursor_pid, press, three};

/// One of each thing that can be open on the list.
fn every_modal(tasks: &mut TaskApp) -> Vec<Modal> {
    let list = tasks.list();
    let targets = list.freeze_targets(Operation::Signal);
    let failure = Failure::new(
        targets.targets()[0].clone(),
        crate::models::Attempt::Signal,
        ControlError::Gone,
    );
    vec![
        Modal::Help(ScrollTop::TOP),
        Modal::Info(key_of(10)),
        Modal::Filter(TextBar::new("")),
        Modal::Search(SearchBar::default()),
        Modal::PidJump(PidQuery::default()),
        Modal::Menu(TaskMenu::sort(Sort::default())),
        Modal::Menu(TaskMenu::user(Vec::new(), &crate::models::UserScope::All)),
        Modal::Mask(MaskPrompt::new(MaskPurpose::Select, "*")),
        Modal::Confirm(KillConfirm::new(
            targets.clone(),
            KillOffer::TerminateOrKill,
        )),
        Modal::Priority(PriorityForm::new(targets, nice(0))),
        Modal::Renice(NiceConfirm::new(list.plan_step(NiceStep::Lower))),
        Modal::Error {
            done: None,
            failures: vec![failure],
        },
    ]
}

#[test]
fn every_modal_closes_with_esc_and_leaves_the_list_open() {
    let mut tasks = three();
    for modal in every_modal(&mut tasks) {
        let name = format!("{modal:?}");
        tasks.list().set_modal(modal);
        press(&mut tasks, KeyCode::Esc);
        assert_eq!(tasks.list().modal(), &Modal::Browse, "{name}");
        assert!(tasks.app.overlay.is_active(), "{name}");
    }
    assert_eq!(tasks.host.controls().len(), 0, "Esc sends nothing");
}

#[test]
fn information_closes_on_f10_and_ignores_the_list_keys() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::Enter);
    press(&mut tasks, KeyCode::Down);
    assert!(matches!(tasks.list().modal(), Modal::Info(_)));
    assert_eq!(cursor_pid(&mut tasks), Some(10), "the table stays put");
    press(&mut tasks, KeyCode::F(10));
    assert_eq!(tasks.list().modal(), &Modal::Browse);
}

#[test]
fn the_renice_confirmation_moves_its_focus_with_tab_and_the_arrows() {
    for key in [
        KeyCode::Tab,
        KeyCode::BackTab,
        KeyCode::Left,
        KeyCode::Right,
    ] {
        let mut tasks = three();
        let plan = tasks.list().plan_step(NiceStep::Lower);
        tasks
            .list()
            .set_modal(Modal::Renice(NiceConfirm::new(plan)));
        press(&mut tasks, key);
        let Modal::Renice(confirm) = tasks.list().modal() else {
            panic!("the confirmation is open");
        };
        assert_eq!(confirm.focused(), ReniceButton::Cancel, "{key:?}");
        press(&mut tasks, KeyCode::Char('x'));
        press(&mut tasks, KeyCode::Enter);
        assert_eq!(tasks.host.controls().len(), 0, "{key:?}");
    }
}

#[test]
fn every_list_key_moves_the_cursor_through_an_open_filter() {
    let mut tasks = three();
    tasks.app.viewport = crate::models::Viewport::new(8);
    press(&mut tasks, KeyCode::F(4));
    press(&mut tasks, KeyCode::PageDown);
    assert_eq!(cursor_pid(&mut tasks), Some(30));
    press(&mut tasks, KeyCode::Up);
    assert_eq!(cursor_pid(&mut tasks), Some(20));
    press(&mut tasks, KeyCode::PageUp);
    assert_eq!(cursor_pid(&mut tasks), Some(10));
    assert!(matches!(tasks.list().modal(), Modal::Filter(_)));
}
