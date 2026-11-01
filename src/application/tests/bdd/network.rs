//! Network browsing, sampling intervals and diagnostic outcomes at the model boundary.

use super::{samples, tasks, BehaviorWorld};
use crate::models::tasks::{ProbeOutcome, ProbeTarget, PROBE_INTERVAL};
use cucumber::{given, then, when};
use std::time::{Duration, Instant};

#[given("the task list has two network counter samples")]
fn sampled(world: &mut BehaviorWorld) {
    let case = world.case();
    case.press("Ctrl+W");
    tasks::apply(case, samples::network(10, 1_000_000, 500_000));
    tasks::apply(case, samples::network(11, 3_097_152, 756_000));
}

#[when(expr = "the diagnostic sweep reports {word}")]
fn diagnostics(world: &mut BehaviorWorld, outcome: String) {
    let case = world.case();
    let list = case.app.task_list_mut().unwrap();
    let now = Instant::now() + PROBE_INTERVAL;
    let targets = list
        .take_probe_request(now)
        .expect("a diagnostic sweep is due");
    let outcomes: Vec<_> = targets
        .into_iter()
        .map(|target| {
            let result = match outcome.as_str() {
                "timeout" => ProbeOutcome::Timeout,
                "slow-DNS" if matches!(target, ProbeTarget::DnsResolve { .. }) => {
                    ProbeOutcome::Ok(Duration::from_millis(320))
                }
                "healthy" | "slow-DNS" => ProbeOutcome::Ok(Duration::from_millis(8)),
                _ => panic!("unknown fixture outcome {outcome}"),
            };
            (target, result)
        })
        .collect();
    list.apply_probe_results(&outcomes, now);
    case.paint();
}

#[then("network traffic rates come from the elapsed sample interval")]
fn rates(world: &mut BehaviorWorld) {
    let list = world.case().app.task_list_mut().unwrap();
    let meter = list.snapshot().unwrap().adapters().first().unwrap();
    assert_eq!(meter.rx_rate().unwrap().bytes_per_sec(), 2_097_152);
    assert_eq!(meter.tx_rate().unwrap().bytes_per_sec(), 256_000);
}

#[then("network diagnostics are held until an explicit refresh")]
fn held(world: &mut BehaviorWorld) {
    let list = world.case().app.task_list_mut().unwrap();
    assert!(list
        .take_probe_request(Instant::now() + Duration::from_secs(30))
        .is_none());
}

#[then("an explicit diagnostic refresh is due")]
fn forced(world: &mut BehaviorWorld) {
    let list = world.case().app.task_list_mut().unwrap();
    assert!(list
        .take_probe_request(Instant::now() + Duration::from_secs(30))
        .is_some());
}
