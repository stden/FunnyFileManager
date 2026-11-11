//! Purpose: Loopback named-pipe IPC and validation tests for the Windows elevated helper session.

use std::os::windows::process::CommandExt;
use std::process::{Command, Stdio};

use tempfile::TempDir;

use super::*;
use crate::models::{App, Denial, OperationFailed, Overlay, Panel};
use crate::progress::Operation;
use crate::status::{FailedEntry, Status};

fn exited_helper() -> ElevatedProcess {
    let mut child = Command::new("cmd.exe")
        .args(["/D", "/C", "exit /b 0"])
        .creation_flags(0x0800_0000)
        .spawn()
        .unwrap();
    child.wait().unwrap();
    ElevatedProcess::from_child(child)
}

fn clear_session() {
    SESSION.with(|session| *session.borrow_mut() = None);
}

fn delete_retry(victim: &Path, base: &Path) -> ElevatedRetry {
    ElevatedRetry {
        operation: Operation::Delete,
        selected: 1,
        sources: vec![victim.to_path_buf()],
        base: base.to_path_buf(),
        settings: None,
    }
}

#[test]
fn pipe_name_and_helper_main_reject_invalid_arguments_and_mismatched_pid() {
    let name = pipe_name();
    assert!(name.starts_with(&format!("ffm-elevated-{}-", std::process::id())));
    assert!(name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-'));
    assert_eq!(
        serve_parent(&OsString::from("bad-prefix"), &OsString::from("1"))
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    assert_eq!(
        serve_parent(&OsString::from("ffm-elevated-1"), &OsString::from("nan"))
            .unwrap_err()
            .kind(),
        io::ErrorKind::InvalidInput
    );
    let bad = [
        OsString::from(HELPER_FLAG),
        OsString::from("ffm-elevated-bad!"),
        OsString::from("1"),
    ];
    assert!(super::super::helper_main(&bad).unwrap().is_err());

    let pipe = platform::create_pipe(&name).unwrap();
    let accept_thread = std::thread::spawn(move || platform::accept(&pipe));
    let wrong_pid = std::process::id().wrapping_add(500_000).to_string();
    let err = serve_parent(&OsString::from(&name), &OsString::from(wrong_pid)).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
    drop(accept_thread.join().unwrap());
}

#[test]
fn wait_for_helper_handles_exited_process_and_delayed_pipe_connection() {
    let name_dead = pipe_name();
    let pipe_dead = platform::create_pipe(&name_dead).unwrap();
    assert!(wait_for_helper(&pipe_dead, &exited_helper(), &name_dead).is_err());

    let name_live = pipe_name();
    let pipe_live = platform::create_pipe(&name_live).unwrap();
    let connect_name = name_live.clone();
    let connector = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(140));
        platform::open_pipe(&connect_name, Duration::from_secs(2)).unwrap()
    });
    let live = ElevatedProcess::from_child(
        Command::new("ping.exe")
            .args(["-n", "2", "127.0.0.1"])
            .stdout(Stdio::null())
            .creation_flags(0x0800_0000)
            .spawn()
            .unwrap(),
    );
    wait_for_helper(&pipe_live, &live, &name_live).unwrap();
    drop(connector.join().unwrap());
}

#[test]
fn retry_as_administrator_executes_over_loopback_pipe_and_clears_broken_link() {
    clear_session();
    let dir = TempDir::new().unwrap();
    let victim = dir.path().join("victim.txt");
    std::fs::write(&victim, b"secret").unwrap();
    let name = pipe_name();
    let server = platform::create_pipe(&name).unwrap();
    let (pipe_os, pid_os) = (name.into(), std::process::id().to_string().into());
    let worker = std::thread::spawn(move || serve_parent(&pipe_os, &pid_os));
    platform::accept(&server).unwrap();
    install_for_test(server, exited_helper()).unwrap();
    assert!(super::super::session_active());

    let p = dir.path().to_path_buf();
    let mut app = App::with_panels(Panel::new(p.clone()).unwrap(), Panel::new(p).unwrap());
    let retry = delete_retry(&victim, dir.path());
    let failed = FailedEntry::new(&victim, &"denied");
    let dialog = OperationFailed::new(Operation::Delete, failed, 2, 0)
        .because(Some(Denial::NeedsAdministrator))
        .offering(Some(retry.clone()));
    app.set_overlay(Overlay::OperationFailed(Box::new(dialog)));
    crate::popup::process_popup_input(
        &mut app,
        crossterm::event::KeyEvent::from(crossterm::event::KeyCode::Char('a')),
    );
    assert!(!victim.exists() && app.status == Status::Deleted { count: 3 });
    clear_session();
    worker.join().unwrap().unwrap();
    assert_broken_session_resets(&mut app, &retry);
}

fn send_progress_then_close(name: &str) {
    use std::io::BufRead;

    let mut client = platform::open_pipe(name, Duration::from_secs(2)).unwrap();
    let mut req_line = String::new();
    BufReader::new(client.try_clone().unwrap())
        .read_line(&mut req_line)
        .unwrap();
    let snap = crate::progress::Snapshot {
        operation: Operation::Delete,
        phase: crate::progress::Phase::Working,
        current: std::path::PathBuf::from("x"),
        transfer: None,
        done: crate::progress::Tally::default(),
        total: crate::progress::Tally::default(),
        elapsed: Duration::ZERO,
    };
    protocol::send(&mut client, &Reply::Progress(snap)).unwrap();
}

fn assert_broken_session_resets(app: &mut App, retry: &ElevatedRetry) {
    use crossterm::event::{KeyCode, KeyEvent};

    let name = pipe_name();
    let server = platform::create_pipe(&name).unwrap();
    let closer = std::thread::spawn(move || send_progress_then_close(&name));
    platform::accept(&server).unwrap();
    install_for_test(server, exited_helper()).unwrap();
    let failed = FailedEntry::new(&retry.base, &"denied");
    let dialog = OperationFailed::new(Operation::Delete, failed, 0, 0)
        .because(Some(Denial::NeedsAdministrator))
        .offering(Some(retry.clone()));
    app.set_overlay(Overlay::OperationFailed(Box::new(dialog)));
    crate::popup::process_popup_input(app, KeyEvent::from(KeyCode::Tab));
    crate::popup::process_popup_input(app, KeyEvent::from(KeyCode::Tab));
    crate::popup::process_popup_input(app, KeyEvent::from(KeyCode::Char(' ')));
    closer.join().unwrap();
    assert!(!super::super::session_active());
    assert!(matches!(&app.status, Status::Error(msg) if msg.contains("administrator")));
}

mod tests_start;
