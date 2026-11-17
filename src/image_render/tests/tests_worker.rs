//! Purpose: Clearing invalidates waiting work even when the result slot is poisoned.

use super::*;

/// A worker with no render thread, so only the test touches its queue.
fn waiting() -> Worker {
    let queue = Queue {
        next: None,
        generation: 0,
        stopped: false,
    };
    Worker {
        shared: Arc::new((Mutex::new(queue), Condvar::new())),
        results: Arc::new(Mutex::new(None)),
        requested: None,
        cache: VecDeque::new(),
    }
}

#[test]
fn clearing_with_a_poisoned_result_still_invalidates_waiting_work() {
    let mut worker = waiting();
    let request = key(Mode::Halfblocks, &[1, 2, 3, 255]);
    worker.request(request.clone());
    worker.cache.push_back((request, None));
    poison(&worker.results);
    worker.clear();
    assert!(worker.requested.is_none());
    assert!(worker.cache.is_empty());
    let queue = worker.shared.0.lock().expect("queue");
    assert_eq!(queue.generation, 1);
    assert!(queue.next.is_none());
    drop(queue);
}
