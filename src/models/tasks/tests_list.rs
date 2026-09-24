//! Purpose: Tests for the task list as a whole: applying samples and
//! failures, the warm-up, health marks, the view switches, a renice
//! recorded, and the viewport clamp `App` applies on resize.

use std::path::PathBuf;
use std::time::Instant;

use super::tests_support::{child_of, key, listing, one_core, pid, process, resample, shown, OWN};
use super::{
    Arrangement, CommandView, Cursor, Health, KernelThreads, Niceness, Owner, Priority,
    ProcessKind, ProcessSample, SampleError, Schedule, SortKey, SystemPart, Table, TaskList,
    TaskSession, UserScope, OVERDUE_AFTER, REFRESH_INTERVAL, STALL_LIMIT, WARM_UP,
};
use crate::models::{App, Overlay, Panel, SortOrder, Viewport};
use crate::prompt::Privilege;

fn owned_by(owner: &str, raw: u32, name: &str) -> ProcessSample {
    ProcessSample {
        owner: Owner::Named(owner.to_owned()),
        ..process(raw, name)
    }
}

fn unreadable() -> SampleError {
    SampleError::Unreadable {
        part: SystemPart::ProcessList,
        kind: std::io::ErrorKind::PermissionDenied,
    }
}

#[test]
fn a_new_list_waits_for_its_first_sample_with_one_due() {
    let list = TaskList::new(TaskSession::new(3), pid(OWN), Privilege::User);
    assert_eq!(list.session(), TaskSession::new(3));
    assert!(matches!(list.table(), Table::Waiting));
    assert!(list.snapshot().is_none() && list.health().is_none());
    assert!(list.rows().is_empty() && list.view().is_empty());
    assert_eq!(list.cursor(), None);
    assert_eq!(list.refresh().schedule(), Schedule::Due);
    assert_eq!(list.cores(), 0);
}

#[test]
fn the_first_snapshot_warms_up_and_the_next_waits_the_interval() {
    let start = Instant::now();
    let mut list = TaskList::new(TaskSession::new(1), pid(OWN), Privilege::User);
    list.take_request(start);
    list.apply_sample(Ok(one_core(vec![process(1, "init")])), start);
    assert_eq!(list.refresh().schedule(), Schedule::At(start + WARM_UP));
    assert_eq!(list.health(), Some(Health::Fresh));
    assert_eq!(list.cores(), 1);

    let later = start + WARM_UP;
    list.take_request(later);
    list.apply_sample(Ok(one_core(vec![process(1, "init")])), later);
    assert_eq!(
        list.refresh().schedule(),
        Schedule::At(later + REFRESH_INTERVAL)
    );
}

#[test]
fn a_failure_without_a_table_is_shown_and_retried_at_the_interval() {
    let start = Instant::now();
    let mut list = TaskList::new(TaskSession::new(1), pid(OWN), Privilege::User);
    list.take_request(start);
    list.apply_sample(Err(unreadable()), start);
    assert!(matches!(list.table(), Table::Failed(error) if *error == unreadable()));
    assert_eq!(
        list.refresh().schedule(),
        Schedule::At(start + REFRESH_INTERVAL)
    );

    let later = start + REFRESH_INTERVAL;
    list.take_request(later);
    list.apply_sample(Ok(one_core(vec![process(1, "init")])), later);
    assert_eq!(list.refresh().schedule(), Schedule::At(later + WARM_UP));
}

#[test]
fn a_failure_after_a_table_keeps_the_table_and_marks_it() {
    let mut list = listing(vec![process(1, "init")]);
    list.apply_sample(Err(unreadable()), Instant::now());
    assert_eq!(list.health(), Some(Health::Failed(unreadable())));
    assert_eq!(shown(&list), [1]);
}

#[test]
fn an_overdue_mark_never_hides_a_worse_one() {
    let mut list = listing(vec![process(1, "init")]);
    list.mark_overdue();
    assert_eq!(list.health(), Some(Health::Overdue));
    list.mark_stalled();
    assert_eq!(list.health(), Some(Health::Stalled));
    assert!(
        matches!(list.refresh().schedule(), Schedule::At(_)),
        "nothing was in flight, so the next sample keeps its time"
    );
    list.mark_overdue();
    assert_eq!(list.health(), Some(Health::Stalled));
    resample(&mut list, vec![process(1, "init")]);
    assert_eq!(list.health(), Some(Health::Fresh));
}

#[test]
fn a_late_result_discarded_by_a_hold_leaves_the_table_held_not_stale() {
    let start = Instant::now();
    let mut list = TaskList::new(TaskSession::new(1), pid(OWN), Privilege::User);
    list.take_request(start);
    list.apply_sample(Ok(one_core(vec![process(1, "init")])), start);
    let due = start + WARM_UP;
    list.take_request(due);
    list.toggle_hold();
    list.mark_overdue();
    assert_eq!(list.health(), Some(Health::Overdue));

    list.apply_sample(Ok(one_core(vec![process(2, "late")])), due + OVERDUE_AFTER);
    assert_eq!(shown(&list), [1], "held: the late result is not shown");
    assert_eq!(
        list.health(),
        Some(Health::Fresh),
        "nothing is in flight any more, so nothing is late"
    );

    list.refresh_now();
    let now = due + STALL_LIMIT;
    list.take_request(now);
    list.apply_sample(Ok(one_core(vec![process(3, "asked")])), now);
    assert_eq!(shown(&list), [3], "Ctrl+R while held shows one sample");
    assert_eq!(list.refresh().schedule(), Schedule::Held);
}

#[test]
fn a_first_sample_that_stalls_says_so_and_is_due_again() {
    let start = Instant::now();
    let mut list = TaskList::new(TaskSession::new(1), pid(OWN), Privilege::User);
    list.take_request(start);
    list.mark_overdue();
    assert!(
        matches!(list.table(), Table::Waiting),
        "no table yet, so nothing is stale"
    );
    list.mark_stalled();
    assert_eq!(list.refresh().schedule(), Schedule::Due);
    assert!(
        matches!(list.table(), Table::Failed(SampleError::WorkerStalled)),
        "a reader stuck before its first table is not left looking like one still starting"
    );

    let later = start + STALL_LIMIT;
    list.take_request(later);
    list.apply_sample(Ok(one_core(vec![process(1, "init")])), later);
    assert_eq!(list.health(), Some(Health::Fresh));
    assert_eq!(list.refresh().schedule(), Schedule::At(later + WARM_UP));
}

#[test]
fn a_lost_worker_stops_the_session_and_says_why() {
    let mut list = listing(vec![process(1, "init")]);
    list.worker_lost(SampleError::WorkerStopped);
    assert_eq!(
        list.health(),
        Some(Health::Failed(SampleError::WorkerStopped))
    );
    assert_eq!(list.refresh().schedule(), Schedule::Stopped);
    assert_eq!(list.take_request(Instant::now()), None);
}

#[test]
fn the_cursor_stays_on_its_process_across_a_sample_and_a_resort() {
    let mut list = listing(vec![
        process(1, "init"),
        process(2, "sshd"),
        process(3, "vim"),
    ]);
    list.select_index(2, 10);
    assert_eq!(list.cursor().map(Cursor::key), Some(key(3, 3)));

    resample(
        &mut list,
        vec![process(16, "new"), process(3, "vim"), process(1, "init")],
    );
    assert_eq!(shown(&list), [1, 3, 16]);
    assert_eq!(
        list.cursor().map(|at| (at.key(), at.index())),
        Some((key(3, 3), 1))
    );

    list.choose_sort(SortKey::Pid);
    list.choose_sort(SortKey::Pid);
    assert_eq!(shown(&list), [16, 3, 1]);
    assert_eq!(list.cursor().map(Cursor::index), Some(1));
}

#[test]
fn choosing_a_sort_key_leaves_the_tree_and_the_same_key_reverses() {
    let mut list = listing(vec![process(1, "init"), child_of(1, process(2, "sshd"))]);
    list.toggle_tree();
    assert_eq!(list.options().arrangement, Arrangement::Tree);
    list.choose_sort(SortKey::Name);
    assert_eq!(list.options().arrangement, Arrangement::Flat);
    assert_eq!(shown(&list), [1, 2]);
    list.choose_sort(SortKey::Name);
    assert_eq!(list.options().sort.order(), SortOrder::Reversed);
    assert_eq!(shown(&list), [2, 1]);
    list.invert_order();
    assert_eq!(shown(&list), [1, 2]);
}

#[test]
fn the_filter_the_scope_and_kernel_threads_rebuild_the_view() {
    let kthreadd = ProcessSample {
        kind: ProcessKind::KernelThread,
        ..process(2, "kthreadd")
    };
    let mut list = listing(vec![
        owned_by("root", 1, "init"),
        kthreadd,
        process(3, "firefox"),
        owned_by("alice", 4, "chromium"),
    ]);
    assert_eq!(shown(&list), [1, 3, 4]);
    list.toggle_kernel_threads();
    assert_eq!(list.options().kernel_threads, KernelThreads::Shown);
    assert_eq!(shown(&list), [1, 2, 3, 4]);
    list.set_filter("FIRE|chrom");
    assert_eq!(list.options().filter.line(), "FIRE|chrom");
    assert_eq!(shown(&list), [3, 4]);
    list.set_scope(UserScope::Only(Owner::Named("bob".to_owned())));
    assert_eq!(shown(&list), [3]);
    list.set_filter("");
    list.set_scope(UserScope::All);
    assert_eq!(shown(&list), [1, 2, 3, 4]);
}

#[test]
fn the_owners_are_listed_once_each_in_order() {
    let list = listing(vec![
        owned_by("root", 1, "init"),
        process(2, "shell"),
        owned_by("alice", 3, "editor"),
        process(4, "vim"),
    ]);
    let names = ["alice", "bob", "root"].map(|name| Owner::Named(name.to_owned()));
    assert_eq!(list.owners(), names);
}

#[test]
fn the_last_column_switches_and_scrolls_back_to_its_start() {
    let mut list = listing(vec![process(1, "init")]);
    list.pan_right(20);
    list.pan_right(1);
    list.pan_left(5);
    assert_eq!(list.command_scroll().cells(), 16);
    list.toggle_command_view();
    assert_eq!(list.command_view(), CommandView::Name);
    assert_eq!(list.command_scroll().cells(), 0);
}

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
    let notice = super::Notice::Refused(super::ControlRefusal::NoTarget);
    list.set_notice(notice);
    assert_eq!(list.notice(), Some(notice));
    list.clear_notice();
    assert_eq!(list.notice(), None);
    list.set_modal(super::Modal::Info(key(1, 1)));
    assert!(matches!(list.modal(), super::Modal::Info(_)));
    *list.modal_mut() = super::Modal::Help(crate::scroll::ScrollTop::TOP);
    list.close_modal();
    assert_eq!(*list.modal(), super::Modal::Browse);
}

#[test]
fn a_shorter_terminal_keeps_the_cursor_on_screen() {
    let processes = (1..=40).map(|raw| process(raw, "worker")).collect();
    let mut list = listing(processes);
    let tall = Viewport::new(30);
    assert_eq!(
        list.page_for(tall),
        21,
        "one core: no bars, nine rows of chrome"
    );
    list.move_cursor(super::ListMotion::End, list.page_for(tall));
    assert_eq!(list.offset(), 40 - 21);

    let short = Viewport::new(24);
    let mut direct = list.clone();
    direct.clamp_to(short);
    assert_eq!(direct.offset(), 40 - 15);

    let mut app = App::with_panels(
        Panel::from_entries(PathBuf::from("/srv/x"), Vec::new()),
        Panel::from_entries(PathBuf::from("/srv/x"), Vec::new()),
    );
    app.set_overlay(Overlay::Tasks(Box::new(list)));
    app.set_viewport(short);
    let Overlay::Tasks(list) = &app.overlay else {
        panic!("the task list stays open");
    };
    assert_eq!(list.page_for(short), 15);
    assert_eq!(list.offset(), 40 - 15, "the last row is on the bottom row");
    assert_eq!(list.header_shape(short).page(), 15);
}
