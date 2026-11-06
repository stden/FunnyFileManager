//! Purpose: a click past a dialog's last control leaves its focus where it was.

use super::*;

#[test]
fn clicking_below_the_mcp_client_list_keeps_its_focus() {
    use crate::mcp_clients::{Scope, ServerState, Survey};
    use crate::models::{McpInstallDialog, McpInstallField};
    let dir = TempDir::new().expect("temp");
    let mut app = app_at(&dir);
    let survey = Survey {
        server: ServerState::Missing,
        folder: None,
        scope: Scope::AllProjects,
        clients: Vec::new(),
    };
    app.set_overlay(Overlay::McpInstall(Box::new(McpInstallDialog::new(survey))));
    click_dialog_row(&mut app, 999);
    assert!(matches!(&app.overlay, Overlay::McpInstall(d) if d.field == McpInstallField::Scope));
}
