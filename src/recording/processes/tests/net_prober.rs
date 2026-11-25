//! Purpose: Replaying a session preserves its read-only network diagnostic provider.

use super::*;
use crate::models::tasks::{Ipv4, ProbeOutcome, ProbeTarget};
use crate::models::{NetProber, ProcessHost, ProcessKey, ProcessReader, SampleError};

struct Host;
struct Probe;

impl NetProber for Probe {
    fn probe(&mut self, targets: &[ProbeTarget]) -> Vec<(ProbeTarget, ProbeOutcome)> {
        targets
            .iter()
            .map(|target| (*target, ProbeOutcome::Timeout))
            .collect()
    }
}

impl ProcessHost for Host {
    fn reader(&self) -> Result<Box<dyn ProcessReader>, SampleError> {
        Err(SampleError::Unsupported)
    }
    fn signal(&self, _: ProcessKey, _: Signal) -> Result<(), ControlError> {
        Err(ControlError::ReadOnly)
    }
    fn set_priority(&self, _: ProcessKey, _: PriorityChange) -> Result<(), ControlError> {
        Err(ControlError::ReadOnly)
    }
    fn net_prober(&self) -> Option<Box<dyn NetProber>> {
        Some(Box::new(Probe))
    }
}

#[test]
fn replay_keeps_an_available_network_prober() {
    let dir = TempDir::new().expect("directory");
    let host = replay(&dir).process_host(Arc::new(Host));
    let target = ProbeTarget::Gateway(Ipv4::new(127, 0, 0, 1));
    assert_eq!(
        host.net_prober().expect("wrapped prober").probe(&[target]),
        [(target, ProbeOutcome::Timeout)]
    );
}
