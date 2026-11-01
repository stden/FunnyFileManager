//! Purpose: a worker whose results nobody reads stops taking requests.

use super::*;
use crate::models::WslOperation;
use std::thread;

struct Refusing;

impl WslBackend for Refusing {
    fn execute(&self, _: &WslOperation) -> Result<WslOutput, String> {
        Err("no distribution in test".to_owned())
    }
}

#[test]
fn a_worker_whose_results_are_dropped_stops_taking_requests() {
    let Worker {
        requests, results, ..
    } = spawn(Arc::new(Refusing)).expect("worker thread");
    drop(results);
    let deadline = Instant::now() + Duration::from_secs(5);

    while requests
        .send(WslRequest {
            id: WslRequestId::new(1),
            operation: WslOperation::Refresh,
        })
        .is_ok()
    {
        assert!(Instant::now() < deadline, "the worker kept taking requests");
        thread::sleep(Duration::from_millis(1));
    }
}
