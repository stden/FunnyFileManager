//! Unit tests: GraphQL query construction, aggregate and root-free parsing, and error rejection.

use std::time::Duration;

use reqwest::blocking::Client;
use serde_json::json;

use super::super::client::{fetch_with, graphql_headers};
use super::super::graphql::{parse_listing, query_payload};
use crate::things::{fetch, Config, OsInfo, Status, ThingsError};

fn graphql_config(root_id: Option<&str>) -> Config {
    Config::from_values(
        Some("https://things.example.com/graphql".to_owned()),
        Some("test-token-123".to_owned()),
    )
    .expect("graphql config")
    .with_root_id(root_id.map(str::to_owned))
}

fn attr_edge(name: &str, value: serde_json::Value) -> serde_json::Value {
    json!({ "node": { "nameUri": name, "value": { "value": value } } })
}

fn vm_node(
    id: &str,
    name: &str,
    life: &str,
    kind: &str,
    attrs: Vec<serde_json::Value>,
) -> serde_json::Value {
    json!({
        "id": id, "identifierValue": name, "lifecycle": life,
        "type": { "nameUri": kind }, "attributes": { "edges": attrs }
    })
}

fn sample_rhel_vm() -> serde_json::Value {
    vm_node(
        "vm-1",
        "web-01",
        "ACTIVE",
        "vm",
        vec![
            attr_edge(
                "1st_interface_dns_objects",
                json!("[{\"Host Name\":\"web-01.example.com\"}]"),
            ),
            attr_edge("1st_interface_ip", json!("10.0.0.10")),
            attr_edge("vmware_power_state", json!("on")),
            attr_edge("os_name", json!("Red Hat Enterprise Linux 9.8")),
            attr_edge("os_version", json!("9.8")),
            attr_edge("location", json!("Frankfurt")),
            attr_edge("stage", json!("prod")),
            json!({ "node": { "nameUri": "partial" } }),
        ],
    )
}

fn sample_win_vm() -> serde_json::Value {
    vm_node(
        "vm-off",
        "win-01",
        "ACTIVE",
        "vm",
        vec![
            attr_edge(
                "1st_interface_dns_objects",
                json!([{"Host Name": "win-01.example.com"}]),
            ),
            attr_edge("vmware_power_state", json!("off")),
            attr_edge("os_name", json!("Windows Server 2025")),
            attr_edge("os_version", json!("10.0")),
            attr_edge("role_name", json!("app")),
        ],
    )
}

fn sample_vms_payload() -> serde_json::Value {
    json!({ "data": { "aggregate": { "things": { "results": [
        sample_rhel_vm(),
        sample_win_vm(),
        vm_node("vm-old", "old-01", "DEPRECATED", "vm", Vec::new()),
        vm_node("svc-1", "svc", "ACTIVE", "application", Vec::new()),
        vm_node("", "nameless", "ACTIVE", "vm", Vec::new()),
    ] } } } })
}

#[test]
fn query_payload_switches_between_aggregate_root_and_top_level_listing() {
    let with_root = query_payload(Some("root-42"));
    assert!(with_root["query"]
        .as_str()
        .expect("query")
        .contains("ThingsByRoot"));
    assert_eq!(with_root["variables"]["rootId"], "root-42");

    let without_root = query_payload(None);
    assert!(without_root["query"]
        .as_str()
        .expect("query")
        .contains("AllThings"));
    assert_eq!(without_root["variables"], json!({}));
}

#[test]
fn graphql_listing_maps_active_vms_and_filters_retired_or_non_vm_things() {
    let bytes = serde_json::to_vec(&sample_vms_payload()).expect("json");
    let listing = parse_listing(&bytes).expect("parsed");
    assert_eq!(listing.machines.len(), 2);
    let web = &listing.machines[0];
    assert_eq!(web.host(), "web-01.example.com");
    assert_eq!(web.status, Status::Online);
    assert_eq!(
        web.os_info,
        Some(OsInfo {
            name: "Red Hat Enterprise Linux 9.8".to_owned(),
            version: String::new(),
        })
    );
    assert_eq!(web.environment.as_deref(), Some("prod"));
    assert_eq!(web.location.as_deref(), Some("Frankfurt"));
    let win = &listing.machines[1];
    assert_eq!(win.status, Status::Offline);
    assert_eq!(win.environment.as_deref(), Some("app"));
}

#[test]
fn graphql_listing_parses_edges_shape_and_rejects_errors_or_invalid_bodies() {
    let edges_body = json!({
        "data": { "things": { "edges": [
            { "node": { "id": "vm-2", "identifierValue": "db-01", "lifecycle": "ACTIVE" } }
        ] } }
    });
    let listing =
        parse_listing(&serde_json::to_vec(&edges_body).expect("json")).expect("edges parsed");
    assert_eq!(listing.machines[0].name, "db-01");
    assert_eq!(listing.machines[0].status, Status::Unknown);
    assert_eq!(
        parse_listing(b"not-json").err(),
        Some(ThingsError::Response)
    );
    assert_eq!(
        parse_listing(br#"{"errors":[{"message":"denied"}]}"#).err(),
        Some(ThingsError::Request)
    );
    assert_eq!(
        parse_listing(br#"{"data":{}}"#).err(),
        Some(ThingsError::Response)
    );
}

#[test]
fn graphql_headers_and_fetch_with_report_local_endpoint_errors() {
    let cfg = graphql_config(Some("root-1"));
    let headers = graphql_headers(&cfg).expect("headers");
    assert_eq!(
        headers
            .get("graphql-schema-version")
            .and_then(|v| v.to_str().ok()),
        Some("v2-alpha")
    );
    let reply = json!({
        "data": { "aggregate": { "things": { "results": [
            { "id": "vm-1", "identifierValue": "web-01", "lifecycle": "ACTIVE", "type": { "nameUri": "vm" } }
        ] } } }
    });
    let (url, server) = super::patch::spawn_json_server(vec![reply]);
    let mut cfg_ok = graphql_config(Some("root-1"));
    cfg_ok.set_base_for_test(url);
    let no_proxy = || Client::builder().no_proxy().timeout(Duration::from_secs(2));
    assert_eq!(
        fetch_with(no_proxy, &cfg_ok).expect("ok").machines[0].name,
        "web-01"
    );
    assert!(server.join().expect("server")[0].contains("POST /graphql"));

    let _env = super::EnvVarsGuard::set_all(&[("NO_PROXY", "*")]);
    let local = Config::from_values(
        Some("https://127.0.0.1:0/graphql".to_owned()),
        Some("test-token-123".to_owned()),
    )
    .expect("local");
    assert_eq!(fetch(&local).err(), Some(ThingsError::Request));
}
