//! The fixture never mistakes an unrelated listener for its own sshd.

use super::Server;
use std::fs;
use std::io;
use std::net::TcpListener;
use std::os::unix::process::CommandExt;
use std::process::{Command, Stdio};

#[test]
fn readiness_requires_a_complete_pidfile_owned_by_the_live_child() {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("local listener");
    listener
        .set_nonblocking(true)
        .expect("nonblocking listener");
    let port = listener.local_addr().expect("listener address").port();
    let server = test_server(port);
    assert!(!server.is_ready());
    assert_no_connection(&listener);
    fs::write(server.local("sshd.pid"), "unfinished").expect("partial pidfile");
    assert!(!server.is_ready());
    assert_no_connection(&listener);
    let wrong = server.child.id().checked_add(1).expect("pid");
    fs::write(server.local("sshd.pid"), format!("{wrong}\n")).expect("wrong pid");
    let refusal = std::panic::catch_unwind(|| server.is_ready());
    assert!(refusal.is_err(), "another pid must be refused");
    assert_no_connection(&listener);
    fs::write(server.local("sshd.pid"), format!("{}\n", server.child.id())).expect("correct pid");
    assert!(server.is_ready());
    assert!(listener.accept().is_ok());
}

fn test_server(port: u16) -> Server {
    let child = Command::new("sleep")
        .arg("30")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .expect("fixture child");
    Server {
        root: tempfile::tempdir().expect("fixture directory"),
        child,
        port,
        user: String::new(),
    }
}

fn assert_no_connection(listener: &TcpListener) {
    assert_eq!(
        listener.accept().unwrap_err().kind(),
        io::ErrorKind::WouldBlock
    );
}
