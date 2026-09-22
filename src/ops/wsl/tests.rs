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

#[test]
fn manager_refreshes_then_preserves_the_selected_distribution_on_refresh() {
    let temp = TempDir::new().expect("temp");
    let mut app = app(temp.path());
    open_manager(&mut app);
    assert_eq!(app.wsl_requests.pending().len(), 1);
    assert_eq!(
        app.wsl_requests.pending()[0].operation,
        WslOperation::Refresh
    );
    complete_distributions(&mut app, &["Ubuntu", "Debian"]);
    assert_eq!(dialog(&app).activity, WslActivity::Ready);
    let Overlay::Wsl(state) = &mut app.overlay else {
        unreachable!()
    };
    state.move_down();
    refresh(&mut app);
    complete_distributions(&mut app, &["Debian", "Alpine"]);
    assert_eq!(
        dialog(&app)
            .selected_distribution()
            .map(DistributionName::as_str),
        Some("Debian")
    );
    assert_eq!(dialog(&app).selected(), Some(0));
}

#[test]
fn empty_selection_and_busy_state_never_queue_an_unsafe_operation() {
    let temp = TempDir::new().expect("temp");
    let mut app = app(temp.path());
    app.set_overlay(Overlay::Wsl(Box::new(WslDialog::new(Vec::new()))));
    for operation in [open_selected, update_packages] {
        operation(&mut app);
        assert!(
            matches!(&dialog(&app).activity, WslActivity::Failed(message) if message.contains("No WSL distribution selected"))
        );
        assert!(app.wsl_requests.pending().is_empty());
    }
    refresh(&mut app);
    let id = pending_id(&app);
    update(&mut app);
    update_packages(&mut app);
    open_selected(&mut app);
    refresh(&mut app);
    assert_eq!(app.wsl_requests.pending().len(), 1);
    assert!(matches!(dialog(&app).activity, WslActivity::Busy { id: active, .. } if active == id));
    app.close_overlay();
    for operation in [refresh, update, update_packages, open_selected] {
        operation(&mut app);
    }
    assert_eq!(app.wsl_requests.pending().len(), 1);
}

#[test]
fn completions_need_the_current_request_id_and_closed_roots_never_navigate() {
    let temp = TempDir::new().expect("temp");
    let other = TempDir::new().expect("other root");
    let mut app = app(temp.path());
    open_manager(&mut app);
    let id = pending_id(&app);
    complete(
        &mut app,
        WslRequestId::new(99),
        Ok(WslOutput::Root(other.path().to_path_buf())),
    );
    assert_eq!(app.left_panel.path, temp.path());
    assert!(matches!(dialog(&app).activity, WslActivity::Busy { id: active, .. } if active == id));
    app.close_overlay();
    complete(
        &mut app,
        id,
        Ok(WslOutput::Root(other.path().to_path_buf())),
    );
    assert_eq!(app.left_panel.path, temp.path());
    assert!(matches!(app.overlay, Overlay::None));
    assert_background_update_outcomes(&mut app, id);
}

#[test]
fn update_outcomes_distinguish_platform_packages_and_failures() {
    let temp = TempDir::new().expect("temp");
    let mut app = app(temp.path());
    app.set_overlay(Overlay::Wsl(Box::new(WslDialog::new(names(&["Debian"])))));
    assert_update_outcome(
        &mut app,
        update,
        WslOperation::Update,
        WslOutput::Updated("WSL updated".to_owned()),
        "WSL updated",
    );
    assert_update_outcome(
        &mut app,
        update_packages,
        WslOperation::UpdatePackages(names(&["Debian"]).remove(0)),
        WslOutput::PackagesUpdated("Packages updated".to_owned()),
        "Packages updated",
    );
    refresh(&mut app);
    let id = pending_id(&app);
    complete(&mut app, id, Err("WSL unavailable".to_owned()));
    assert_eq!(
        dialog(&app).activity,
        WslActivity::Failed("WSL unavailable".to_owned())
    );
    assert_eq!(app.status, Status::Error("WSL unavailable".to_owned()));
}

#[test]
fn opened_distribution_uses_the_active_panel_and_normal_descent_and_copy() {
    let destination = TempDir::new().expect("destination");
    let root = TempDir::new().expect("injected distribution root");
    std::fs::create_dir(root.path().join("home")).expect("home directory");
    std::fs::write(root.path().join("home/config.env"), "NAME=Debian\n").expect("file");
    let mut app = app(destination.path());
    app.active_panel = PanelSide::Right;
    app.set_overlay(Overlay::Wsl(Box::new(WslDialog::new(names(&["Debian"])))));
    open_selected(&mut app);
    let id = pending_id(&app);
    complete(&mut app, id, Ok(WslOutput::Root(root.path().to_path_buf())));
    assert!(matches!(app.overlay, Overlay::None));
    assert_eq!(app.right_panel.path, root.path());
    assert_eq!(app.left_panel.path, destination.path());
    assert_eq!(app.status, Status::Opened(root.path().to_path_buf()));
    app.right_panel.select_named("home");
    app.right_panel.enter().expect("normal descent");
    app.right_panel.select_named("config.env");
    crate::ops::handle_copy(&mut app).expect("normal file copy");
    assert_eq!(
        std::fs::read_to_string(destination.path().join("config.env")).expect("copied file"),
        "NAME=Debian\n"
    );
    assert!(root.path().join("home/config.env").exists());
}

#[test]
fn failed_root_navigation_preserves_the_current_panel_and_reports_error() {
    let temp = TempDir::new().expect("temp");
    std::fs::write(temp.path().join("kept.txt"), "kept").expect("file");
    let mut app = app(temp.path());
    app.left_panel.select_named("kept.txt");
    app.set_overlay(Overlay::Wsl(Box::new(WslDialog::new(names(&["Debian"])))));
    open_selected(&mut app);
    let id = pending_id(&app);
    complete(
        &mut app,
        id,
        Ok(WslOutput::Root(temp.path().join("missing"))),
    );
    assert_eq!(app.left_panel.path, temp.path());
    assert_eq!(
        app.left_panel
            .get_selected_entry()
            .expect("selection")
            .name
            .as_str(),
        "kept.txt"
    );
    assert!(
        matches!(&dialog(&app).activity, WslActivity::Failed(message) if message.contains("Cannot open WSL root"))
    );
    assert!(
        matches!(&app.status, Status::Error(message) if message.contains("Cannot open WSL root"))
    );
}

fn complete_distributions(app: &mut App, values: &[&str]) {
    complete(
        app,
        pending_id(app),
        Ok(WslOutput::Distributions(names(values))),
    );
}

fn assert_background_update_outcomes(app: &mut App, id: WslRequestId) {
    complete(
        app,
        id,
        Ok(WslOutput::Updated("Updated in background".to_owned())),
    );
    assert_eq!(
        app.status,
        Status::CommandOutput("Updated in background".to_owned())
    );
    complete(app, id, Err("Background update failed".to_owned()));
    assert_eq!(
        app.status,
        Status::Error("Background update failed".to_owned())
    );
}

fn assert_update_outcome(
    app: &mut App,
    operation: fn(&mut App),
    expected: WslOperation,
    output: WslOutput,
    message: &str,
) {
    operation(app);
    assert_eq!(
        app.wsl_requests
            .pending()
            .last()
            .expect("request")
            .operation,
        expected
    );
    complete(app, pending_id(app), Ok(output));
    assert_eq!(
        dialog(app).activity,
        WslActivity::Notice(message.to_owned())
    );
}
