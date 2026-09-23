//! Purpose: F9 Commands > Install MCP server — open the dialog, switch its
//! scope, and register `ffm-mcp` with the checked clients.
//!
//! Every function reads the machine through `app.live.mcp_host`, which is inert
//! unless the production entry point installed the real one.

mod claude;
mod config_file;
mod report;
mod survey;
mod system;

use crate::mcp_clients::{ClientId, Host, Mechanism, ProjectDir, Scope, ServerBinary};
use crate::models::{App, Mark, McpInstallDialog, McpInstallProgress, Overlay, Viewer};
use crate::search::Direction;
use crate::status::Status;

use claude::CLI_TIMEOUT;
use report::{Registration, REPORT_TITLE};
pub use survey::survey;
use survey::{project_dir, Target};
pub use system::system_host;

/// The status line when Register is pressed with every box clear.
pub const NOTHING_CHECKED: &str = "Nothing to register: check a client first";

/// The active panel's folder as a project, when it shows one. A `NetBox`
/// panel shows none: its path still names the local folder it hides.
#[must_use]
pub fn active_folder(app: &App) -> Option<ProjectDir> {
    let panel = app.get_active_panel_ref();
    if panel.kind.is_netbox() {
        return None;
    }
    project_dir(&panel.path)
}

/// Open the dialog at All projects, on the active panel's folder.
pub fn open(app: &mut App) {
    let survey = survey(&app.live.mcp_host, Scope::AllProjects, active_folder(app));
    app.set_overlay(Overlay::McpInstall(Box::new(McpInstallDialog::new(survey))));
}

/// Left / Right on the scope radio: survey again for the other scope, which
/// resets every mark to that scope's defaults. Nothing happens without a
/// folder to register in.
pub fn switch_scope(app: &mut App, direction: Direction) {
    let Overlay::McpInstall(dialog) = &mut app.overlay else {
        return;
    };
    let Some(folder) = dialog.folder.clone() else {
        return;
    };
    let scope = crate::cycle::step(&Scope::ALL, dialog.scope, !direction.is_backward());
    dialog.apply(survey(&app.live.mcp_host, scope, Some(folder)));
}

/// Enter on Register: when there is something to register, say in the
/// dialog what is about to run, and leave the work to [`run_pending`], which
/// the event loop calls once that is on screen. `claude` can take up to half
/// a minute a call, and a screen that froze without a word would read as a
/// hang.
pub fn submit(app: &mut App) {
    if ready(app).is_none() {
        return;
    }
    if let Overlay::McpInstall(dialog) = &mut app.overlay {
        dialog.progress = McpInstallProgress::Registering(registering_message(dialog));
    }
}

/// Whether a registration [`submit`] started is waiting for [`run_pending`].
#[must_use]
pub fn is_pending(app: &App) -> bool {
    matches!(&app.overlay, Overlay::McpInstall(dialog)
        if matches!(dialog.progress, McpInstallProgress::Registering(_)))
}

/// Run the registration [`submit`] started; nothing when none is waiting.
pub fn run_pending(app: &mut App) {
    let Overlay::McpInstall(dialog) = &mut app.overlay else {
        return;
    };
    if dialog.progress == McpInstallProgress::Choosing {
        return;
    }
    // Back to choosing first: a registration that stops early leaves the
    // dialog open, and it must not run again on the next event.
    dialog.progress = McpInstallProgress::Choosing;
    register(app);
}

/// What the dialog says while the registration runs: a `claude` call is the
/// one step that can take a while.
fn registering_message(dialog: &McpInstallDialog) -> String {
    let runs_claude = dialog.rows.iter().any(|row| {
        row.mark == Mark::Checked
            && row.status.state.is_actionable()
            && row.status.client.client().mechanism(dialog.scope) == Mechanism::ClaudeCli
    });
    if runs_claude {
        format!(
            "Registering ffm-mcp: waiting for claude (up to {} s a call)…",
            CLI_TIMEOUT.as_secs()
        )
    } else {
        "Registering ffm-mcp…".to_owned()
    }
}

/// What Register works through: the server, the checked clients, and where.
struct Request {
    server: ServerBinary,
    clients: Vec<ClientId>,
    scope: Scope,
    folder: Option<ProjectDir>,
}

/// The request when Register can go ahead. Otherwise a dialog with no
/// server closes, and one with nothing checked says so.
fn ready(app: &mut App) -> Option<Request> {
    let Overlay::McpInstall(dialog) = &app.overlay else {
        return None;
    };
    let Some(server) = dialog.server.binary().cloned() else {
        app.close_overlay();
        return None;
    };
    let clients = dialog.checked_clients();
    if clients.is_empty() {
        app.set_status(Status::Error(NOTHING_CHECKED.to_owned()));
        return None;
    }
    Some(Request {
        server,
        clients,
        scope: dialog.scope,
        folder: dialog.folder.clone(),
    })
}

/// Register the checked clients, then show what happened.
///
/// Runs on the event loop's thread: only Claude Code at All projects starts
/// a process, at most two bounded `claude` calls, and [`submit`] has already
/// put the wait on screen, so a background job would be more machinery than
/// the wait deserves.
pub fn register(app: &mut App) {
    let Some(request) = ready(app) else {
        return;
    };
    let target = Target::new(request.scope, request.folder.as_ref());
    let server = &request.server;
    let registrations: Vec<Registration> = request
        .clients
        .into_iter()
        .map(|client| register_client(&app.live.mcp_host, client, target, server))
        .collect();
    let lines = report::lines(server, target, &registrations);
    app.close_overlay();
    app.set_overlay(Overlay::Viewer(Viewer::report(REPORT_TITLE, lines)));
    app.set_status(report::status(&registrations));
}

/// One client's registration. A failure here never stops the next client.
fn register_client(
    host: &Host,
    client: ClientId,
    target: Target<'_>,
    server: &ServerBinary,
) -> Registration {
    let row = client.client();
    match row.mechanism(target.scope()) {
        Mechanism::ClaudeCli => claude::register(host, &target.entry(server)),
        Mechanism::EditFile => config_file::register(host, row, target, server),
    }
}

#[cfg(test)]
mod tests;
