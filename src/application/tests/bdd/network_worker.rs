//! Background diagnostics use an injected sampler and prober, without sockets.

use super::{samples, BehaviorWorld};
use crate::{
    application::task_jobs::TaskJobs,
    models::tasks::{NetHealth, NetProber, ProbeOutcome, ProbeTarget},
    models::{
        ControlError, PriorityChange, ProcessHost, ProcessKey, ProcessReader, Sample, SampleError,
        Signal,
    },
    test_support::processes::FixtureHost,
};
use cucumber::{given, then, when};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

struct NetworkHost(FixtureHost);
struct NetworkReader(u64);
struct NetworkProber;

impl ProcessReader for NetworkReader {
    fn sample(&mut self) -> Result<Sample, SampleError> {
        self.0 += 1;
        Ok(samples::network(self.0, self.0 * 1_024, self.0 * 256))
    }
}

impl NetProber for NetworkProber {
    fn probe(&mut self, targets: &[ProbeTarget]) -> Vec<(ProbeTarget, ProbeOutcome)> {
        targets
            .iter()
            .map(|&target| (target, ProbeOutcome::Ok(Duration::from_millis(8))))
            .collect()
    }
}

impl ProcessHost for NetworkHost {
    fn reader(&self) -> Result<Box<dyn ProcessReader>, SampleError> {
        Ok(Box::new(NetworkReader(0)))
    }
    fn signal(&self, key: ProcessKey, signal: Signal) -> Result<(), ControlError> {
        self.0.signal(key, signal)
    }
    fn set_priority(&self, key: ProcessKey, change: PriorityChange) -> Result<(), ControlError> {
        self.0.set_priority(key, change)
    }
    fn net_prober(&self) -> Option<Box<dyn NetProber>> {
        Some(Box::new(NetworkProber))
    }
}

#[given("a fixture network worker is installed")]
fn installed(world: &mut BehaviorWorld) {
    let case = world.case();
    case.app
        .task_host
        .install(Arc::new(NetworkHost(case.host.clone())));
    case.press("Ctrl+W");
}

#[when("background task sampling and diagnostic probes complete")]
fn completed(world: &mut BehaviorWorld) {
    let case = world.case();
    let mut jobs = TaskJobs::default();
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        jobs.pump(&mut case.app);
        if case
            .app
            .task_list_mut()
            .unwrap()
            .net_diagnostics()
            .overall_health()
            == NetHealth::Healthy
        {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "fixture background diagnostics timed out"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
    case.paint();
}

#[then("background probe latency is recorded without probe payloads")]
fn recorded(world: &mut BehaviorWorld) {
    let report = world.case().app.performance.report().join("\n");
    assert!(report.contains("Background/Tasks/NetProbe"));
    assert!(!report.contains("192.0.2."));
}
