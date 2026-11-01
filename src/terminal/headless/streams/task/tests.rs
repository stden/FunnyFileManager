//! Purpose: Cleanup must tolerate an output-worker panic without a second panic.

use super::*;

#[test]
fn close_consumes_a_panicked_worker_and_can_be_repeated() {
    let mut task = Task::spawn(|_| panic!("private reader fixture panic")).unwrap();
    task.close();
    assert!(task.stop.load(Ordering::Acquire));
    assert!(task.worker.is_none());
    task.close();
    drop(task);
}
