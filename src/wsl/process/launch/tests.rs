//! Purpose: Unit tests for Windows directory lookup, WSL command construction, and `ChildGuard` lifecycle.

use std::io::{self, Read};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

use super::*;

#[test]
fn windows_directory_and_wsl_command_resolve_system32_executable() {
    let win_dir = windows_directory().expect("Windows directory");
    assert!(win_dir.is_dir());

    let cmd = command(Path::new(r"C:\Windows"), &["--list", "--quiet"]);
    let program = cmd.get_program().to_string_lossy();
    assert!(program.ends_with(r"\wsl.exe"), "{program}");
    let args: Vec<_> = cmd
        .get_args()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(args, ["--list", "--quiet"]);
}

#[test]
fn child_guard_polls_running_success_failure_and_kills_live_process_on_drop() {
    let live = Command::new("ping.exe")
        .args(["-n", "10", "127.0.0.1"])
        .stdout(Stdio::null())
        .creation_flags(0x0800_0000)
        .spawn()
        .unwrap();
    let mut guard = ChildGuard(live);
    assert_eq!(guard.poll().unwrap(), None);
    drop(guard);

    let mut ok_child = Command::new("cmd.exe")
        .args(["/D", "/C", "exit /b 0"])
        .creation_flags(0x0800_0000)
        .spawn()
        .unwrap();
    ok_child.wait().unwrap();
    let mut ok_guard = ChildGuard(ok_child);
    assert_eq!(ok_guard.poll().unwrap(), Some(Exit::Success));
    drop(ok_guard);

    let mut fail_child = Command::new("cmd.exe")
        .args(["/D", "/C", "exit /b 7"])
        .creation_flags(0x0800_0000)
        .spawn()
        .unwrap();
    fail_child.wait().unwrap();
    let mut fail_guard = ChildGuard(fail_child);
    assert_eq!(fail_guard.poll().unwrap(), Some(Exit::Failed(Some(7))));
}

struct FlakyReader(u8);

impl Read for FlakyReader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        self.0 += 1;
        if self.0 == 1 {
            Err(io::Error::from(io::ErrorKind::Interrupted))
        } else {
            Err(io::Error::other("pipe read fault"))
        }
    }
}

#[test]
fn read_pipe_retries_interrupted_reads_and_surfaces_terminal_io_error() {
    let rx = read_pipe(FlakyReader(0)).unwrap();
    let err = rx
        .recv_timeout(Duration::from_secs(2))
        .unwrap()
        .unwrap_err();
    assert!(err.contains("pipe read fault"), "{err}");
}
