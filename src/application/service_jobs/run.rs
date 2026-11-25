//! Purpose: Queue extraction and synchronous worker execution helpers for service jobs.

use std::sync::Arc;

use crate::models::services::ai::AiRequestId;
use crate::models::services::jira::JiraRequestId;
use crate::models::services::mcp_remote::McpRequestId;
use crate::models::{App, Overlay};
use crate::values::ai::AiRequest;
use crate::values::jira::{JiraReply, JiraRequest};
use crate::values::mcp_remote::McpRequest;

pub(super) fn take_jira(app: &mut App) -> Option<(Arc<()>, JiraRequestId, JiraRequest)> {
    let Overlay::Jira(dialog) = &mut app.overlay else {
        return None;
    };
    let (id, request) = dialog.take_queued()?;
    Some((Arc::clone(&dialog.generation), id, request))
}

pub(super) fn take_mcp(app: &mut App) -> Option<(Arc<()>, McpRequestId, McpRequest)> {
    let Overlay::McpServices(dialog) = &mut app.overlay else {
        return None;
    };
    let (id, request) = dialog.take_queued()?;
    Some((Arc::clone(&dialog.generation), id, request))
}

pub(super) fn take_ai(app: &mut App) -> Option<(Arc<()>, AiRequestId, AiRequest)> {
    let Overlay::Ai(dialog) = &mut app.overlay else {
        return None;
    };
    let (id, request) = dialog.take_queued()?;
    Some((Arc::clone(&dialog.generation), id, request))
}

pub(super) fn run_live_jira(request: JiraRequest) -> JiraReply {
    if let JiraRequest::ExportTickets {
        issues,
        user,
        base_url,
        output_dir,
        force,
    } = request
    {
        return JiraReply::Exported(crate::jira::export::export_tickets(
            &output_dir,
            &base_url,
            user.as_ref(),
            &issues,
            force,
        ));
    }
    let settings = crate::jira::config::load_default_settings();
    match crate::jira::config::resolve_config(&settings) {
        Ok(cfg) => crate::jira::execute(&cfg, request),
        Err(error) => JiraReply::fail(&request, error),
    }
}

#[cfg(test)]
mod tests;
