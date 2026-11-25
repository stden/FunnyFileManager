//! Purpose: Private package updates execute only an explicit fixture and preserve outcomes.

use super::*;
use crate::{
    models::{Panel, TerminalMode},
    packages::{Elevation, Manager},
    terminal::headless::HeadlessHost,
    WritableTestBackend,
};
use tempfile::TempDir;

fn fixture(unix_script: &str, windows_script: &str) -> io::Result<(TempDir, App)> {
    let _ = (unix_script, windows_script);
    let root = TempDir::new()?;
    #[cfg(unix)]
    let executable = {
        use std::os::unix::fs::PermissionsExt;
        let p = root.path().join("fake package manager");
        std::fs::write(&p, format!("#!/bin/sh\n{unix_script}\n"))?;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o700))?;
        p
    };
    #[cfg(windows)]
    let executable = {
        let p = root.path().join("fake package manager.cmd");
        std::fs::write(&p, format!("@echo off\r\n{windows_script}\r\n"))?;
        p
    };
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
    let (root, mut app) = fixture(
        "printf '%s\\n' \"$1\" >>calls.log; echo \"$1 done\"; printf ran >\"$1.ran\"",
        "echo %~1>>calls.log\r\necho %~1 done\r\n<nul set /p =ran>%~1.ran\r\nexit /b 0",
    )?;
    let host = HeadlessHost::new();
    let _scope = host.enter();
    let mut terminal = ratatui::Terminal::new(WritableTestBackend::new(80, 24))?;
    crate::ops::packages::run_pending(&mut terminal, &mut app)?;
    let calls = std::fs::read_to_string(root.path().join("calls.log"))?.replace("\r\n", "\n");
    assert_eq!(calls, "update\nupgrade\n");
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
    let (root, mut app) = fixture(
        "printf '%s\\n' \"$1\" >>calls.log; echo expected-failure; exit 17",
        "echo %~1>>calls.log\r\necho expected-failure\r\nexit /b 17",
    )?;
    let host = HeadlessHost::new();
    let _scope = host.enter();
    run(&mut app)?;
    assert!(
        matches!(&app.status, Status::Error(message) if message.contains("code 17") && message.contains("expected-failure"))
    );
    assert!(app.package_update.is_none());
    let calls = std::fs::read_to_string(root.path().join("calls.log"))?.replace("\r\n", "\n");
    assert_eq!(calls, "update\n");
    assert!(app
        .console
        .visible(100)
        .iter()
        .any(|line| line.text().contains("expected-failure")));
    Ok(())
}

#[test]
fn cancelled_update_never_starts_the_fixture() -> io::Result<()> {
    let (root, mut app) = fixture(
        "printf started >unexpected",
        "<nul set /p =started>unexpected",
    )?;
    let host = HeadlessHost::new();
    host.cancel();
    let _scope = host.enter();
    run(&mut app)?;
    assert_eq!(app.status, Status::PackageUpdateCancelled);
    assert!(app.package_update.is_none());
    assert!(!root.path().join("unexpected").exists());
    Ok(())
}
