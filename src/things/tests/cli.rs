//! Unit tests: CLI parsing, config resolution, dry-run, and wave execution.

use std::ffi::OsString;
use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::json;

use super::patch::spawn_json_server;
use crate::things::cli::{
    arguments,
    options::{resolve_config, CliOptions},
    run, run_with,
};
use crate::things::{Config, ROOT_ID_VARIABLE, TOKEN_VARIABLE, URL_VARIABLE};

pub(super) fn no_proxy_client() -> reqwest::blocking::ClientBuilder {
    Client::builder().no_proxy().timeout(Duration::from_secs(3))
}

pub(super) fn things_reply(names: &[&str]) -> serde_json::Value {
    let nodes: Vec<_> = names
        .iter()
        .map(|name| {
            json!({
                "id": format!("id-{name}"),
                "identifierValue": name,
                "lifecycle": "ACTIVE",
                "type": { "nameUri": "vm" },
                "attributes": { "edges": [
                    { "node": { "nameUri": "os_name", "value": { "value": "Red Hat Enterprise Linux 9.8" } } },
                    { "node": { "nameUri": "vmware_power_state", "value": { "value": "on" } } }
                ] }
            })
        })
        .collect();
    json!({ "data": { "aggregate": { "things": { "results": nodes } } } })
}

fn graphql_cfg(url: String) -> Config {
    let mut cfg = Config::from_values(
        Some("https://things.example.com/graphql".to_owned()),
        Some("tok".to_owned()),
    )
    .expect("cfg")
    .with_root_id(Some("r1".to_owned()));
    cfg.set_base_for_test(url);
    cfg
}

#[test]
fn arguments_and_valid_cli_flags_parse() {
    assert!(arguments(&[]).is_none());
    let args = vec![
        OsString::from("--things-patch"),
        OsString::from("--execute"),
    ];
    assert_eq!(arguments(&args), Some(&args[1..]));
    let parsed = CliOptions::parse(&[
        "--scope".into(),
        "rhel".into(),
        "--hosts".into(),
        "srv-01,srv-02".into(),
        "--exclude".into(),
        "srv-03".into(),
        "--root".into(),
        "root-9".into(),
        "--comment".into(),
        "rollout".into(),
        "--no-reboot".into(),
        "--execute".into(),
        "--wait".into(),
        "--poll-secs".into(),
        "0".into(),
        "--timeout-secs".into(),
        "30".into(),
    ])
    .expect("parsed");
    assert!(parsed.execute && parsed.wait && !parsed.reboot);
    assert_eq!(parsed.hosts, vec!["srv-01", "srv-02"]);
}

#[test]
fn invalid_cli_flags_are_rejected() {
    for bad in [
        vec!["--unknown".into()],
        vec!["--scope".into()],
        vec!["--scope".into(), "bsd".into()],
        vec!["--poll-secs".into(), "abc".into()],
    ] {
        assert!(CliOptions::parse(&bad).is_err(), "{bad:?}");
    }
    assert!(run(&["--unknown".into()]).is_err());
}

#[test]
fn resolve_config_and_run_use_token_file_and_environment() {
    let dir = tempfile::TempDir::new().expect("temp");
    let token_path = dir.path().join("token.txt");
    std::fs::write(&token_path, "  file-token \n").expect("write token");
    let opts = CliOptions::parse(&[
        "--root".into(),
        "root-9".into(),
        "--token-file".into(),
        token_path.into(),
    ])
    .expect("opts");
    let _env = super::EnvVarsGuard::set_all(&[
        ("NO_PROXY", "*"),
        (URL_VARIABLE, "https://127.0.0.1:0/graphql"),
        (TOKEN_VARIABLE, "env-tok"),
        (ROOT_ID_VARIABLE, "env-root"),
    ]);
    let cfg = resolve_config(&opts).expect("resolve");
    assert_eq!(cfg.token().as_str(), "file-token");
    assert_eq!(cfg.root_id(), Some("root-9"));
    assert!(run(&["--root".into(), "r1".into()]).is_err());
}

#[test]
fn cli_dry_run_filters_and_splits_waves() {
    let reply_list = things_reply(&["srv-01", "srv-02", "srv-03"]);
    let (url, server) = spawn_json_server(vec![reply_list]);
    let cfg = graphql_cfg(url);
    let opts = CliOptions::parse(&[
        "--hosts".into(),
        "srv-01,srv-02,srv-03".into(),
        "--exclude".into(),
        "srv-03".into(),
    ])
    .expect("dry");
    let mut out = Vec::new();
    run_with(no_proxy_client, &cfg, &opts, &mut out).expect("dry run");
    let text = String::from_utf8(out).expect("utf8");
    assert!(text.contains("Dry run: pass --execute"));
    assert!(text.contains("Wave 1") && text.contains("Wave 2"));
    assert_eq!(server.join().expect("server").len(), 1);
}

#[test]
fn cli_execute_waits_for_each_wave_sequentially() {
    let start = |g, e| {
        json!({ "data": { "bulkStartWorkflowExecutions": { "executionGroup": {
            "id": g, "status": "PENDING", "executions": { "edges": [{ "node": { "id": e, "status": "PENDING" } }] }
        } } } })
    };
    let done = |g| json!({ "data": { "workflows": { "executionGroup": { "id": g, "status": "FINISHED" } } } });
    let (url, server) = spawn_json_server(vec![
        things_reply(&["srv-01", "srv-02"]),
        start("g1", "e1"),
        done("g1"),
        start("g2", "e2"),
        done("g2"),
    ]);
    let cfg = graphql_cfg(url);
    let opts = CliOptions::parse(&[
        "--hosts".into(),
        "srv-01,srv-02".into(),
        "--execute".into(),
        "--wait".into(),
        "--poll-secs".into(),
        "0".into(),
    ])
    .expect("exec");
    let mut out = Vec::new();
    run_with(no_proxy_client, &cfg, &opts, &mut out).expect("exec run");
    let text = String::from_utf8(out).expect("utf8");
    assert!(text.contains("Started wave 1: group=g1"));
    assert!(text.contains("Started wave 2: group=g2"));
    assert_eq!(server.join().expect("server").len(), 5);
}
