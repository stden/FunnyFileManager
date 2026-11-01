//! Purpose: Private package updates execute only an explicit fixture and preserve outcomes.

use super::*;
use crate::{
    models::{Panel, TerminalMode},
    packages::{Elevation, Manager},
    terminal::headless::HeadlessHost,
    WritableTestBackend,
};
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

fn fixture(script: &str) -> io::Result<(TempDir, App)> {
    let root = TempDir::new()?;
    let executable = root.path().join("fake package manager");
    std::fs::write(&executable, format!("#!/bin/sh\n{script}\n"))?;
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700))?;
    let panel = Panel::new(root.path().to_owned())?;
    let mut app = App::with_panels(panel.clone(), panel);
    app.live.terminal_mode = TerminalMode::Headless;
    app.package_update = Some(UpdatePlan {
        manager: Manager::Brew,
        executable,
        elevation: Elevation::CurrentUser,
    });
    Ok((root, app))
}

#[test]
fn private_update_runs_each_step_once_and_keeps_its_output() -> io::Result<()> {
    let (root, mut app) =
        fixture("printf '%s\\n' \"$1\" >>calls.log; echo \"$1 done\"; printf ran >\"$1.ran\"")?;
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let mut terminal = ratatui::Terminal::new(WritableTestBackend::new(80, 24))?;
    crate::ops::packages::run_pending(&mut terminal, &mut app)?;
    assert_eq!(
        std::fs::read_to_string(root.path().join("calls.log"))?,
        "update\nupgrade\n"
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("update.ran"))?,
        "ran"
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("upgrade.ran"))?,
        "ran"
    );
    assert!(app.status_text().contains("Package update completed"));
    assert!(app.package_update.is_none());
    let lines = app.console.visible(100);
    assert!(lines.iter().any(|line| line.text().contains("update done")));
    assert!(lines
        .iter()
        .any(|line| line.text().contains("upgrade done")));
    let count = lines.len();
    run(&mut app)?;
    assert_eq!(app.console.visible(100).len(), count);
    Ok(())
}

#[test]
fn failed_step_keeps_its_reason_and_stops_the_plan() -> io::Result<()> {
    let (root, mut app) =
        fixture("printf '%s\\n' \"$1\" >>calls.log; echo expected-failure; exit 17")?;
    let host = HeadlessHost::new();
    let _scope = host.enter();
    run(&mut app)?;
    assert!(
        matches!(&app.status, Status::Error(message) if message.contains("code 17") && message.contains("expected-failure"))
    );
    assert!(app.package_update.is_none());
    assert_eq!(
        std::fs::read_to_string(root.path().join("calls.log"))?,
        "update\n"
    );
    assert!(app
        .console
        .visible(100)
        .iter()
        .any(|line| line.text().contains("expected-failure")));
    Ok(())
}

#[test]
fn cancelled_update_never_starts_the_fixture() -> io::Result<()> {
    let (root, mut app) = fixture("printf started >unexpected")?;
    let host = HeadlessHost::new();
    host.cancel();
    let _scope = host.enter();
    run(&mut app)?;
    assert_eq!(app.status, Status::PackageUpdateCancelled);
    assert!(app.package_update.is_none());
    assert!(!root.path().join("unexpected").exists());
    Ok(())
}
