//! Purpose: Ctrl+C typed while a package manager owns the terminal stops the
//! update, not FFM.
//!
//! The terminal delivers Ctrl+C as SIGINT to its whole foreground process
//! group, and the package manager runs in FFM's group. The test raises a real
//! SIGINT, which only a process of its own may do: the parent test re-runs the
//! ignored child test in a fresh copy of this test binary and checks that the
//! copy lived to report success.

use super::*;
use crate::{
    models::Panel,
    packages::{Elevation, Manager},
    terminal::NoopTerminalOps,
    WritableTestBackend,
};
use tempfile::TempDir;

const CHILD_TEST: &str =
    "ops::packages::tests_interrupt::ctrl_c_at_the_prompt_is_reported_as_a_cancelled_update";

#[test]
fn ctrl_c_during_a_package_update_cancels_it_and_ffm_keeps_running() {
    let child = Command::new(std::env::current_exe().expect("this test binary"))
        .args(["--ignored", "--exact", CHILD_TEST])
        .output()
        .expect("the child test ran");
    let stdout = String::from_utf8_lossy(&child.stdout);
    assert!(
        child.status.success() && stdout.contains("1 passed"),
        "the process that took the Ctrl+C ended with {:?}:\n{stdout}{}",
        child.status,
        String::from_utf8_lossy(&child.stderr)
    );
}

/// Child of [`ctrl_c_during_a_package_update_cancels_it_and_ffm_keeps_running`].
#[test]
#[ignore = "raises SIGINT; run in its own process by the test above"]
fn ctrl_c_at_the_prompt_is_reported_as_a_cancelled_update() {
    let temp = TempDir::new().expect("tempdir");
    let mut app = App::with_panels(
        Panel::new(temp.path().to_owned()).expect("panel"),
        Panel::new(temp.path().to_owned()).expect("panel"),
    );
    app.package_update = Some(UpdatePlan {
        manager: Manager::Apt,
        executable: temp.path().join("fake-apt"),
        elevation: Elevation::Sudo,
    });
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("terminal");

    run_with(&mut terminal, &mut app, &mut NoopTerminalOps, |_, _| {
        // What the terminal does with Ctrl+C at sudo's password prompt: SIGINT
        // to the foreground group, FFM included. sudo then gives up with 1.
        signal_hook::low_level::raise(signal_hook::consts::SIGINT)?;
        Err(io::Error::other(
            "sudo did not run apt-get update (exit code 1)",
        ))
    })
    .expect("the terminal is restored");

    assert_eq!(app.status_text(), "Package update cancelled");
    terminal.draw(|_| {}).expect("FFM is still drawing");
}
