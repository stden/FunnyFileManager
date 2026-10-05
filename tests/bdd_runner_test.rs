//! Purpose: Pins how the Cucumber runner reports a failed step and runs a suite.
//!
//! The BDD suites are `harness = false` targets, so the runner's own behaviour
//! is checked here, under libtest.

#[path = "support/bdd_runner.rs"]
mod bdd_runner;

use bdd_runner::{Context as _, StepResult};
use std::io;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[test]
fn a_failed_result_reads_like_the_expect_it_replaced() {
    let failure: io::Result<()> = Err(io::Error::new(io::ErrorKind::NotFound, "gone"));
    assert_eq!(
        failure.context("write file"),
        Err(r#"write file: Custom { kind: NotFound, error: "gone" }"#.to_owned())
    );

    let slot: Result<u8, &str> = Err("slot out of range");
    assert_eq!(
        slot.context("slot"),
        Err(r#"slot: "slot out of range""#.to_owned())
    );
}

#[test]
fn a_missing_value_fails_with_the_expect_message_alone() {
    let app: StepResult<u8> = None.context("no app");
    assert_eq!(app, Err("no app".to_owned()));
}

#[test]
fn present_values_pass_through() {
    let loaded: io::Result<u8> = Ok(7);
    assert_eq!(loaded.context("load"), Ok(7));
    assert_eq!(Some(7).context("load"), Ok(7));
}

#[test]
fn run_returns_once_the_suite_has_finished() -> io::Result<()> {
    let finished = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&finished);
    bdd_runner::run(move || flag.store(true, Ordering::SeqCst))?;
    assert!(finished.load(Ordering::SeqCst));
    Ok(())
}

#[test]
#[should_panic(expected = "a scenario failed")]
fn a_panic_in_the_suite_reaches_the_caller() {
    drop(bdd_runner::run(|| panic!("a scenario failed")));
}
