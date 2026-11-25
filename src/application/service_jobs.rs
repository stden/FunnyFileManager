//! Purpose: Generic off-UI worker supervisor for Jira, HTTP MCP, and AI dialogs.

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_edges;

mod run;
mod slot;

#[cfg(test)]
use std::thread;

use run::{run_live_jira, take_ai, take_jira, take_mcp};
use slot::WorkerSlot;
#[cfg(test)]
use slot::{Pending, FORCE_SPAWN_FAILURE};

use super::Redraw;
use crate::models::services::ai::AiRequestId;
use crate::models::services::jira::JiraRequestId;
use crate::models::services::mcp_remote::McpRequestId;
use crate::models::App;
use crate::values::ai::{AiAccess, AiError, AiRequest, AiResponse};
use crate::values::jira::{JiraAccess, JiraError, JiraReply, JiraRequest};
use crate::values::mcp_remote::{McpAccess, McpRemoteError, McpReply, McpRequest};

type AiReply = Result<AiResponse, AiError>;

fn jira_fallback(request: &JiraRequest) -> JiraReply {
    JiraReply::fail(request, JiraError::Worker)
}

fn mcp_fallback(request: &McpRequest) -> McpReply {
    McpReply::fail(request, McpRemoteError::Worker)
}

const fn ai_fallback(_request: &AiRequest) -> AiReply {
    Err(AiError::Worker)
}

/// Combined background jobs controller for Jira, HTTP MCP, and AI dialogs.
#[derive(Default)]
pub(super) struct ServiceJobs {
    jira: WorkerSlot<JiraRequestId, JiraReply>,
    mcp: WorkerSlot<McpRequestId, McpReply>,
    ai: WorkerSlot<AiRequestId, AiReply>,
}

impl ServiceJobs {
    pub(super) fn pump(&mut self, app: &mut App) -> Redraw {
        self.pump_jira(app)
            .or(self.pump_mcp(app))
            .or(self.pump_ai(app))
    }

    pub(super) fn pump_jira_with(
        &mut self,
        app: &mut App,
        work: impl FnOnce(JiraRequest) -> JiraReply + Send + 'static,
    ) -> Redraw {
        self.jira.pump_with(
            app,
            "ffm-jira",
            (take_jira, jira_fallback, crate::ops::jira::publish),
            work,
        )
    }

    pub(super) fn pump_mcp_with(
        &mut self,
        app: &mut App,
        work: impl FnOnce(McpRequest) -> McpReply + Send + 'static,
    ) -> Redraw {
        self.mcp.pump_with(
            app,
            "ffm-mcp-remote",
            (take_mcp, mcp_fallback, crate::ops::mcp_remote::publish),
            work,
        )
    }

    pub(super) fn pump_ai_with(
        &mut self,
        app: &mut App,
        work: impl FnOnce(AiRequest) -> AiReply + Send + 'static,
    ) -> Redraw {
        self.ai.pump_with(
            app,
            "ffm-ai",
            (take_ai, ai_fallback, crate::ops::ai::actions::publish),
            work,
        )
    }

    fn pump_jira(&mut self, app: &mut App) -> Redraw {
        let access = app.live.jira;
        self.pump_jira_with(app, move |req| match access {
            JiraAccess::Network => run_live_jira(req),
            JiraAccess::Offline => crate::jira::unavailable(&req),
        })
    }

    fn pump_mcp(&mut self, app: &mut App) -> Redraw {
        let access = app.live.mcp_remote;
        self.pump_mcp_with(app, move |req| match access {
            McpAccess::Network => crate::mcp_remote::execute(req),
            McpAccess::Offline => crate::mcp_remote::unavailable(&req),
        })
    }

    fn pump_ai(&mut self, app: &mut App) -> Redraw {
        let access = app.live.ai;
        self.pump_ai_with(app, move |req| match access {
            AiAccess::Network => crate::ai::execute(&req),
            AiAccess::Offline => crate::ai::unavailable(),
        })
    }
}
