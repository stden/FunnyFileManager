//! Purpose: Jira REST API v2 adapter and dual-format exporter matching `my_jira.py` scripts.

pub mod api;
pub mod client;
pub mod config;
pub mod export;
pub mod markdown;

use std::path::Path;

use crate::values::jira::{JiraError, JiraListing, JiraReply, JiraRequest};
use client::{HttpTransport, Transport};
pub use config::RuntimeConfig;

const SEARCH_FIELDS: &[&str] = &[
    "summary",
    "description",
    "status",
    "comment",
    "assignee",
    "priority",
    "issuetype",
    "updated",
];
const ALL_FIELDS: &[&str] = &["*all"];

/// Execute one Jira request using `RuntimeConfig` on a background thread.
#[must_use]
pub fn execute(config: &RuntimeConfig, request: JiraRequest) -> JiraReply {
    match &request {
        JiraRequest::ExportTickets {
            issues,
            user,
            base_url,
            output_dir,
            force,
        } => {
            let reply = export::export_tickets(output_dir, base_url, user.as_ref(), issues, *force);
            JiraReply::Exported(reply)
        }
        _ => match HttpTransport::new(config) {
            Ok(mut transport) => execute_with(&mut transport, &config.base_url, request),
            Err(error) => JiraReply::fail(&request, error),
        },
    }
}

/// Execute one Jira request through `transport`.
pub fn execute_with(
    transport: &mut dyn Transport,
    base_url: &str,
    request: JiraRequest,
) -> JiraReply {
    let exported = match request {
        JiraRequest::Fetch { jql, page_size } => {
            return JiraReply::Fetched(fetch_listing(transport, base_url, &jql, page_size));
        }
        JiraRequest::ExportKey {
            key,
            output_dir,
            force,
        } => export_one_key(transport, base_url, &key, &output_dir, force),
        JiraRequest::ExportSnapshot {
            jql,
            page_size,
            output_dir,
        } => fetch_and_snapshot(transport, base_url, &jql, page_size, &output_dir),
        JiraRequest::ExportTickets {
            issues,
            user,
            base_url: url,
            output_dir,
            force,
        } => export::export_tickets(&output_dir, &url, user.as_ref(), &issues, force),
    };
    JiraReply::Exported(exported)
}

/// Answer a request when network access is disabled (replays / tests).
#[must_use]
pub fn unavailable(request: &JiraRequest) -> JiraReply {
    JiraReply::fail(request, JiraError::Disabled)
}

fn fetch_listing(
    transport: &mut dyn Transport,
    base_url: &str,
    jql: &str,
    page_size: u16,
) -> Result<JiraListing, JiraError> {
    let user = api::fetch_myself(transport)?;
    let (issues, total) = api::search_all(transport, jql, page_size, SEARCH_FIELDS)?;
    Ok(JiraListing {
        user,
        issues,
        total,
        jql: jql.to_owned(),
        base_url: base_url.to_owned(),
    })
}

fn export_one_key(
    transport: &mut dyn Transport,
    base_url: &str,
    key: &str,
    output_dir: &Path,
    force: bool,
) -> Result<crate::values::jira::ExportReport, JiraError> {
    let user = api::fetch_myself(transport)?;
    let issue = api::fetch_single(transport, key, SEARCH_FIELDS)?;
    export::export_tickets(output_dir, base_url, Some(&user), &[issue], force)
}

fn fetch_and_snapshot(
    transport: &mut dyn Transport,
    base_url: &str,
    jql: &str,
    page_size: u16,
    output_dir: &Path,
) -> Result<crate::values::jira::ExportReport, JiraError> {
    let user = api::fetch_myself(transport)?;
    let (issues, total) = api::search_all(transport, jql, page_size, ALL_FIELDS)?;
    export::export_snapshot(output_dir, base_url, &user, &issues, total, jql)
}

#[cfg(test)]
mod tests;
