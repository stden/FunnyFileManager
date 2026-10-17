//! Purpose: Verify `kill_shell_tree` terminates grandchild processes and releases stdout/stderr pipes.

use std::io::Read;
use std::time::Duration;

use super::{kill_shell_tree, spawn_shell};

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
