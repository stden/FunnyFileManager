use super::*;
use crate::{
    models::Panel,
    packages::{Elevation, Manager},
    terminal::NoopTerminalOps,
    WritableTestBackend,
};
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
        elevation: Elevation::CurrentUser,
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
    run_with(&mut terminal, &mut app, &mut NoopTerminalOps, |_, _| {
        Err(io::Error::other("update failed"))
    })
    .expect("restored");
    assert!(app.package_update.is_none());
    assert!(app.status_text().contains("update failed"));
    terminal.draw(|_| {}).expect("usable after failure");
    run_with(&mut terminal, &mut app, &mut NoopTerminalOps, |_, _| {
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
    run_with(&mut terminal, &mut app, &mut NoopTerminalOps, |_, _| {
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
    let (temp, mut app, mut plan) = fixture();
    plan.manager = Manager::Brew;
    std::fs::write(
        &plan.executable,
        "#!/bin/sh\ntest \"$1\" = update || test \"$1\" = upgrade\n",
    )
    .expect("fixture executable");
    std::fs::set_permissions(&plan.executable, std::fs::Permissions::from_mode(0o700))
        .expect("executable");
    assert!(execute(&plan, &mut app)
        .expect("fixture success")
        .contains("completed"));
    std::fs::write(&plan.executable, "#!/bin/sh\nexit 17\n").expect("failure fixture");
    assert!(execute(&plan, &mut app)
        .expect_err("failure")
        .to_string()
        .contains("code 17"));
    plan.executable = temp.path().join("absent");
    assert!(execute(&plan, &mut app).is_err());
}

/// Make `plan` a Brew plan whose executable is the shell `script`.
#[cfg(unix)]
fn scripted(plan: &mut UpdatePlan, script: &str) {
    use std::os::unix::fs::PermissionsExt;
    plan.manager = Manager::Brew;
    std::fs::write(&plan.executable, script).expect("fixture script");
    std::fs::set_permissions(&plan.executable, std::fs::Permissions::from_mode(0o700))
        .expect("executable");
}

#[cfg(unix)]
fn transcript(app: &App) -> Vec<String> {
    app.console
        .visible(usize::MAX)
        .into_iter()
        .map(|line| line.text().to_owned())
        .collect()
}

/// The reason a step failed is printed on stderr (sudo's "a password is
/// required", apt-get's "E: ..."), on a screen the panels then cover. It
/// has to survive into the status and the Ctrl+O transcript.
#[cfg(unix)]
#[test]
fn a_failed_step_keeps_its_reason_in_the_status_and_the_transcript() {
    let (_temp, mut app, mut plan) = fixture();
    let reason = "E: fixture lock is held (expected by this test)";
    scripted(
        &mut plan,
        &format!("#!/bin/sh\necho '{reason}' >&2\nexit 100\n"),
    );
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");

    // A test thread that forks while the script is still open for writing
    // keeps it busy (ETXTBSY) for a moment: the fixture's race, not FFM's.
    for _ in 0..5 {
        app.package_update = Some(plan.clone());
        run_with(&mut terminal, &mut app, &mut NoopTerminalOps, execute).expect("restored");
        if !app.status_text().contains("Text file busy") {
            break;
        }
    }

    let expected = format!("brew update exited with code 100: {reason}");
    assert_eq!(app.status, Status::Error(expected));
    let transcript = transcript(&app);
    let echoed = transcript
        .iter()
        .any(|line| line.ends_with("> brew update"));
    assert!(echoed, "{transcript:?}");
    assert!(
        transcript.iter().any(|line| line == reason),
        "{transcript:?}"
    );
}
