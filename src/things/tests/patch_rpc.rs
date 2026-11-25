//! Unit tests: loopback GraphQL execution, polling, public wrappers, and status checks.

use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::json;

use super::patch::{os_machine, spawn_json_server};
use crate::things::{
    build_plan, execute_plan, is_success_status, is_terminal_status,
    patch::{execute_plan_with, poll_group_with, start_wave_with},
    poll_group, start_wave, Config, ExecutionInfo, PatchPlan, PatchReport, PatchScope, Status,
    ThingsError,
};

fn loopback_config(url: String) -> Config {
    let mut cfg = Config::from_values(
        Some("https://things.example.com/graphql".to_owned()),
        Some("tok".to_owned()),
    )
    .expect("cfg");
    cfg.set_base_for_test(url);
    cfg
}

fn single_wave_plan() -> PatchPlan {
    build_plan(
        &[os_machine(
            "web-01",
            "Red Hat Enterprise Linux 9.8",
            Status::Online,
        )],
        PatchScope::All,
        true,
        "test",
    )
}

#[test]
fn start_wave_poll_group_and_execute_plan_handle_loopback_graphql() {
    let start_json = |id| {
        json!({ "data": { "bulkStartWorkflowExecutions": { "executionGroup": {
            "id": id, "status": "PENDING",
            "executions": { "edges": [{ "node": { "id": "ex-1", "status": "PENDING" } }] }
        } } } })
    };
    let poll_json = json!({
        "data": { "workflows": { "executionGroup": { "id": "grp-1", "status": "FINISHED" } } }
    });
    let (url, server) =
        spawn_json_server(vec![start_json("grp-1"), poll_json, start_json("grp-2")]);
    let cfg = loopback_config(url);
    let plan = single_wave_plan();
    let no_proxy = || Client::builder().no_proxy().timeout(Duration::from_secs(3));
    assert_eq!(
        start_wave_with(no_proxy(), &cfg, &plan.waves[0], true, "test")
            .expect("s")
            .id,
        "grp-1"
    );
    let polled = poll_group_with(no_proxy(), &cfg, "grp-1").expect("poll");
    assert_eq!(polled.executions.len(), 0);
    assert_eq!(
        execute_plan_with(no_proxy, &cfg, &plan)
            .expect("exec")
            .summary(),
        "1 wave(s), 1 execution(s)"
    );
    assert_eq!(server.join().expect("server").len(), 3);
}

#[test]
fn public_wrappers_reject_closed_loopback() {
    let plan = single_wave_plan();
    let _env = super::EnvVarsGuard::set_all(&[("NO_PROXY", "*")]);
    let local = loopback_config("https://127.0.0.1:0/graphql".to_owned());
    assert_eq!(
        start_wave(&local, &plan.waves[0], true, "test").err(),
        Some(ThingsError::Request)
    );
    assert_eq!(
        poll_group(&local, "grp-1").err(),
        Some(ThingsError::Request)
    );
    assert_eq!(
        execute_plan(&local, &plan).err(),
        Some(ThingsError::Request)
    );
    let empty = PatchPlan {
        waves: Vec::new(),
        skipped: Vec::new(),
        reboot: true,
        comment: String::new(),
    };
    assert_eq!(
        execute_plan(&local, &empty).err(),
        Some(ThingsError::Response)
    );
}

#[test]
fn workflow_status_helpers_and_types_work() {
    assert!(is_terminal_status("FINISHED") && is_terminal_status("CANCELED"));
    assert!(!is_terminal_status("RUNNING"));
    assert!(is_success_status("FINISHED") && !is_success_status("FAILED"));
    let exec = ExecutionInfo {
        id: "x".to_owned(),
        status: "OK".to_owned(),
    };
    let report = PatchReport { groups: Vec::new() };
    assert_eq!(exec.id, "x");
    assert_eq!(report.summary(), "0 wave(s), 0 execution(s)");
}
