//! Purpose: GraphQL mutations and polling queries for Things server patch execution groups.

use reqwest::blocking::ClientBuilder;
use serde_json::{json, Value};

use super::{ExecutionInfo, GroupResult, PatchKind, PatchPlan, PatchReport, PatchWave};
use crate::things::{
    client::{graphql_headers, https_builder, post_json},
    graphql::parse_graphql_data,
    Config, ThingsError,
};

const BULK_START_MUTATION: &str =
    "mutation BulkStart($input: BulkStartWorkflowExecutionsMutationInput!) { \
  bulkStartWorkflowExecutions(input: $input) { \
    executionGroup { id status executions { edges { node { id status } } } } \
  } }";

const EXECUTION_GROUP_QUERY: &str = "query ExecutionGroup($id: ID!) { \
  workflows { executionGroup(id: $id) { \
    id status executions { edges { node { id status } } } \
  } } }";

/// Build the GraphQL payload to start one patch wave; omits `rebootSystem` on Windows.
pub fn build_start_payload(wave: &PatchWave, reboot: bool, comment: &str) -> Value {
    let inputs: Vec<Value> = wave
        .targets
        .iter()
        .map(|target| match wave.kind {
            PatchKind::Rhel => json!({ "graphqlId": target.id, "rebootSystem": reboot }),
            PatchKind::Windows => json!({ "graphqlId": target.id }),
        })
        .collect();
    json!({
        "query": BULK_START_MUTATION,
        "variables": {
            "input": {
                "typeNameUri": wave.kind.workflow_uri(),
                "inputs": inputs,
                "comment": comment,
                "linkThingIds": [],
            }
        }
    })
}

/// Start one patch wave against the configured Things GraphQL endpoint.
pub fn start_wave(
    config: &Config,
    wave: &PatchWave,
    reboot: bool,
    comment: &str,
) -> Result<GroupResult, ThingsError> {
    start_wave_with(https_builder(), config, wave, reboot, comment)
}

pub fn start_wave_with(
    builder: ClientBuilder,
    config: &Config,
    wave: &PatchWave,
    reboot: bool,
    comment: &str,
) -> Result<GroupResult, ThingsError> {
    let payload = build_start_payload(wave, reboot, comment);
    let headers = graphql_headers(config)?;
    let bytes = post_json(builder, &config.graphql_url(), headers, &payload)?;
    let data = parse_graphql_data(&bytes)?;
    let group = data
        .get("bulkStartWorkflowExecutions")
        .and_then(|res| res.get("executionGroup"))
        .ok_or(ThingsError::Response)?;
    parse_group(group)
}

/// Poll the current status of a previously started execution group.
pub fn poll_group(config: &Config, group_id: &str) -> Result<GroupResult, ThingsError> {
    poll_group_with(https_builder(), config, group_id)
}

pub fn poll_group_with(
    builder: ClientBuilder,
    config: &Config,
    group_id: &str,
) -> Result<GroupResult, ThingsError> {
    let payload = json!({
        "query": EXECUTION_GROUP_QUERY,
        "variables": { "id": group_id },
    });
    let headers = graphql_headers(config)?;
    let bytes = post_json(builder, &config.graphql_url(), headers, &payload)?;
    let data = parse_graphql_data(&bytes)?;
    let group = data
        .get("workflows")
        .and_then(|wf| wf.get("executionGroup"))
        .ok_or(ThingsError::Response)?;
    parse_group(group)
}

/// Start every wave in `plan` sequentially and collect the started execution groups.
pub fn execute_plan(config: &Config, plan: &PatchPlan) -> Result<PatchReport, ThingsError> {
    execute_plan_with(https_builder, config, plan)
}

pub fn execute_plan_with(
    make_builder: fn() -> ClientBuilder,
    config: &Config,
    plan: &PatchPlan,
) -> Result<PatchReport, ThingsError> {
    if plan.waves.is_empty() {
        return Err(ThingsError::Response);
    }
    let mut groups = Vec::with_capacity(plan.waves.len());
    for wave in &plan.waves {
        groups.push(start_wave_with(
            make_builder(),
            config,
            wave,
            plan.reboot,
            &plan.comment,
        )?);
    }
    Ok(PatchReport { groups })
}

fn parse_group(group: &Value) -> Result<GroupResult, ThingsError> {
    let id = group
        .get("id")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .ok_or(ThingsError::Response)?
        .to_owned();
    let status = group
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or("UNKNOWN")
        .to_owned();
    let executions = parse_executions(group);
    Ok(GroupResult {
        id,
        status,
        executions,
    })
}

fn parse_executions(group: &Value) -> Vec<ExecutionInfo> {
    let Some(edges) = group
        .get("executions")
        .and_then(|exec| exec.get("edges"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    edges
        .iter()
        .filter_map(|edge| edge.get("node"))
        .filter_map(|node| {
            let id = node.get("id").and_then(Value::as_str)?.to_owned();
            let status = node
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("UNKNOWN")
                .to_owned();
            Some(ExecutionInfo { id, status })
        })
        .collect()
}
