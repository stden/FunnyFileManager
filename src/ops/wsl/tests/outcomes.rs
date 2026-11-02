//! Purpose: Update outcomes, and opening a distribution's root in the active panel.

use super::*;

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
