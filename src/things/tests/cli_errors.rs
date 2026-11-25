//! Unit tests: CLI failure paths (empty selection, failed execution group, and timeout).

use serde_json::json;

use super::cli::{no_proxy_client, things_reply};
use super::patch::spawn_json_server;
use crate::things::cli::{options::CliOptions, run_with};
use crate::things::Config;

fn server_cfg(replies: Vec<serde_json::Value>) -> (Config, std::thread::JoinHandle<Vec<String>>) {
    let (url, server) = spawn_json_server(replies);
    let mut cfg = Config::from_values(
        Some("https://things.example.com/graphql".to_owned()),
        Some("tok".to_owned()),
    )
    .expect("cfg")
    .with_root_id(Some("r1".to_owned()));
    cfg.set_base_for_test(url);
    (cfg, server)
}

fn start_reply(group_id: &str) -> serde_json::Value {
    json!({
        "data": { "bulkStartWorkflowExecutions": { "executionGroup": {
            "id": group_id, "status": "PENDING", "executions": { "edges": [] }
        } } }
    })
}

fn poll_reply(group_id: &str, status: &str) -> serde_json::Value {
    json!({ "data": { "workflows": { "executionGroup": { "id": group_id, "status": status } } } })
}

#[test]
fn cli_run_with_reports_empty_selection() {
    let (cfg, server) = server_cfg(vec![things_reply(&["srv-01"])]);
    let opts =
        CliOptions::parse(&["--hosts".into(), "missing".into(), "--execute".into()]).expect("none");
    assert!(run_with(no_proxy_client, &cfg, &opts, &mut Vec::new()).is_err());
    assert_eq!(server.join().expect("server").len(), 1);
}

#[test]
fn cli_run_with_reports_failed_group_and_timeout() {
    let (cfg, server) = server_cfg(vec![
        things_reply(&["srv-01"]),
        start_reply("g1"),
        poll_reply("g1", "FAILED"),
        things_reply(&["srv-01"]),
        start_reply("g2"),
        poll_reply("g2", "RUNNING"),
    ]);
    let fail_opts = CliOptions::parse(&[
        "--execute".into(),
        "--wait".into(),
        "--poll-secs".into(),
        "0".into(),
    ])
    .expect("fail");
    assert!(run_with(no_proxy_client, &cfg, &fail_opts, &mut Vec::new()).is_err());

    let timeout_opts = CliOptions::parse(&[
        "--execute".into(),
        "--wait".into(),
        "--poll-secs".into(),
        "0".into(),
        "--timeout-secs".into(),
        "0".into(),
    ])
    .expect("timeout");
    assert!(run_with(no_proxy_client, &cfg, &timeout_opts, &mut Vec::new()).is_err());
    assert_eq!(server.join().expect("server").len(), 6);
}

#[cfg(unix)]
fn non_utf8_arg() -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt;
    std::ffi::OsString::from_vec(vec![0xff])
}

#[cfg(windows)]
fn non_utf8_arg() -> std::ffi::OsString {
    use std::os::windows::ffi::OsStringExt;
    std::ffi::OsString::from_wide(&[0xD800])
}

struct FailAfterSummaryWriter(Vec<u8>);

impl std::io::Write for FailAfterSummaryWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.extend_from_slice(buf);
        if String::from_utf8_lossy(&self.0).contains("Started wave") {
            return Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
        }
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn things_with_offline() -> serde_json::Value {
    json!({ "data": { "aggregate": { "things": { "results": [
        { "id": "id-1", "identifierValue": "srv-01", "lifecycle": "ACTIVE",
          "type": { "nameUri": "vm" }, "attributes": { "edges": [
            { "node": { "nameUri": "os_name", "value": { "value": "RHEL 9" } } },
            { "node": { "nameUri": "vmware_power_state", "value": { "value": "on" } } }
        ] } },
        { "id": "id-2", "identifierValue": "off-01", "lifecycle": "ACTIVE",
          "type": { "nameUri": "vm" }, "attributes": { "edges": [
            { "node": { "nameUri": "os_name", "value": { "value": "RHEL 9" } } },
            { "node": { "nameUri": "vmware_power_state", "value": { "value": "off" } } }
        ] } }
    ] } } } })
}

#[test]
fn cli_execute_without_wait_logs_skipped_and_handles_wave_write_errors() {
    let (cfg, server) = server_cfg(vec![
        things_with_offline(),
        start_reply("g1"),
        things_reply(&["srv-01"]),
        start_reply("g2"),
    ]);
    let opts = CliOptions {
        hosts: vec!["   ".to_owned(), "srv-01".to_owned(), "off-01".to_owned()],
        execute: true,
        wait: false,
        ..CliOptions::default()
    };
    let mut out = Vec::new();
    run_with(no_proxy_client, &cfg, &opts, &mut out).expect("execute without wait");
    assert!(String::from_utf8_lossy(&out).contains("Skipped: off-01: offline"));
    let mut failing = FailAfterSummaryWriter(Vec::new());
    assert!(run_with(no_proxy_client, &cfg, &opts, &mut failing).is_err());
    assert_eq!(server.join().expect("server").len(), 4);
}

fn assert_wait_polls_finish(group_id: &str, interval: std::time::Duration) {
    let (cfg, server) = server_cfg(vec![
        poll_reply(group_id, "RUNNING"),
        poll_reply(group_id, "FINISHED"),
    ]);
    let mut out = Vec::new();
    crate::things::cli::wait_for_group_with(
        no_proxy_client,
        &cfg,
        group_id,
        interval,
        std::time::Duration::from_secs(5),
        &mut out,
    )
    .expect("wait succeeds");
    assert!(String::from_utf8_lossy(&out).contains("status=FINISHED"));
    assert_eq!(server.join().expect("server").len(), 2);
}

#[test]
fn cli_wait_for_group_with_sleeps_between_polls_and_finishes() {
    assert_wait_polls_finish("g3", std::time::Duration::from_millis(1));
    assert_wait_polls_finish("g4", std::time::Duration::ZERO);
}

#[test]
fn cli_options_reject_non_utf8_and_resolve_env_fallback_or_invalid_config() {
    use crate::things::cli::options::resolve_config;
    use crate::things::{ROOT_ID_VARIABLE, TOKEN_VARIABLE, URL_VARIABLE};
    assert!(CliOptions::parse(&["--root".into(), non_utf8_arg()]).is_err());
    {
        let _env = super::EnvVarsGuard::set_all(&[
            (URL_VARIABLE, "https://things.example.com/graphql"),
            (TOKEN_VARIABLE, "tok"),
            (ROOT_ID_VARIABLE, "env-root"),
        ]);
        let cfg = resolve_config(&CliOptions::default()).expect("env fallback");
        assert_eq!(cfg.root_id(), Some("env-root"));
    }
    let _empty = super::EnvVarsGuard::set_all(&[(URL_VARIABLE, ""), (TOKEN_VARIABLE, "")]);
    assert!(resolve_config(&CliOptions::default()).is_err());
}
