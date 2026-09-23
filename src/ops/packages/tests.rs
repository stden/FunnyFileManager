use super::*;
use crate::{models::Panel, packages::Manager, terminal::NoopTerminalOps, WritableTestBackend};
use tempfile::TempDir;

fn fixture() -> (TempDir, App, UpdatePlan) {
    let temp = TempDir::new().expect("tempdir");
    let app = App::with_panels(
        Panel::new(temp.path().to_owned()).expect("panel"),
        Panel::new(temp.path().to_owned()).expect("panel"),
    );
    let plan = UpdatePlan {
        manager: Manager::Apt,
        executable: temp.path().join("fake-apt"),
    };
    (temp, app, plan)
}

/// On Windows this script runs instead of `Manager::commands`, so the two must
/// agree (unattended `-y` included), and choco.exe goes in exactly once.
#[test]
fn elevated_chocolatey_script_runs_the_managers_unattended_upgrade() {
    let upgrade = Manager::Chocolatey.commands()[0]
        .iter()
        .map(|arg| format!("'{arg}'"))
        .collect::<Vec<_>>()
        .join(", ");
    assert!(
        ELEVATED_UPGRADE_SCRIPT.contains(&format!("-ArgumentList {upgrade} ")),
        "elevated_upgrade.ps1 must pass {upgrade}"
    );
    assert_eq!(
        ELEVATED_UPGRADE_SCRIPT
            .matches(CHOCO_EXE_PLACEHOLDER)
            .count(),
        1
    );
}

#[test]
fn dialog_defaults_to_cancel_and_only_an_explicit_choice_queues_updates() {
    let (_temp, mut app, plan) = fixture();
    open_with(&mut app, Some(plan));
    let Overlay::MenuDialog(dialog) = &app.overlay else {
        panic!("dialog");
    };
    let MenuDialog::Choices { rows, selected, .. } = dialog.as_ref() else {
        panic!("choices");
    };
    assert_eq!(*selected, 0);
    assert!(matches!(rows[0].target, ChoiceTarget::Close));
    let run = rows[1].target.clone();
    assert!(app.package_update.is_none());
    crate::ops::menu::choose(&mut app, run).expect("queue");
    assert!(app.package_update.is_some());
}

#[test]
fn absent_managers_and_process_failures_have_visible_outcomes() {
    let (_temp, mut app, plan) = fixture();
    open_with(&mut app, None);
    assert!(app.status_text().contains("No supported package manager"));
    app.package_update = Some(plan);
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    run_with(&mut terminal, &mut app, &mut NoopTerminalOps, |_| {
        Err(io::Error::other("update failed"))
    })
    .expect("restored");
    assert!(app.package_update.is_none());
    assert!(app.status_text().contains("update failed"));
    terminal.draw(|_| {}).expect("usable after failure");
    run_with(&mut terminal, &mut app, &mut NoopTerminalOps, |_| {
        panic!("not queued")
    })
    .expect("idle");
}

#[test]
fn native_command_construction_does_not_execute_the_manager() {
    let (_temp, _app, plan) = fixture();
    let prepared = command(&plan, &plan.executable, &["update"]);
    assert!(prepared.get_args().count() > 0);
}

#[test]
fn opening_updates_only_discovers_a_plan_and_success_restores_the_terminal() {
    let (_temp, mut app, plan) = fixture();
    open(&mut app);
    assert!(app.package_update.is_none());
    assert!(
        matches!(app.overlay, Overlay::MenuDialog(_)) || app.status_text().contains("No supported")
    );
    app.package_update = Some(plan);
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");
    run_with(&mut terminal, &mut app, &mut NoopTerminalOps, |_| {
        Ok("Fixture updated".to_owned())
    })
    .expect("restore");
    assert!(app.status_text().contains("Fixture updated"));
    terminal.draw(|_| {}).expect("terminal remains usable");
}

#[cfg(unix)]
#[test]
fn real_child_process_adapter_only_executes_the_explicit_fixture_program() {
    use std::os::unix::fs::PermissionsExt;
    let (temp, _app, mut plan) = fixture();
    plan.manager = Manager::Brew;
    std::fs::write(
        &plan.executable,
        "#!/bin/sh\ntest \"$1\" = update || test \"$1\" = upgrade\n",
    )
    .expect("fixture executable");
    std::fs::set_permissions(&plan.executable, std::fs::Permissions::from_mode(0o700))
        .expect("executable");
    assert!(execute(&plan)
        .expect("fixture success")
        .contains("completed"));
    std::fs::write(&plan.executable, "#!/bin/sh\nexit 17\n").expect("failure fixture");
    assert!(execute(&plan)
        .expect_err("failure")
        .to_string()
        .contains("code 17"));
    plan.executable = temp.path().join("absent");
    assert!(execute(&plan).is_err());
}
