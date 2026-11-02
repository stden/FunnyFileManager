//! Purpose: Execute named automation commands through the application's canonical interpreters.

use std::io::{self, Write};

use ratatui::{backend::Backend, Terminal};

use crate::{
    action::{netbox::NetboxCommand, vault::VaultCommand, viewer::ViewerCommand},
    automation::catalog::Invocation,
    models::{App, Overlay, PanelKind},
    popup,
};

mod context;

#[expect(
    clippy::redundant_pub_crate,
    reason = "pub(crate) re-export inside a pub(super) parent: visibility is intentionally crate-wide for the snapshot module"
)]
pub(crate) use context::current_context;

#[expect(
    clippy::redundant_pub_crate,
    reason = "pub(crate) fn inside a pub(super) parent: visibility is intentionally crate-wide for the headless runtime"
)]
pub(crate) fn invoke<B: Backend + Write>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    invocation: Invocation,
) -> io::Result<()> {
    context::validate(app, invocation)?;
    match invocation {
        Invocation::Panels(action) => super::dispatch::dispatch_action(terminal, app, action),
        Invocation::TreePanel(command) => {
            popup::tree::apply(app, command);
        }
        Invocation::NetboxPanel(command) => apply_netbox(app, command),
        Invocation::CommandLine(command) => popup::command_line::apply(app, command),
        Invocation::CommandLineSearch(command) => popup::command_line::apply_search(app, command),
        Invocation::CommandLineCompletion(command) => {
            popup::command_line::apply_completion(app, command);
        }
        Invocation::Viewer(command) => apply_viewer(app, command),
        Invocation::Editor(command) => popup::editor::apply_editor_command(app, command),
        Invocation::TaskList(command) => {
            popup::tasks::invoke(app, command);
        }
        Invocation::TaskListSearch(command) => {
            popup::tasks::invoke_search(app, command);
        }
        Invocation::WslManager(command) => popup::wsl::apply(app, command),
        Invocation::LanguageServers(command) => popup::lsp_editor::apply(app, command),
        Invocation::PathEditor(command) => popup::path_env::apply(app, command),
        Invocation::Vault(command) => apply_vault(app, command),
    }
    Ok(())
}

fn apply_netbox(app: &mut App, command: NetboxCommand) {
    match app.get_active_panel_ref().kind {
        PanelKind::Sessions(_) => {
            popup::sessions::apply(app, command);
        }
        PanelKind::Remote(_) => {
            popup::remote::apply(app, command);
        }
        _ => {}
    }
}

fn apply_viewer(app: &mut App, command: ViewerCommand) {
    match (&app.overlay, command) {
        (Overlay::None, ViewerCommand::Scroll(scroll)) => {
            popup::quickview::scroll(app, scroll);
        }
        _ => popup::viewer::apply(app, command),
    }
}

fn apply_vault(app: &mut App, command: VaultCommand) {
    #[cfg(feature = "keepass")]
    popup::kdbx::apply(app, command);
    #[cfg(not(feature = "keepass"))]
    let _ = (app, command);
}

#[cfg(test)]
mod tests;
