//! Purpose: Tests for starting the Windows elevated helper session and auto-running active sessions.

use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

use tempfile::TempDir;

use super::super::*;
use super::{clear_session, delete_retry, exited_helper};
use crate::models::{App, Denial, Overlay, Panel};
use crate::progress::Operation;
use crate::status::{FailedEntry, Status};

const CREATE_NO_WINDOW: u32 = 0x0800_0000;

fn pipe_from_params(params: &OsStr) -> String {
    params
        .to_str()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .to_owned()
}

fn spawn_idle_child() -> io::Result<(std::process::Child, Option<std::process::ChildStdin>)> {
    let mut child = Command::new("cmd.exe")
        .args(["/C", "more"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;
    let stdin = child.stdin.take();
    Ok((child, stdin))
}

#[test]
fn exchange_starts_helper_and_rejects_impostor_pipe_client() {
    clear_session();
    let dir = TempDir::new().unwrap();
    let retry = delete_retry(&dir.path().join("victim.txt"), dir.path());
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let mut held_stdin = None;
    let mut connector_slot = None;
    let err = exchange_with(&retry, |_exe, params| {
        let name = pipe_from_params(params);
        let (child, stdin) = spawn_idle_child()?;
        held_stdin = stdin;
        connector_slot = Some(std::thread::spawn(move || {
            let _client = platform::open_pipe(&name, Duration::from_secs(10)).unwrap();
            let _ignored = release_rx.recv();
        }));
        Ok(ElevatedProcess::from_child(child))
    })
    .unwrap_err();
    drop((release_tx, held_stdin));
    connector_slot.unwrap().join().unwrap();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    assert!(!super::super::super::session_active());
}

#[test]
fn exchange_starts_matching_helper_process() {
    clear_session();
    let dir = TempDir::new().unwrap();
    let victim = dir.path().join("auto.txt");
    std::fs::write(&victim, b"x").unwrap();
    let retry = delete_retry(&victim, dir.path());

    let script = concat!(
        "$p = [System.IO.Pipes.NamedPipeClientStream]::new('.', $env:FFM_TEST_PIPE, [System.IO.Pipes.PipeDirection]::InOut); ",
        "$p.Connect(5000); ",
        "$r = [System.IO.StreamReader]::new($p); [void]$r.ReadLine(); ",
        "$w = [System.IO.StreamWriter]::new($p); $w.AutoFlush = $true; ",
        "$w.WriteLine('{\"Finished\":{\"done\":1,\"skipped\":0,\"renamed\":0,\"left\":0,\"failed\":null}}'); ",
        "Start-Sleep -Milliseconds 100"
    );
    let outcome = exchange_with(&retry, |_exe, params| {
        let name = pipe_from_params(params);
        let child = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", script])
            .env("FFM_TEST_PIPE", name)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()?;
        Ok(ElevatedProcess::from_child(child))
    })
    .unwrap();
    assert_eq!(outcome.done, 1);
    assert!(super::super::super::session_active());
    clear_session();
}

fn active_loopback_session() -> std::thread::JoinHandle<io::Result<()>> {
    let name = pipe_name();
    let server = platform::create_pipe(&name).unwrap();
    let pipe_os = OsString::from(name);
    let pid_os = OsString::from(std::process::id().to_string());
    let worker = std::thread::spawn(move || serve_parent(&pipe_os, &pid_os));
    platform::accept(&server).unwrap();
    install_for_test(server, exited_helper()).unwrap();
    worker
}

#[test]
fn stop_auto_retries_delete_via_active_elevated_session() {
    clear_session();
    let dir = TempDir::new().unwrap();
    let victim = dir.path().join("auto.txt");
    std::fs::write(&victim, b"x").unwrap();
    let worker = active_loopback_session();

    let p = dir.path().to_path_buf();
    let mut app = App::with_panels(Panel::new(p.clone()).unwrap(), Panel::new(p).unwrap());
    let failed = FailedEntry::new(&victim, &"denied");
    let stopped = crate::ops::file::Stop {
        operation: Operation::Delete,
        failed: &failed,
        denial: Some(Denial::NeedsAdministrator),
        done: 0,
        remaining: std::slice::from_ref(&victim),
        base: dir.path(),
        selected: 1,
        settings: None,
    };
    crate::ops::file::stop(&mut app, &stopped);
    assert!(!victim.exists());
    assert_eq!(app.status, Status::Deleted { count: 1 });
    assert!(matches!(app.overlay, Overlay::None));
    clear_session();
    worker.join().unwrap().unwrap();
}
