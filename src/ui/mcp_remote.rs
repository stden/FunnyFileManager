//! Purpose: Render the HTTP MCP services browser, JSON argument prompt, and tool output viewer.

#[cfg(test)]
mod tests;

use ratatui::Frame;

use super::dialog::{self, Ground, Row};
use super::overlays::{clip_cells as clip, list_window as window, render_scrolled_lines};
use crate::models::services::mcp_remote::{McpFocus, McpMode, McpRemoteDialog};
use crate::mouse::DialogMouse;

pub(super) fn render(f: &mut Frame<'_>, tracker: &mut DialogMouse, dialog: &McpRemoteDialog) {
    let max_w = usize::from(f.area().width.saturating_sub(12)).clamp(32, 96);
    let max_h = usize::from(f.area().height.saturating_sub(16)).clamp(4, 12);
    let caret = match &dialog.mode {
        McpMode::Browse => render_browse(f, tracker, dialog, max_w, max_h),
        McpMode::CallInput { tool, args, notice } => {
            render_call_input(f, tracker, tool, args, notice, max_w)
        }
        McpMode::Output {
            title,
            lines,
            scroll,
        } => render_scrolled_lines(
            f,
            tracker,
            title,
            lines,
            *scroll,
            (
                "(empty tool output)",
                "Up/Down Scroll | F2/Ctrl+S Save to mcp-output.txt | Esc Back",
            ),
            (max_w, max_h),
        ),
    };
    if let Some(pos) = caret {
        f.set_cursor_position(pos);
    }
}

fn render_browse(
    f: &mut Frame<'_>,
    tracker: &mut DialogMouse,
    dialog: &McpRemoteDialog,
    max_w: usize,
    max_h: usize,
) -> Option<(u16, u16)> {
    let (head, status, keys) = browse_chrome(dialog, max_w);
    let servers = server_rows(dialog, max_w, (max_h / 2).max(2));
    let tools = tool_rows(dialog, max_w, (max_h / 2).max(3));
    let mut rows = vec![Row::Plain(&head), Row::Divider];
    rows.extend(servers.iter().map(|s| Row::Plain(s)));
    rows.push(Row::Divider);
    rows.extend(tools.iter().map(|s| Row::Plain(s)));
    rows.extend([Row::Divider, Row::Plain(&status), Row::Plain(&keys)]);
    dialog::render_tracked(
        f,
        tracker,
        "MCP Services (HTTP)",
        Ground::Prompt,
        &rows,
        &[],
    )
}

fn browse_chrome(dialog: &McpRemoteDialog, max_w: usize) -> (String, String, String) {
    let focus_label = match dialog.focus {
        McpFocus::Servers => "Servers",
        McpFocus::Tools => "Tools",
    };
    let head = format!(
        "Services: {} | Tools: {} | Focus: {focus_label}",
        dialog.servers.len(),
        dialog.tools.len()
    );
    (
        clip(&head, max_w),
        clip(&dialog.status, max_w),
        clip(
            "Tab Switch  Enter/F5 Tools or Call  F4 Edit ffm.yml  Esc Close",
            max_w,
        ),
    )
}

fn server_rows(dialog: &McpRemoteDialog, max_w: usize, height: usize) -> Vec<String> {
    if dialog.servers.is_empty() {
        return vec!["No HTTP MCP services discovered — check ffm.yml / .mcp.json".to_owned()];
    }
    let active = dialog.focus == McpFocus::Servers;
    window(dialog.servers.len(), dialog.server_cursor, height)
        .filter_map(|idx| dialog.servers.get(idx).map(|s| (idx, s)))
        .map(|(idx, s)| {
            let cur = if active && idx == dialog.server_cursor {
                ">"
            } else {
                " "
            };
            clip(&format!("{cur} {} -> {}", s.name, s.url), max_w)
        })
        .collect()
}

fn tool_rows(dialog: &McpRemoteDialog, max_w: usize, height: usize) -> Vec<String> {
    if dialog.tools.is_empty() {
        return vec!["Press Enter on a service to list MCP tools".to_owned()];
    }
    let active = dialog.focus == McpFocus::Tools;
    window(dialog.tools.len(), dialog.tool_cursor, height)
        .filter_map(|idx| dialog.tools.get(idx).map(|t| (idx, t)))
        .map(|(idx, t)| {
            let cur = if active && idx == dialog.tool_cursor {
                ">"
            } else {
                " "
            };
            let desc = t.description.lines().next().unwrap_or("");
            clip(&format!("{cur} {} — {desc}", t.name), max_w)
        })
        .collect()
}

fn render_call_input(
    f: &mut Frame<'_>,
    tracker: &mut DialogMouse,
    tool: &str,
    args: &crate::lineedit::LineEditor,
    notice: &str,
    max_w: usize,
) -> Option<(u16, u16)> {
    let label = clip(&format!("Call tool {tool} — JSON arguments:"), max_w);
    let err_line = clip(notice, max_w);
    let mut rows = vec![Row::Label(&label), Row::line(args, true, false)];
    if !notice.is_empty() {
        rows.extend([Row::Divider, Row::Plain(&err_line)]);
    }
    rows.extend([Row::Divider, Row::Plain("Enter Run tool | Esc Back")]);
    dialog::render_tracked(f, tracker, "Invoke MCP Tool", Ground::Prompt, &rows, &[])
}
