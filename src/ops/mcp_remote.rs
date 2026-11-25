//! Purpose: Operations for browsing Streamable HTTP MCP services and invoking their tools.

pub mod discover;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use crate::command::McpServicesCommand;
use crate::lineedit::LineEditor;
use crate::mcp_remote::schema::default_args_template;
use crate::models::services::mcp_remote::{McpMode, McpRemoteDialog, McpRequestId};
use crate::models::{App, Overlay};
use crate::status::Status;
use crate::values::mcp_remote::{McpReply, McpRequest};

/// Open the Streamable HTTP MCP services dialog.
pub fn open(app: &mut App) {
    let active_dir = app.get_active_panel().path.clone();
    let (extra_env, tls_env) = discover::live_overrides(app.live.mcp_remote);
    let servers = discover::discover_for(
        &app.live.mcp_host,
        app.live.mcp_remote,
        &active_dir,
        None,
        extra_env.as_deref(),
        tls_env,
    );
    app.set_overlay(Overlay::McpServices(Box::new(McpRemoteDialog::new(
        servers,
    ))));
}

/// Dispatch `:mcps ...` command variants.
pub fn command(app: &mut App, cmd: McpServicesCommand) -> std::io::Result<()> {
    match cmd {
        McpServicesCommand::Browse => {
            open(app);
            Ok(())
        }
        McpServicesCommand::Edit => edit_config(app),
    }
}

/// Open `ffm.yml` in the built-in editor to add `mcp_configs:` or `mcp_servers:`.
pub fn edit_config(app: &mut App) -> std::io::Result<()> {
    crate::ops::network_drives::edit_config(app)
}

/// Queue `tools/list` for the highlighted MCP server.
pub fn queue_list_tools(app: &mut App) {
    let Overlay::McpServices(dialog) = &mut app.overlay else {
        return;
    };
    let Some(server) = dialog.selected_server().cloned() else {
        return;
    };
    dialog.queue(McpRequest::ListTools(server));
}

/// Open the JSON argument editor for the highlighted MCP tool.
pub fn open_call_input(app: &mut App) {
    let Overlay::McpServices(dialog) = &mut app.overlay else {
        return;
    };
    let Some(tool) = dialog.selected_tool().cloned() else {
        return;
    };
    let starter = default_args_template(&tool.input_schema);
    dialog.mode = McpMode::CallInput {
        tool: tool.name,
        args: LineEditor::from(starter),
        notice: String::new(),
    };
}

/// Parse the argument JSON in `CallInput` and queue `tools/call`.
pub fn submit_call(app: &mut App) {
    let Overlay::McpServices(dialog) = &mut app.overlay else {
        return;
    };
    let Some(server) = dialog.selected_server().cloned() else {
        return;
    };
    let McpMode::CallInput { tool, args, notice } = &mut dialog.mode else {
        return;
    };
    let raw = args.text().trim();
    let parsed: serde_json::Value = if raw.is_empty() {
        serde_json::json!({})
    } else {
        match serde_json::from_str(raw) {
            Ok(val) => val,
            Err(err) => {
                *notice = format!("Invalid JSON arguments: {err}");
                return;
            }
        }
    };
    let tool_name = tool.clone();
    dialog.mode = McpMode::Browse;
    dialog.queue(McpRequest::CallTool {
        server,
        tool: tool_name,
        arguments: parsed,
    });
}

/// Save displayed MCP output to a unique text file in the active panel.
pub fn save_output(app: &mut App) -> std::io::Result<()> {
    let text = {
        let Overlay::McpServices(dialog) = &app.overlay else {
            return Ok(());
        };
        let McpMode::Output { title, lines, .. } = &dialog.mode else {
            return Ok(());
        };
        format!("# {title}\n\n{}\n", lines.join("\n"))
    };
    let dir = app.get_active_panel_ref().path.clone();
    let dest = crate::utils::unique_destination(&dir, std::ffi::OsStr::new("mcp-output.txt"))?;
    crate::file_write::replace_owner_only(
        &dest,
        "mcp-output",
        &text,
        crate::file_write::SyncPolicy::Skip,
    )?;
    drop(crate::utils::refresh_panels(app));
    app.set_status(Status::CommandOutput(format!("Saved {}", dest.display())));
    Ok(())
}

/// Apply a finished MCP worker reply if the originating dialog and request ID match.
pub fn publish(app: &mut App, generation: &Arc<()>, id: McpRequestId, reply: McpReply) {
    let Overlay::McpServices(dialog) = &mut app.overlay else {
        return;
    };
    if !Arc::ptr_eq(&dialog.generation, generation) || !dialog.awaits(id) {
        return;
    }
    match dialog.apply_reply(reply) {
        Ok(Some(msg)) => app.set_status(Status::CommandOutput(msg)),
        Ok(None) => {
            let msg = dialog.status.clone();
            app.set_status(Status::CommandOutput(msg));
        }
        Err(error) => {
            let msg = error.to_string();
            dialog.status.clone_from(&msg);
            app.set_status(Status::Error(msg));
        }
    }
}
