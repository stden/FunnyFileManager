//! Input-to-frame timing excludes external execution and never persists typed payloads.

use super::{BehaviorWorld, Case};
use crate::application::timing::Pending;
use crossterm::event::Event;
use cucumber::{then, when};
use std::time::{Duration, Instant};

pub(super) struct TimingSample {
    expected: Duration,
    paused: Duration,
}

#[when("an external fixture command runs during a measured action")]
fn external_wait(world: &mut BehaviorWorld) {
    let case = world.case();
    let started = Instant::now();
    let pending = Pending::start(&case.app, &Event::Paste("fixture payload".into()), started);
    let before = crate::performance::paused_total();
    crate::ops::execute_command(
        &mut case.app,
        "python -c \"import time; time.sleep(0.02); print('fixture payload')\"",
    )
    .unwrap();
    let paused = crate::performance::paused_total().saturating_sub(before);
    let rendering = Instant::now();
    let rendered = Duration::from_millis(2);
    pending.finish(&mut case.app.performance, rendering, rendering + rendered);
    case.timing = Some(TimingSample {
        expected: rendering.duration_since(started).saturating_sub(paused) + rendered,
        paused,
    });
}

pub(super) fn saved(case: &Case) -> serde_json::Value {
    let file = case.root.path().join("statistics.json");
    case.app.performance.save_to(&file).unwrap();
    serde_json::from_str(&std::fs::read_to_string(file).unwrap()).unwrap()
}

#[then("the external wait is excluded from recorded input latency")]
fn excluded(world: &mut BehaviorWorld) {
    let case = world.case();
    let report = saved(case);
    let timing = case.timing.as_ref().unwrap();
    assert!(timing.paused >= Duration::from_millis(20));
    let measured = report["actions"][0]["total_ms"].as_f64().unwrap();
    let expected_ms = timing.expected.as_secs_f64() * 1000.0;
    assert!((measured - expected_ms).abs() < 0.001);
    assert_eq!(report["actions"][0]["render_ms"], 2.0);
}

#[when("I paste a private fixture command into the command line")]
fn private_command(world: &mut BehaviorWorld) {
    let case = world.case();
    case.press(":");
    case.queued.extend([
        Event::Paste("echo fixture-private-payload".into()),
        Case::key("Esc"),
    ]);
    case.batch();
}

#[then("saved statistics identify the action without its contents or paths")]
fn redacted(world: &mut BehaviorWorld) {
    let case = world.case();
    let text = saved(case).to_string();
    assert!(text.contains("CommandLine/Paste"));
    assert!(!text.contains("fixture-private-payload"));
    assert!(!text.contains(&case.root.path().to_string_lossy().to_string()));
}
