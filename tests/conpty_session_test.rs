//! Purpose: Run the real `ffm` binary on a Windows `ConPTY` pseudo-console to
//! exercise `main`, `bootstrap`, `CrosstermOps`, `video_mode`, `typeahead`,
//! and live `recording` input end-to-end.
#![cfg(windows)]

use std::process::Command;
use std::time::Duration;

use tempfile::TempDir;

#[path = "conpty_session_test/harness.rs"]
mod harness;

use harness::{contains, ConPtyFfm, PATIENCE};

#[test]
fn cli_error_routes_report_to_stderr_and_exit_with_failure() {
    for args in [
        &["--automation", "--unknown"][..],
        &["--elevated-helper", "bad-pipe", "0"][..],
        &["--invalid-flag"][..],
    ] {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_ffm"));
        cmd.args(args);
        if let Some(prof) = std::env::var_os("LLVM_PROFILE_FILE") {
            cmd.env("LLVM_PROFILE_FILE", prof);
        }
        let out = cmd.output().expect("run ffm");
        assert!(!out.status.success(), "{args:?} unexpectedly succeeded");
        assert!(String::from_utf8_lossy(&out.stderr).starts_with("ffm: "));
    }
}

#[test]
fn conpty_live_session_runs_program_pause_video_mode_typeahead_and_clean_exit() {
    let dir_blocker = TempDir::new().expect("blocker dir");
    let blocker_arg = dir_blocker.path().to_string_lossy().into_owned();
    let mut ffm = ConPtyFfm::start(25, &["--record", &blocker_arg]);
    assert!(ffm.wait_for(PATIENCE, |s| contains(s, b"recording disabled")));
    ffm.send(b"\x1b[20;3~");
    std::thread::sleep(Duration::from_millis(350));
    ffm.send(b"\x1b[F\r");
    assert!(ffm.wait_for(PATIENCE, |s| contains(s, b"Press any key to return to FFM")));
    ffm.clear_screen();
    ffm.send(b"x");
    assert!(ffm.wait_for(PATIENCE, |s| contains(s, b"Quit")));
    ffm.send(b":ping -n 2 127.0.0.1 >nul\rz");
    std::thread::sleep(Duration::from_millis(1500));
    ffm.clear_screen();
    ffm.send(b":mkdir done_ok\r");
    let done = ffm.work().join("done_ok");
    assert!(ffm.wait_for(PATIENCE, |s| done.is_dir() && contains(s, b"done_ok")));
    assert!(ffm.quit(PATIENCE));
}

#[test]
fn conpty_tall_session_records_live_input_and_requests_compact_video_mode() {
    let rec_dir = TempDir::new().expect("rec dir");
    let rec_file = rec_dir.path().join("recorded_session.py");
    let rec_arg = rec_file.to_string_lossy().into_owned();
    let mut tall = ConPtyFfm::start(40, &["--record", &rec_arg]);
    tall.send(b"\x1b[20;3~");
    std::thread::sleep(Duration::from_millis(350));
    assert!(tall.quit(PATIENCE));
    assert!(rec_file.is_file());
}
