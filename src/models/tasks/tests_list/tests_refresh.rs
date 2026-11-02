//! Purpose: unit tests for the list's refresh: the first sample and the warm-up,
//! failures with and without a table, and the overdue, stalled and lost-worker marks.

use super::*;

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
