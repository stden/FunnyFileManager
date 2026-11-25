//! Purpose: Streamable HTTP MCP services browser and tool invocation modal state.

use std::sync::Arc;

use crate::lineedit::LineEditor;
use crate::models::{RowCursor, RowMotion};
use crate::values::mcp_remote::{
    HttpMcpServer, McpBusy, McpRemoteError, McpReply, McpRequest, McpTool,
};

/// Numbers HTTP MCP requests so stale background replies are ignored.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct McpRequestId(u64);

impl McpRequestId {
    fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Active column inside the MCP Services browser.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum McpFocus {
    Servers,
    Tools,
}

/// Active sub-view inside the MCP Services dialog.
#[derive(Clone)]
pub enum McpMode {
    Browse,
    CallInput {
        tool: String,
        args: LineEditor,
        notice: String,
    },
    Output {
        title: String,
        lines: Vec<String>,
        scroll: usize,
    },
}

/// Interactive browser for Claude Code / Kiro / `ffm.yml` HTTP MCP services.
#[derive(Clone)]
pub struct McpRemoteDialog {
    pub(crate) generation: Arc<()>,
    pub(crate) latest: McpRequestId,
    pub(crate) servers: Vec<HttpMcpServer>,
    pub(crate) server_cursor: usize,
    pub(crate) tools: Vec<McpTool>,
    pub(crate) tool_cursor: usize,
    pub(crate) focus: McpFocus,
    pub(crate) mode: McpMode,
    pub(crate) busy: Option<McpBusy>,
    pub(crate) queued: Option<(McpRequestId, McpRequest)>,
    pub(crate) status: String,
}

impl McpRemoteDialog {
    #[must_use]
    pub(crate) fn new(servers: Vec<HttpMcpServer>) -> Self {
        let status = if servers.is_empty() {
            "No HTTP MCP services found — check ~/.kiro/settings/mcp.json, .mcp.json, or ffm.yml"
                .to_owned()
        } else {
            format!("Loaded {} HTTP MCP service(s)", servers.len())
        };
        let mut dialog = Self {
            generation: Arc::new(()),
            latest: McpRequestId::default(),
            servers,
            server_cursor: 0,
            tools: Vec::new(),
            tool_cursor: 0,
            focus: McpFocus::Servers,
            mode: McpMode::Browse,
            busy: None,
            queued: None,
            status,
        };
        if let Some(first) = dialog.selected_server().cloned() {
            dialog.queue(McpRequest::ListTools(first));
        }
        dialog
    }

    #[must_use]
    pub(crate) fn selected_server(&self) -> Option<&HttpMcpServer> {
        self.servers.get(self.server_cursor)
    }

    #[must_use]
    pub(crate) fn selected_tool(&self) -> Option<&McpTool> {
        self.tools.get(self.tool_cursor)
    }

    pub(crate) fn move_cursor(&mut self, motion: RowMotion) {
        if let McpMode::Output { lines, scroll, .. } = &mut self.mode {
            let max_rows = lines.len();
            *scroll = RowCursor::at(*scroll, max_rows)
                .moved(motion, max_rows)
                .index();
            return;
        }
        match self.focus {
            McpFocus::Servers => {
                let len = self.servers.len();
                self.server_cursor = RowCursor::at(self.server_cursor, len)
                    .moved(motion, len)
                    .index();
            }
            McpFocus::Tools => {
                let len = self.tools.len();
                self.tool_cursor = RowCursor::at(self.tool_cursor, len)
                    .moved(motion, len)
                    .index();
            }
        }
    }

    pub(crate) fn queue(&mut self, request: McpRequest) {
        let id = self.latest.next();
        self.latest = id;
        self.busy = Some(request.busy());
        request.busy().label().clone_into(&mut self.status);
        self.queued = Some((id, request));
    }

    pub(crate) fn take_queued(&mut self) -> Option<(McpRequestId, McpRequest)> {
        self.queued.take()
    }

    #[must_use]
    pub(crate) const fn awaits(&self, id: McpRequestId) -> bool {
        self.latest.0 == id.0
    }

    #[must_use]
    pub(crate) const fn is_busy(&self) -> bool {
        self.busy.is_some() || self.queued.is_some()
    }

    pub(crate) fn apply_reply(
        &mut self,
        reply: McpReply,
    ) -> Result<Option<String>, McpRemoteError> {
        self.busy = None;
        match reply {
            McpReply::Tools(server_name, result) => {
                let tools = result?;
                let count = tools.len();
                self.tools = tools;
                self.tool_cursor = 0;
                self.focus = McpFocus::Tools;
                let msg = format!("{server_name}: {count} tool(s)");
                self.status.clone_from(&msg);
                Ok(Some(msg))
            }
            McpReply::Called(result) => {
                let call = result?;
                let prefix = if call.is_error { "ERROR: " } else { "" };
                let title = format!("{prefix}{}:{}", call.server, call.tool);
                let lines = call.text.lines().map(str::to_owned).collect();
                self.mode = McpMode::Output {
                    title,
                    lines,
                    scroll: 0,
                };
                self.status = format!("{} finished", call.tool);
                Ok(None)
            }
        }
    }
}
