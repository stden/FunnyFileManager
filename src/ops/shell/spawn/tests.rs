//! Purpose: Verify `kill_shell_tree` terminates grandchild processes and `detach_shell_tree` lets backgrounded ones survive.

use std::io::Read;
use std::time::Duration;

use super::{detach_shell_tree, kill_shell_tree, spawn_shell};

#[test]
fn kill_shell_tree_releases_pipes_held_by_grandchildren() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    #[cfg(windows)]
    let cmd = "ping -n 30 127.0.0.1";
    #[cfg(not(windows))]
    let cmd = "sleep 30";

    let mut child = spawn_shell(cmd, dir.path()).expect("spawn");
    std::thread::sleep(Duration::from_millis(100));
    kill_shell_tree(&mut child);

    let mut stdout = child.stdout.take().expect("stdout");
    let done = crate::test_support::run_with_timeout(Duration::from_secs(3), move || {
        let mut buf = Vec::new();
        drop(stdout.read_to_end(&mut buf));
    });
    assert!(
        done.is_some(),
        "grandchild kept stdout pipe open after kill_shell_tree"
    );
}

#[test]
fn detach_shell_tree_preserves_backgrounded_child_across_many_subsequent_spawns() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let marker = dir.path().join("done.txt");
    #[cfg(windows)]
    let cmd = format!(
        "start /B cmd /C \"ping -n 2 127.0.0.1 >nul && echo ok > \"\"{}\"\"\"",
        marker.display()
    );
    #[cfg(not(windows))]
    let cmd = format!("(sleep 0.3 && echo ok > '{}') &", marker.display());

    let mut bg = spawn_shell(&cmd, dir.path()).expect("spawn");
    drop(bg.wait());
    #[cfg(windows)]
    super::ACTIVE_JOBS
        .with(|j| assert!(crate::windows::disarm_kill_on_close(&j.borrow()[0].1).is_ok()));
    detach_shell_tree(&bg);

    for _ in 0..10 {
        let mut echo = spawn_shell("echo hi", dir.path()).expect("spawn");
        drop(echo.wait());
        detach_shell_tree(&echo);
    }

    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while std::time::Instant::now() < deadline && !marker.exists() {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(marker.exists(), "backgrounded process was killed");
}
