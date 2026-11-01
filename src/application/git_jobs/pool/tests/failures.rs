//! Worker startup and poisoned-queue failure handling.

use super::super::*;
use std::time::Duration;

#[test]
fn a_poisoned_queue_takes_no_work_and_reports_no_demand() {
    let pool = Pool::new();
    let shared = Arc::clone(&pool.shared);
    let poisoner = std::thread::spawn(move || {
        let _held = shared.0.lock();
        panic!("poison the queue");
    });
    assert!(poisoner.join().is_err());

    let batch = Some((PathBuf::from("/work"), vec![FileName::from("repo")]));
    assert_eq!(
        pool.update_queue([Path::new("/work"), Path::new("/work")], [batch, None]),
        0
    );
}

#[test]
fn failed_spawn_keeps_queued_task_and_retry_delivers_it() {
    let mut pool = Pool::new();
    let parent = PathBuf::from("/work");
    let name = FileName::from("repo");
    let prober: Prober = Arc::new(|_, _| true);
    let batch = Some((parent.clone(), vec![name.clone()]));
    assert_eq!(pool.update_queue([&parent, &parent], [batch, None]), 1);

    FORCE_SPAWN_FAILURE.set(true);
    pool.grow_to(1, &prober);
    FORCE_SPAWN_FAILURE.set(false);

    assert_eq!(pool.workers, 0);
    {
        let queue = pool.shared.0.lock().expect("queue");
        assert_eq!(queue.busy, 0);
        assert_eq!(queue.tasks.front(), Some(&(parent.clone(), name.clone())));
        assert_eq!(queue.tasks.len(), 1);
    }

    pool.grow_to(1, &prober);
    let result = pool.outcomes.recv_timeout(Duration::from_secs(5));
    let state = {
        let queue = pool.shared.0.lock().expect("queue");
        (queue.tasks.len(), queue.busy)
    };
    pool.close();

    assert_eq!(pool.workers, 1);
    assert_eq!(result.expect("outcome"), (parent, name, true));
    assert_eq!(state, (0, 0));
}
