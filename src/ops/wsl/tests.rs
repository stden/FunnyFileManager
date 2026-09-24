//! WSL transitions run against injected results and temporary local roots.

use tempfile::TempDir;

use crate::models::{DistributionName, Panel, PanelSide};

use super::*;

fn app(root: &std::path::Path) -> App {
    let mut app = App::new().expect("app");
    app.left_panel = Panel::new(root.to_path_buf()).expect("left panel");
    app.right_panel = Panel::new(root.to_path_buf()).expect("right panel");
    app
}

fn names(values: &[&str]) -> Vec<DistributionName> {
    values
        .iter()
        .map(|name| DistributionName::new(name).expect("distribution"))
        .collect()
}

fn dialog(app: &App) -> &WslDialog {
    let Overlay::Wsl(dialog) = &app.overlay else {
        panic!("WSL dialog")
    };
    dialog
}

fn pending_id(app: &App) -> WslRequestId {
    app.wsl_requests
        .pending()
        .last()
        .expect("pending request")
        .id
}

mod manager;
mod outcomes;
