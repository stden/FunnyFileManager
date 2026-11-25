//! Queue ordering, cancellation, deduplication and worker wake-up.

use super::*;
use std::time::Duration;

mod failures;
mod threshold;

fn batch(parent: &str, names: &[&str]) -> (PathBuf, Vec<FileName>) {
    (
        parent.into(),
        names.iter().map(|name| FileName::from(*name)).collect(),
    )
}

fn queued(pool: &Pool) -> Vec<(PathBuf, FileName)> {
    pool.shared
        .0
        .lock()
        .expect("queue")
        .tasks
        .iter()
        .cloned()
        .collect()
}

fn expected(parent: &str, name: &str) -> (PathBuf, FileName) {
    (parent.into(), FileName::from(name))
}

#[test]
fn shared_panel_batches_deduplicate_in_first_seen_order() {
    let pool = Pool::new();
    let active = [Path::new("/work"), Path::new("/work")];
    assert_eq!(
        pool.update_queue(
            active,
            [
                Some(batch("/work", &["b", "a", "b", "c", "d", "h", "i"])),
                Some(batch("/work", &["a", "c", "e", "f", "g", "i", "j"]))
            ]
        ),
        10
    );
    assert_eq!(
        queued(&pool),
        ["b", "a", "c", "d", "h", "i", "e", "f", "g", "j"]
            .into_iter()
            .map(|name| expected("/work", name))
            .collect::<Vec<_>>()
    );
}

#[test]
fn names_in_different_parents_are_distinct_and_repeated_batches_append_only_new_work() {
    let pool = Pool::new();
    let active = [Path::new("/left"), Path::new("/right")];
    let first = [
        Some(batch("/left", &["repo"])),
        Some(batch("/right", &["repo"])),
    ];
    assert_eq!(pool.update_queue(active, first), 2);
    let names = ["repo", "new", "x", "y", "z", "v", "w"];
    let second = [Some(batch("/left", &names)), Some(batch("/right", &names))];
    assert_eq!(pool.update_queue(active, second), 14);
    let mut expected_queue = vec![expected("/left", "repo"), expected("/right", "repo")];
    for parent in ["/left", "/right"] {
        expected_queue.extend(
            ["new", "x", "y", "z", "v", "w"]
                .into_iter()
                .map(|name| expected(parent, name)),
        );
    }
    assert_eq!(queued(&pool), expected_queue);
}

#[test]
fn navigation_discards_only_inactive_queued_work_and_preserves_fifo() {
    let pool = Pool::new();
    let active = [Path::new("/old"), Path::new("/kept")];
    pool.update_queue(
        active,
        [
            Some(batch("/old", &["a", "b"])),
            Some(batch("/kept", &["x", "y"])),
        ],
    );
    let next = [Path::new("/new"), Path::new("/kept")];
    assert_eq!(
        pool.update_queue(next, [Some(batch("/new", &["z"])), None]),
        3
    );
    assert_eq!(
        queued(&pool),
        vec![
            expected("/kept", "x"),
            expected("/kept", "y"),
            expected("/new", "z")
        ]
    );
}

#[test]
fn in_flight_work_counts_as_busy_but_does_not_deduplicate_queued_work() {
    let pool = Pool::new();
    let active = [Path::new("/work"), Path::new("/work")];
    assert_eq!(
        pool.update_queue(active, [Some(batch("/work", &["a"])), None]),
        1
    );
    assert_eq!(next_task(&pool.shared), Some(expected("/work", "a")));
    assert_eq!(
        pool.update_queue(active, [Some(batch("/work", &["a"])), None]),
        2
    );
    assert_eq!(queued(&pool), vec![expected("/work", "a")]);
    finish_task(&pool.shared);
    assert_eq!(pool.update_queue(active, [None, None]), 1);
}

#[test]
fn a_single_name_from_either_panel_respects_existing_queue_order() {
    let pool = Pool::new();
    let active = [Path::new("/work"), Path::new("/work")];
    pool.update_queue(active, [Some(batch("/work", &["a", "b"])), None]);
    assert_eq!(
        pool.update_queue(
            active,
            [Some(batch("/work", &[])), Some(batch("/work", &["b"]))]
        ),
        2
    );
    assert_eq!(
        pool.update_queue(active, [None, Some(batch("/work", &["c"]))]),
        3
    );
    assert_eq!(
        queued(&pool),
        vec![
            expected("/work", "a"),
            expected("/work", "b"),
            expected("/work", "c")
        ]
    );
}

#[test]
fn newly_added_work_wakes_a_waiting_worker() {
    let pool = Pool::new();
    let shared = Arc::clone(&pool.shared);
    let (started, ready) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let (lock, wake) = &*shared;
        let mut queue = lock.lock().expect("queue");
        started.send(()).expect("worker started");
        while queue.tasks.is_empty() {
            let (next, timeout) = wake
                .wait_timeout(queue, Duration::from_secs(5))
                .expect("wait");
            queue = next;
            if timeout.timed_out() && queue.tasks.is_empty() {
                return false;
            }
        }
        true
    });
    ready
        .recv_timeout(Duration::from_secs(5))
        .expect("worker ready");
    pool.update_queue(
        [Path::new("/work"), Path::new("/work")],
        [Some(batch("/work", &["a"])), None],
    );
    assert!(
        worker.join().expect("worker"),
        "new work must notify the worker"
    );
}
