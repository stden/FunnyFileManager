//! Purpose: The compiler-wrapper preflight: a stalled or missing wrapper fails fast,
//! a working one passes.

use std::{
    os::unix::fs::PermissionsExt,
    path::Path,
    process::{Command, Output},
    time::{Duration, Instant},
};

const SCRIPT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/cargo_preflight.py");
/// Limit for the cases that must fail; the passing cases never wait for it.
const STALL_SECS: u64 = 5;
/// Limit for the cases that must pass, generous for a loaded machine.
const ANSWER_SECS: u64 = 60;

/// A private `CARGO_HOME`: no other Cargo's package-cache lock, and no
/// developer config (which may select sccache) leaking into the probe.
#[expect(
    clippy::expect_used,
    reason = "test fixture: a failed setup fails the calling test; clippy.toml allows that in \
              #[test] fns and #[cfg(test)] items, not in integration-test helpers"
)]
fn sandbox() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    std::fs::create_dir(dir.path().join("cargo-home")).expect("CARGO_HOME");
    dir
}

/// A stuck sccache in miniature: the wrapper exits, but a process it
/// started keeps Cargo's output pipe open, so Cargo waits for an EOF.
#[expect(
    clippy::expect_used,
    reason = "test fixture: a failed setup fails the calling test; clippy.toml allows that in \
              #[test] fns and #[cfg(test)] items, not in integration-test helpers"
)]
fn stalling_wrapper(sandbox: &Path) -> String {
    let wrapper = sandbox.join("stalling-wrapper");
    std::fs::write(&wrapper, "#!/bin/sh\nsleep 20 &\nexec \"$@\"\n").expect("write wrapper");
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755))
        .expect("chmod wrapper");
    wrapper.to_str().expect("UTF-8 path").to_owned()
}

/// `wrapper: None` leaves the choice to Cargo's config files.
#[expect(
    clippy::expect_used,
    reason = "test fixture: a failed setup fails the calling test; clippy.toml allows that in \
              #[test] fns and #[cfg(test)] items, not in integration-test helpers"
)]
fn preflight(sandbox: &Path, wrapper: Option<&str>, limit_secs: u64) -> (Output, Duration) {
    let mut command = Command::new("python3");
    command
        .arg(SCRIPT)
        .env("CARGO", env!("CARGO"))
        .env("CARGO_HOME", sandbox.join("cargo-home"))
        .env("FFM_WRAPPER_PROBE_SECS", limit_secs.to_string())
        .env_remove("CARGO_BUILD_RUSTC_WRAPPER");
    match wrapper {
        Some(wrapper) => command.env("RUSTC_WRAPPER", wrapper),
        None => command.env_remove("RUSTC_WRAPPER"),
    };
    let started = Instant::now();
    let output = command.output().expect("run the preflight");
    (output, started.elapsed())
}

#[test]
fn passes_when_no_wrapper_is_selected() {
    let sandbox = sandbox();
    let (output, _) = preflight(sandbox.path(), Some(""), ANSWER_SECS);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The fix itself: a wrapper forced by the user's Cargo config (here one
/// that stalls, as a stuck sccache does) never reaches this repository.
#[test]
fn a_wrapper_from_the_user_config_does_not_reach_this_repository() {
    let sandbox = sandbox();
    let wrapper = stalling_wrapper(sandbox.path());
    std::fs::write(
        sandbox.path().join("cargo-home/config.toml"),
        format!("[build]\nrustc-wrapper = {wrapper:?}\n"),
    )
    .expect("write user config");

    let (output, _) = preflight(sandbox.path(), None, STALL_SECS);

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn fails_within_the_limit_when_the_wrapper_holds_cargos_pipe_open() {
    let sandbox = sandbox();
    let wrapper = stalling_wrapper(sandbox.path());

    let (output, elapsed) = preflight(sandbox.path(), Some(&wrapper), STALL_SECS);

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        elapsed < Duration::from_secs(STALL_SECS + 10),
        "took {elapsed:?}"
    );
    assert!(stderr.contains("got no answer"), "{stderr}");
}

#[test]
fn reports_cargos_reason_when_the_wrapper_is_missing() {
    let sandbox = sandbox();
    let (output, _) = preflight(
        sandbox.path(),
        Some("/nonexistent/compiler-wrapper"),
        ANSWER_SECS,
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains("could not query the Rust compiler"),
        "{stderr}"
    );
    // Cargo's own words, not only the preflight's summary.
    assert!(stderr.contains("could not execute process"), "{stderr}");
}
