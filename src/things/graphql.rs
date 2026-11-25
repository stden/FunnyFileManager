//! Purpose: GraphQL machine listing query and attribute mapper for Things.

use std::collections::BTreeMap;

use reqwest::blocking::ClientBuilder;
use serde_json::{json, Value};

use super::client::{graphql_headers, post_json};
use super::{Config, Listing, Machine, OsInfo, Status, ThingsError};

const ROOT_THINGS_QUERY: &str = "query ThingsByRoot($rootId: ID!) { \
  aggregate { things(rootThingId: $rootId, direction: CHILDREN) { \
    results { id identifierValue lifecycle type { nameUri } \
      attributes(first: 100) { edges { node { nameUri value { value } } } } } \
  } } }";

const ALL_THINGS_QUERY: &str = "query AllThings { \
  things(first: 200) { edges { node { \
    id identifierValue lifecycle type { nameUri } \
    attributes(first: 100) { edges { node { nameUri value { value } } } } \
  } } } }";

pub(in crate::things) fn fetch(
    builder: ClientBuilder,
    config: &Config,
) -> Result<Listing, ThingsError> {
    let payload = query_payload(config.root_id());
    let headers = graphql_headers(config)?;
    let body = post_json(builder, &config.graphql_url(), headers, &payload)?;
    parse_listing(&body)
}

pub(in crate::things) fn query_payload(root_id: Option<&str>) -> Value {
    match root_id {
        Some(id) => json!({ "query": ROOT_THINGS_QUERY, "variables": { "rootId": id } }),
        None => json!({ "query": ALL_THINGS_QUERY, "variables": {} }),
    }
}

/// Validate JSON and reject non-empty GraphQL error arrays, returning `data`.
pub(in crate::things) fn parse_graphql_data(body: &[u8]) -> Result<Value, ThingsError> {
    let root: Value = serde_json::from_slice(body).map_err(|_json_error| ThingsError::Response)?;
    let has_errors = root
        .get("errors")
        .and_then(Value::as_array)
        .is_some_and(|errors| !errors.is_empty());
    if has_errors {
        return Err(ThingsError::Request);
    }
    root.get("data").cloned().ok_or(ThingsError::Response)
}

/// Parse a GraphQL `aggregate.things` or `things` response into active VM machines.
pub fn parse_listing(body: &[u8]) -> Result<Listing, ThingsError> {
    let data = parse_graphql_data(body)?;
    let nodes = extract_nodes(&data).ok_or(ThingsError::Response)?;
    let machines = nodes.into_iter().filter_map(node_to_machine).collect();
    Ok(Listing::new(machines, None))
}

fn extract_nodes(data: &Value) -> Option<Vec<&Value>> {
    if let Some(results) = data
        .get("aggregate")
        .and_then(|agg| agg.get("things"))
        .and_then(|things| things.get("results"))
        .and_then(Value::as_array)
    {
        return Some(results.iter().collect());
    }
    data.get("things")
        .and_then(|things| things.get("edges"))
        .and_then(Value::as_array)
        .map(|edges| edges.iter().filter_map(|edge| edge.get("node")).collect())
}

fn node_to_machine(node: &Value) -> Option<Machine> {
    let lifecycle = str_field(node, "lifecycle");
    if !lifecycle.is_empty() && !lifecycle.eq_ignore_ascii_case("ACTIVE") {
        return None;
    }
    let kind = node
        .get("type")
        .and_then(|kind| kind.get("nameUri"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !kind.is_empty() && !kind.eq_ignore_ascii_case("vm") {
        return None;
    }
    let id = str_field(node, "id");
    let name = str_field(node, "identifierValue");
    if id.is_empty() || name.is_empty() {
        return None;
    }
    let attrs = collect_attributes(node);
    Some(machine_from_attrs(id, name, &attrs))
}

fn machine_from_attrs(id: String, name: String, attrs: &BTreeMap<&str, &Value>) -> Machine {
    let hostname = extract_hostname(attrs.get("1st_interface_dns_objects").copied());
    let ip_address = attr_text(attrs, "1st_interface_ip");
    let status = parse_power_state(attr_text(attrs, "vmware_power_state").as_deref());
    let os_info = build_os_info(attrs);
    let environment = attr_text(attrs, "stage")
        .or_else(|| attr_text(attrs, "environment"))
        .or_else(|| attr_text(attrs, "role_name"));
    let location = attr_text(attrs, "location");
    Machine {
        id,
        name,
        hostname,
        ip_address,
        status,
        os_info,
        environment,
        location,
        tags: Vec::new(),
    }
}

fn collect_attributes(node: &Value) -> BTreeMap<&str, &Value> {
    let mut map = BTreeMap::new();
    let Some(edges) = node
        .get("attributes")
        .and_then(|attrs| attrs.get("edges"))
        .and_then(Value::as_array)
    else {
        return map;
    };
    for item in edges.iter().filter_map(|edge| edge.get("node")) {
        if let (Some(key), Some(val)) = (
            item.get("nameUri").and_then(Value::as_str),
            item.get("value").and_then(|v| v.get("value")),
        ) {
            map.insert(key, val);
        }
    }
    map
}

fn str_field(node: &Value, key: &str) -> String {
    node.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim()
        .to_owned()
}

fn attr_text(attrs: &BTreeMap<&str, &Value>, key: &str) -> Option<String> {
    attrs
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn extract_hostname(value: Option<&Value>) -> String {
    let Some(val) = value else {
        return String::new();
    };
    if let Some(text) = val.as_str() {
        return serde_json::from_str::<Value>(text)
            .ok()
            .and_then(|parsed| host_from_array(&parsed))
            .unwrap_or_default();
    }
    host_from_array(val).unwrap_or_default()
}

fn host_from_array(val: &Value) -> Option<String> {
    val.as_array()?
        .iter()
        .filter_map(|item| item.get("Host Name").and_then(Value::as_str))
        .map(str::trim)
        .find(|host| !host.is_empty())
        .map(str::to_owned)
}

fn parse_power_state(state: Option<&str>) -> Status {
    match state.map(str::trim) {
        Some(s) if s.eq_ignore_ascii_case("on") => Status::Online,
        Some(s) if s.eq_ignore_ascii_case("off") => Status::Offline,
        _ => Status::Unknown,
    }
}

fn build_os_info(attrs: &BTreeMap<&str, &Value>) -> Option<OsInfo> {
    let name = attr_text(attrs, "os_name").unwrap_or_default();
    let mut version = attr_text(attrs, "os_version").unwrap_or_default();
    if name.is_empty() && version.is_empty() {
        return None;
    }
    if !version.is_empty() && name.ends_with(&version) {
        version.clear();
    }
    Some(OsInfo { name, version })
}
