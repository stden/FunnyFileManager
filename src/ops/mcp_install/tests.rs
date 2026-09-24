//! Unit tests for [`super`]: opening the dialog on the active panel's folder,
//! switching scope, and registering the checked clients through an `App` on
//! a fake machine.

use std::fs;

use crate::mcp_clients::{ConfigState, ServerState};
use crate::models::{McpInstallField, Panel};
use crate::test_support::{ClaudeBehaviour, FakeMachine};

use super::*;

/// A machine with every client installed and the server beside `ffm`, and
/// an `App` whose panels show its project folder.
fn machine_app() -> (FakeMachine, App) {
    let machine = FakeMachine::new().expect("fake machine");
    machine.install_clients().unwrap();
    machine.install_server().unwrap();
    let app = app_on(&machine);
    (machine, app)
}

fn app_on(machine: &FakeMachine) -> App {
    let mut app = App::with_panels(
        Panel::new(machine.folder()).unwrap(),
        Panel::new(machine.folder()).unwrap(),
    );
    app.live.mcp_host = machine.host();
    app
}

fn dialog(app: &App) -> &McpInstallDialog {
    match &app.overlay {
        Overlay::McpInstall(dialog) => dialog,
        _ => panic!("the Install MCP server dialog is not open"),
    }
}

fn dialog_mut(app: &mut App) -> &mut McpInstallDialog {
    match &mut app.overlay {
        Overlay::McpInstall(dialog) => dialog,
        _ => panic!("the Install MCP server dialog is not open"),
    }
}

mod dialog_scope;
mod home_folder;
mod registering_clients;
