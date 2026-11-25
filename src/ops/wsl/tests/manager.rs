//! Purpose: The manager's refresh, its guards against unsafe queues, and stale completions.

use super::*;

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
        panic!("the WSL dialog is open");
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
        assert_eq!(app.wsl_requests.pending().len(), 0);
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
