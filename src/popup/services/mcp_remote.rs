//! Purpose: Keyboard handling inside the HTTP MCP services browser and tool call editor.

use crossterm::event::{KeyCode, KeyModifiers};

use super::{edit_active_line, motion_for, scroll_lines};
use crate::lineedit::LineEditor;
use crate::models::services::mcp_remote::{McpFocus, McpMode, McpRemoteDialog};
use crate::models::{App, Overlay};
use crate::ops::mcp_remote::{
    edit_config, open_call_input, queue_list_tools, save_output, submit_call,
};

enum BrowseEffect {
    Close,
    EditConfig,
    ListTools,
    OpenCallInput,
    Handled,
}

pub(super) fn process(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    let Overlay::McpServices(dialog) = &mut app.overlay else {
        return;
    };
    match &mut dialog.mode {
        McpMode::Output { .. } if is_save(code, modifiers) => drop(save_output(app)),
        McpMode::Output { .. } => process_output_nav(dialog, code),
        McpMode::CallInput { .. } if matches!(code, KeyCode::Esc) => dialog.mode = McpMode::Browse,
        McpMode::CallInput { .. } if matches!(code, KeyCode::Enter) => submit_call(app),
        McpMode::CallInput { .. } => edit_active_line(app, code, modifiers, pick_args),
        McpMode::Browse => match browse_effect(dialog, code, modifiers) {
            BrowseEffect::Close => app.close_overlay(),
            BrowseEffect::EditConfig => drop(edit_config(app)),
            BrowseEffect::ListTools => queue_list_tools(app),
            BrowseEffect::OpenCallInput => open_call_input(app),
            BrowseEffect::Handled => {}
        },
    }
}

fn is_save(code: KeyCode, modifiers: KeyModifiers) -> bool {
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    matches!(code, KeyCode::F(2)) || (ctrl && matches!(code, KeyCode::Char('s' | 'S')))
}

fn process_output_nav(dialog: &mut McpRemoteDialog, code: KeyCode) {
    if matches!(code, KeyCode::Esc) {
        dialog.mode = McpMode::Browse;
        return;
    }
    if let (Some(motion), McpMode::Output { lines, scroll, .. }) =
        (motion_for(code), &mut dialog.mode)
    {
        scroll_lines(scroll, lines.len(), motion);
    }
}

pub(super) fn pick_args(overlay: &mut Overlay) -> Option<&mut LineEditor> {
    let Overlay::McpServices(dialog) = overlay else {
        return None;
    };
    match &mut dialog.mode {
        McpMode::CallInput { args, .. } => Some(args),
        _ => None,
    }
}

fn browse_effect(
    dialog: &mut McpRemoteDialog,
    code: KeyCode,
    modifiers: KeyModifiers,
) -> BrowseEffect {
    let ctrl = modifiers.contains(KeyModifiers::CONTROL);
    match code {
        KeyCode::Esc => BrowseEffect::Close,
        KeyCode::Tab | KeyCode::Left | KeyCode::Right => {
            switch_focus(dialog, code);
            BrowseEffect::Handled
        }
        KeyCode::F(4) => BrowseEffect::EditConfig,
        KeyCode::F(5) => BrowseEffect::ListTools,
        KeyCode::Char('r' | 'R') if ctrl => BrowseEffect::ListTools,
        KeyCode::Enter => match dialog.focus {
            McpFocus::Servers => BrowseEffect::ListTools,
            McpFocus::Tools => BrowseEffect::OpenCallInput,
        },
        _ => {
            if let Some(motion) = motion_for(code) {
                dialog.move_cursor(motion);
            }
            BrowseEffect::Handled
        }
    }
}

fn switch_focus(dialog: &mut McpRemoteDialog, code: KeyCode) {
    dialog.focus = match code {
        KeyCode::Left => McpFocus::Servers,
        KeyCode::Right => McpFocus::Tools,
        _ => match dialog.focus {
            McpFocus::Servers => McpFocus::Tools,
            McpFocus::Tools => McpFocus::Servers,
        },
    };
}
