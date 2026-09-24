//! Purpose: Tests for the task list's key router and its Browse keys: every
//! key consumed, the close keys that never quit ffm, the notice cleared by
//! the next key, a dialog re-centred when what is open changes, and each
//! Browse command reaching the list.

use crossterm::event::{KeyCode, KeyModifiers};

use crate::models::{
    Arrangement, Cadence, CommandView, ControlRefusal, KernelThreads, Modal, Notice, Schedule,
    SortKey, TaskMenu,
};
use crate::prompt::Privilege;
use crate::test_support::task_list::{key_of, named, TaskApp};

/// Presses `code` with no modifier; true when the list consumed it.
pub(super) fn press(tasks: &mut TaskApp, code: KeyCode) -> bool {
    tasks.press(code)
}

pub(super) fn chord(tasks: &mut TaskApp, code: KeyCode, modifiers: KeyModifiers) -> bool {
    tasks.chord(code, modifiers)
}

/// Types `text` key by key.
pub(super) fn type_text(tasks: &mut TaskApp, text: &str) {
    tasks.type_text(text);
}

/// Three of bob's processes, the cursor on the first, PID 10.
pub(super) fn three() -> TaskApp {
    TaskApp::open(
        vec![named(10, "vim"), named(20, "python3"), named(30, "nginx")],
        Privilege::User,
    )
}

/// The PID under the cursor.
pub(super) fn cursor_pid(tasks: &mut TaskApp) -> Option<u32> {
    tasks.list().cursor().map(|cursor| cursor.key().pid().get())
}

#[test]
fn every_close_key_closes_the_list_and_never_quits() {
    let closers = [
        (KeyCode::Esc, KeyModifiers::NONE),
        (KeyCode::F(10), KeyModifiers::NONE),
        (KeyCode::Char('q'), KeyModifiers::NONE),
        (KeyCode::Char('c'), KeyModifiers::CONTROL),
        (KeyCode::Char('w'), KeyModifiers::CONTROL),
    ];
    for (code, modifiers) in closers {
        let mut tasks = three();
        assert!(chord(&mut tasks, code, modifiers));
        assert!(!tasks.app.overlay.is_active(), "{modifiers:?} {code:?}");
        assert!(tasks.app.lifecycle.is_running());
    }
}

#[test]
fn an_unbound_key_is_consumed_and_changes_nothing() {
    let mut tasks = three();
    assert!(press(&mut tasks, KeyCode::Char('x')));
    assert!(press(&mut tasks, KeyCode::F(12)));
    assert_eq!(tasks.list().modal(), &Modal::Browse);
    assert_eq!(cursor_pid(&mut tasks), Some(10));
}

#[test]
fn a_key_clears_the_notice_before_it_acts() {
    let mut tasks = three();
    tasks
        .list()
        .set_notice(Notice::Refused(ControlRefusal::NoTarget));
    press(&mut tasks, KeyCode::Down);
    assert_eq!(tasks.list().notice(), None);
}

#[test]
fn a_dialog_opens_centred_and_keeps_its_place_while_it_stays_open() {
    let mut tasks = three();
    tasks.app.dialog_mouse.offset = (3, 1);
    press(&mut tasks, KeyCode::Down);
    assert_eq!(tasks.app.dialog_mouse.offset, (3, 1), "nothing opened");
    press(&mut tasks, KeyCode::F(8));
    assert_eq!(tasks.app.dialog_mouse.offset, (0, 0), "opened centred");
    tasks.app.dialog_mouse.offset = (2, 2);
    press(&mut tasks, KeyCode::Tab);
    assert_eq!(tasks.app.dialog_mouse.offset, (2, 2), "the same dialog");
    press(&mut tasks, KeyCode::Esc);
    assert_eq!(tasks.app.dialog_mouse.offset, (0, 0));
}

#[test]
fn the_list_keys_move_the_cursor_and_mark() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::End);
    assert_eq!(cursor_pid(&mut tasks), Some(30));
    press(&mut tasks, KeyCode::Home);
    press(&mut tasks, KeyCode::Char(' '));
    assert_eq!(cursor_pid(&mut tasks), Some(20), "marked and moved down");
    assert!(tasks.list().is_marked(key_of(10)));
    press(&mut tasks, KeyCode::Insert);
    assert_eq!(tasks.list().marked_count(), 2);
    press(&mut tasks, KeyCode::Char('*'));
    assert_eq!(tasks.list().marked_count(), 1, "inverted");
    press(&mut tasks, KeyCode::Char('U'));
    assert_eq!(tasks.list().marked_count(), 0);
    press(&mut tasks, KeyCode::Char('c'));
    assert!(tasks.list().is_marked(key_of(30)), "the cursor's family");
}

#[test]
fn the_view_keys_change_what_is_shown_and_how() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::F(2));
    assert_eq!(tasks.list().command_view(), CommandView::Name);
    press(&mut tasks, KeyCode::Char('t'));
    assert_eq!(tasks.list().options().arrangement, Arrangement::Tree);
    press(&mut tasks, KeyCode::Char('N'));
    assert_eq!(tasks.list().options().sort.key(), SortKey::Pid);
    assert_eq!(tasks.list().options().arrangement, Arrangement::Flat);
    press(&mut tasks, KeyCode::Char('I'));
    assert_eq!(tasks.shown(), [30, 20, 10]);
    press(&mut tasks, KeyCode::Char('K'));
    assert_eq!(tasks.list().options().kernel_threads, KernelThreads::Shown);
    chord(&mut tasks, KeyCode::F(3), KeyModifiers::CONTROL);
    assert_eq!(tasks.list().options().sort.key(), SortKey::Name);
}

#[test]
fn hold_and_refresh_reach_the_schedule() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::F(11));
    assert_eq!(tasks.list().refresh().cadence(), Cadence::Held);
    chord(&mut tasks, KeyCode::Char('r'), KeyModifiers::CONTROL);
    assert_eq!(tasks.list().refresh().schedule(), Schedule::Due);
    press(&mut tasks, KeyCode::Char('Z'));
    assert_eq!(tasks.list().refresh().cadence(), Cadence::Live);
}

#[test]
fn left_and_right_pan_the_command_column_by_one_or_twenty() {
    let mut tasks = three();
    press(&mut tasks, KeyCode::Right);
    chord(&mut tasks, KeyCode::Right, KeyModifiers::CONTROL);
    assert_eq!(tasks.list().command_scroll().cells(), 21);
    press(&mut tasks, KeyCode::Left);
    assert_eq!(tasks.list().command_scroll().cells(), 20);
    chord(&mut tasks, KeyCode::Left, KeyModifiers::CONTROL);
    assert_eq!(tasks.list().command_scroll().cells(), 0);
}

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

#[cfg(unix)]
#[test]
fn kernel_threads_join_the_list_with_capital_k_only() {
    let kthread = crate::models::ProcessSample {
        kind: crate::models::ProcessKind::KernelThread,
        ..named(2, "kthreadd")
    };
    let mut tasks = TaskApp::open(vec![kthread, named(10, "vim")], Privilege::User);
    assert_eq!(tasks.shown(), [10]);
    chord(&mut tasks, KeyCode::Char('k'), KeyModifiers::SHIFT);
    assert_eq!(tasks.shown(), [2, 10]);
    press(&mut tasks, KeyCode::Char('k'));
    assert!(matches!(
        tasks.list().modal(),
        Modal::Menu(TaskMenu::Signal { .. })
    ));
}
