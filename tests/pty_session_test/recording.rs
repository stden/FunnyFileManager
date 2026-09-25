//! Purpose: A `--record` that cannot start says so where the user is looking.

use super::*;

/// A refused recording directory was reported only on stderr, just before the
/// TUI took the screen, so nobody ever read it. It is on the status line once
/// the panels are up. The directory here is a link somebody planted in the
/// shared temporary directory to redirect the keystrokes.
#[test]
fn a_refused_recording_directory_is_reported_on_the_status_line() {
    let sandbox = TempDir::new().expect("sandbox");
    let tmp = sandbox.path().join("tmp");
    let elsewhere = sandbox.path().join("elsewhere");
    fs::create_dir_all(&tmp).expect("tmp");
    fs::create_dir_all(&elsewhere).expect("elsewhere");
    let uid = rustix::process::getuid().as_raw();
    let root = tmp.join(format!("ffm-recordings-{uid}"));
    std::os::unix::fs::symlink(&elsewhere, root).expect("plant the link");
    let path = std::env::var_os("PATH").unwrap_or_default();

    let mut ffm = Ffm::start_in(sandbox, &path, &["--record"]);

    let shown = ffm.wait_for(PATIENCE, |ffm| {
        contains(&ffm.screen, b"Error: recording disabled")
    });
    assert!(shown.is_some(), "the status line never said why");
    let recorded = fs::read_dir(&elsewhere).expect("elsewhere").count();
    assert_eq!(recorded, 0, "nothing is recorded through the link");
}
