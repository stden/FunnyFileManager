//! Purpose: Tests for what opens on the task list: the PID query's limits,
//! menu cursors that stay on a row, the confirmation's buttons and signals,
//! the priority form's value, and every notice sentence.

use super::tests_support::{listing, process};
use super::{
    Attempt, ConfirmButton, ControlError, ControlRefusal, Failure, FormFocus, Found, FrozenTargets,
    KillConfirm, KillOffer, MenuCursor, Modal, NiceStep, Niceness, Notice, Operation, Owner, Pid,
    PidQuery, PriorityForm, SearchBar, Severity, Signal, Sort, SortKey, TaskMenu, TextBar,
    UserScope,
};
use crate::models::SortOrder;

fn nice(value: i32) -> Niceness {
    Niceness::try_from(value).expect("a fixture nice value")
}

fn vim() -> FrozenTargets {
    listing(vec![process(2, "vim")]).freeze_targets(Operation::Signal)
}

#[test]
fn a_pid_query_takes_ten_digits_and_nothing_else() {
    let mut query = PidQuery::default();
    for ch in "42x7".chars() {
        query.push(ch);
    }
    assert_eq!(query.digits(), "427");
    assert_eq!(query.pid().map(Pid::get), Some(427));
    for _ in 0..20 {
        query.push('9');
    }
    assert_eq!(query.digits().len(), PidQuery::MAX_DIGITS);
    assert_eq!(query.pid(), None, "4 279 999 999 is above i32::MAX");
    query.pop();
    assert_eq!(query.digits(), "427999999");
}

#[test]
fn a_pid_query_never_names_a_pid_above_i32_max() {
    let spell = |digits: &str| {
        let mut query = PidQuery::default();
        digits.chars().for_each(|digit| query.push(digit));
        query.pid().map(Pid::get)
    };
    assert_eq!(spell("2147483647"), Some(2_147_483_647));
    assert_eq!(spell("2147483648"), None);
    assert_eq!(spell("0"), None);
    assert_eq!(spell(""), None);
}

#[test]
fn a_menu_cursor_is_always_on_a_row() {
    assert_eq!(MenuCursor::at(0, 0), None);
    assert_eq!(MenuCursor::at(7, 3).map(MenuCursor::index), Some(2));
    assert_eq!(MenuCursor::at(1, 3).map(MenuCursor::index), Some(1));
}

#[test]
fn the_sort_menu_opens_on_the_key_in_force_and_clamps_moves() {
    let mut menu = TaskMenu::sort(Sort::new(SortKey::CpuTime, SortOrder::Reversed));
    assert_eq!(menu.cursor().index(), 8);
    assert_eq!(menu.row_count(), 13);
    assert_eq!(TaskMenu::SORT_ROWS, 13);
    menu.select(99);
    assert_eq!(menu.cursor().index(), 12, "Reverse order is the last row");
}

#[test]
fn the_user_menu_opens_on_the_scope_in_force() {
    let owners = vec![Owner::Named("alice".into()), Owner::Named("bob".into())];
    let all = TaskMenu::user(owners.clone(), &UserScope::All);
    assert_eq!((all.cursor().index(), all.row_count()), (0, 3));
    let bob = TaskMenu::user(owners.clone(), &UserScope::Only(Owner::Named("bob".into())));
    assert_eq!(bob.cursor().index(), 2);
    let gone = TaskMenu::user(owners, &UserScope::Only(Owner::Uid(4_000_000_000)));
    assert_eq!(
        gone.cursor().index(),
        0,
        "an owner no longer listed falls back to All"
    );
}

#[cfg(unix)]
#[test]
fn the_signal_menu_opens_on_sigterm() {
    let menu = TaskMenu::signal(vim()).expect("Unix offers signals");
    assert_eq!(menu.cursor().index(), 0);
    assert_eq!(menu.row_count(), Signal::OFFERED.len());
    assert_eq!(Signal::OFFERED.first(), Some(&Signal::Terminate));
}

#[cfg(unix)]
#[test]
fn terminate_or_kill_focuses_terminate_and_wraps() {
    let mut confirm = KillConfirm::new(vim(), KillOffer::TerminateOrKill);
    assert_eq!(confirm.targets().targets().len(), 1);
    assert_eq!(confirm.offer(), KillOffer::TerminateOrKill);
    assert_eq!(
        confirm.buttons(),
        [
            ConfirmButton::Terminate,
            ConfirmButton::Kill,
            ConfirmButton::Cancel
        ]
    );
    assert_eq!(confirm.focused(), ConfirmButton::Terminate);
    confirm.focus_next();
    assert_eq!(confirm.signal(confirm.focused()), Some(Signal::Kill));
    confirm.focus_next();
    confirm.focus_next();
    assert_eq!(confirm.focused(), ConfirmButton::Terminate, "wrapped");
    confirm.focus_previous();
    assert_eq!(confirm.focused(), ConfirmButton::Cancel);
    assert_eq!(confirm.signal(ConfirmButton::Cancel), None);
    assert_eq!(
        confirm.signal(ConfirmButton::Send),
        None,
        "not on this dialog"
    );
}

#[test]
fn a_chosen_signal_is_sent_by_the_send_button_only() {
    let mut confirm = KillConfirm::new(vim(), KillOffer::Send(Signal::Hangup));
    assert_eq!(
        confirm.buttons(),
        [ConfirmButton::Send, ConfirmButton::Cancel]
    );
    assert_eq!(confirm.signal(ConfirmButton::Send), Some(Signal::Hangup));
    assert_eq!(confirm.signal(ConfirmButton::Kill), None);
    confirm.focus_on(ConfirmButton::Kill);
    assert_eq!(
        confirm.focused(),
        ConfirmButton::Send,
        "a missing button is not focused"
    );
    confirm.focus_on(ConfirmButton::Cancel);
    assert_eq!(confirm.focused(), ConfirmButton::Cancel);
}

#[test]
fn the_priority_form_steps_within_the_nice_range() {
    let targets = listing(vec![process(2, "vim")]).freeze_targets(Operation::Renice);
    let mut form = PriorityForm::new(targets, nice(18));
    assert_eq!(form.value().text(), "18");
    assert_eq!(form.targets().targets().len(), 1);
    form.step(NiceStep::Lower);
    form.step(NiceStep::Lower);
    assert_eq!(form.niceness(), Some(Niceness::MAX));
    form.value_mut().set_text(" -20 ");
    form.step(NiceStep::Raise);
    assert_eq!(form.niceness(), Some(Niceness::MIN));
    form.value_mut().set_text("-21");
    assert_eq!(form.niceness(), None, "Set is disabled");
    form.step(NiceStep::Lower);
    assert_eq!(
        form.value().text(),
        "-21",
        "a value that is not one is left to fix"
    );
}

#[test]
fn the_priority_form_focus_cycles_both_ways() {
    let mut form = PriorityForm::new(FrozenTargets::default(), nice(0));
    assert_eq!(form.focus(), FormFocus::Field);
    for expected in [FormFocus::Set, FormFocus::Cancel, FormFocus::Field] {
        form.focus_next();
        assert_eq!(form.focus(), expected);
    }
    for expected in [FormFocus::Cancel, FormFocus::Set, FormFocus::Field] {
        form.focus_previous();
        assert_eq!(form.focus(), expected);
    }
}

#[test]
fn the_bars_keep_their_text_and_the_search_its_result() {
    let mut filter = TextBar::new("fire");
    filter.line_mut().insert_char('|');
    assert_eq!(filter.line().text(), "fire|");
    let mut search = SearchBar::default();
    search.line_mut().insert_str("pyth");
    assert_eq!(search.found(), Found::Match);
    search.set_found(Found::NoMatch);
    assert_eq!(
        (search.line().text(), search.found()),
        ("pyth", Found::NoMatch)
    );
    assert_eq!(Modal::default(), Modal::Browse);
}

#[test]
fn a_sent_signal_and_a_renice_read_as_information() {
    let sent = Notice::Sent {
        signal: Signal::Terminate,
        count: 2,
    };
    assert_eq!(sent.to_string(), "Sent SIGTERM to 2 processes.");
    let reniced = Notice::Reniced {
        count: 1,
        nice: nice(5),
    };
    assert_eq!(reniced.to_string(), "Nice set to 5 for 1 process.");
    for notice in [sent, reniced] {
        assert_eq!(notice.severity(), Severity::Information);
    }
}

#[test]
fn marks_are_information_and_a_refusal_is_a_warning() {
    let marked = Notice::Marked {
        changed: 3,
        total: 5,
    };
    assert_eq!(
        marked.to_string(),
        "Marks changed on 3 processes; 5 marked."
    );
    assert_eq!(marked.severity(), Severity::Information);
    let refused = Notice::Refused(ControlRefusal::KernelThread);
    assert_eq!(refused.to_string(), "Kernel threads ignore signals.");
    assert_eq!(refused.severity(), Severity::Warning);
}

#[test]
fn a_failure_keeps_its_target_and_error() {
    let target = vim().targets()[0].clone();
    let failure = Failure::new(target.clone(), Attempt::Signal, ControlError::Gone);
    assert_eq!(failure.target(), &target);
    assert_eq!(failure.attempt(), Attempt::Signal);
    assert_eq!(failure.error(), ControlError::Gone);
}
