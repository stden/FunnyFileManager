//! Purpose: unit tests for what the list shows after an action: a recorded renice
//! until the next sample, and a notice and a modal set and cleared.

use super::*;

#[test]
fn a_recorded_renice_shows_until_the_next_sample() {
    let mut list = listing(vec![process(1, "init"), process(2, "vim")]);
    let five = Niceness::try_from(5).expect("a nice value");
    list.record_nice(key(1, 1), five);
    let recorded = list.row(key(1, 1)).map(|row| row.sample().priority);
    assert!(matches!(recorded, Some(Priority::Unix { nice, .. }) if nice == five));
    list.choose_sort(SortKey::Priority);
    assert_eq!(shown(&list), [2, 1], "a nicer process sorts later");
    assert!(list.row(key(9, 9)).is_none());
}

#[test]
fn a_notice_and_a_modal_are_set_and_cleared() {
    let mut list = listing(vec![process(1, "init")]);
    let notice = Notice::Refused(ControlRefusal::NoTarget);
    list.set_notice(notice);
    assert_eq!(list.notice(), Some(notice));
    list.clear_notice();
    assert_eq!(list.notice(), None);
    list.set_modal(Modal::Info(key(1, 1)));
    assert!(matches!(list.modal(), Modal::Info(_)));
    *list.modal_mut() = Modal::Help(crate::scroll::ScrollTop::TOP);
    list.close_modal();
    assert_eq!(*list.modal(), Modal::Browse);
}
