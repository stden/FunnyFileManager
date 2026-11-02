//! Purpose: Verify cancellable installer jobs without installing host packages or using the network.

use super::*;
use crate::config::{InstallCommand, InstallRecipe};
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration, Instant},
};
use tempfile::TempDir;

pub(super) fn recipe(script: &str) -> ServerSettings {
    ServerSettings {
        command: "missing-ffm-test-server".into(),
        args: vec!["--stdio".into()],
        install: Some(InstallRecipe::Command {
            steps: vec![InstallCommand {
                command: "python".into(),
                args: vec!["-c".into(), script.into()],
                env: BTreeMap::new(),
            }],
            executable: "server".into(),
        }),
        ..ServerSettings::default()
    }
}

pub(super) fn simple_recipe() -> ServerSettings {
    recipe("from pathlib import Path; Path('server').write_text('installed'); Path('server').chmod(0o755)")
}

fn wait(job: &Job) -> Result<ServerSettings, InstallError> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if let Some(result) = job.poll() {
            return result;
        }
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn a_job_publishes_a_usable_local_command_and_preserves_server_arguments() {
    let root = TempDir::new().unwrap();
    let job = Job::start(
        simple_recipe(),
        root.path().join("cache"),
        root.path().join("project"),
    )
    .unwrap();
    let installed = wait(&job).unwrap();
    assert!(Path::new(&installed.command).is_file());
    assert_eq!(installed.args, ["--stdio"]);
    assert!(installed.install.is_none());
}

#[test]
fn dropping_a_running_install_cancels_the_process_and_removes_staging() {
    let root = TempDir::new().unwrap();
    let settings =
        recipe("from pathlib import Path; import time; Path('started').touch(); time.sleep(30)");
    let job = Job::start(
        settings,
        root.path().to_path_buf(),
        root.path().join("project"),
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    while !std::fs::read_dir(root.path())
        .unwrap()
        .any(|entry| entry.unwrap().path().join("started").exists())
    {
        assert!(Instant::now() < deadline);
        thread::sleep(Duration::from_millis(5));
    }
    let start = Instant::now();
    drop(job);
    assert!(start.elapsed() < Duration::from_secs(2));
    assert!(std::fs::read_dir(root.path()).unwrap().all(|entry| entry
        .unwrap()
        .path()
        .extension()
        .is_some_and(|ext| ext == "lock")));
}

#[test]
fn a_failed_installer_reports_failure_and_allows_an_explicit_retry() {
    let root = TempDir::new().unwrap();
    let settings = recipe("from pathlib import Path; marker=Path('../attempt'); failed=not marker.exists(); marker.touch(); exec('if failed: raise SystemExit(7)'); Path('server').touch(); Path('server').chmod(0o755)");
    let job = Job::start(
        settings.clone(),
        root.path().to_path_buf(),
        root.path().to_path_buf(),
    )
    .unwrap();
    assert_eq!(wait(&job).unwrap_err(), InstallError::Failed(Step::Custom));
    let retry = Job::start(
        settings,
        root.path().to_path_buf(),
        root.path().to_path_buf(),
    )
    .unwrap();
    assert!(wait(&retry).is_ok());
}
