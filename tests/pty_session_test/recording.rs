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
    assert!(ffm.quit().expect("ffm exits normally").success());
}

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
fn replay_session_runs_to_clean_completion_without_live_hangup_watcher() {
    use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    let sandbox = TempDir::new().expect("sandbox");
    let replay_path = sandbox.path().join("session.jsonl");
    let header = r#"{"version":1,"executable":"ffm","directory":".","size":[80,24]}"#;
    let warmup = serde_json::json!({
        "delay_ms": 0,
        "event": Event::Resize(80, 24),
    });
    let quit = serde_json::json!({
        "delay_ms": 500,
        "event": Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
    });
    fs::write(&replay_path, format!("{header}\n{warmup}\n{quit}\n")).expect("write replay");
    let path = std::env::var_os("PATH").unwrap_or_default();
    let replay_arg = replay_path.to_str().expect("utf8 path").to_owned();

    let mut ffm = Ffm::start_in(sandbox, &path, &["--replay", &replay_arg]);
    assert!(ffm.wait_for_exit().expect("replay exits").success());
}

/// A replay that runs out of events before the application quits ends ffm
/// with a failure it reports, rather than waiting for input that never comes.
#[test]
fn a_replay_finishes_from_its_own_input_and_reports_a_truncated_session() {
    use crossterm::event::Event;

    let sandbox = TempDir::new().expect("sandbox");
    let replay_path = sandbox.path().join("session.jsonl");
    let header = r#"{"version":1,"executable":"ffm","directory":".","size":[80,24]}"#;
    let warmup = serde_json::json!({"delay_ms": 0, "event": Event::Resize(80, 24)});
    let last = serde_json::json!({"delay_ms": 500, "event": Event::FocusGained});
    fs::write(&replay_path, format!("{header}\n{warmup}\n{last}\n")).expect("write replay");
    let path = std::env::var_os("PATH").unwrap_or_default();
    let replay_arg = replay_path.to_str().expect("utf8 path").to_owned();

    let mut ffm = Ffm::start_in(sandbox, &path, &["--replay", &replay_arg]);
    assert!(!ffm.wait_for_exit().expect("replay exits").success());
    let reported = ffm.wait_for(PATIENCE, |ffm| {
        contains(&ffm.screen, b"Recording ended before the application quit")
    });
    assert!(
        reported.is_some(),
        "the truncated replay was never reported"
    );
}
