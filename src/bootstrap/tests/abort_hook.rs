//! Purpose: The panic hook restores an interactive terminal before the
//! previous hook runs, and leaves a headless session's output untouched.

use crate::bootstrap::panic_restore_hook;
use crate::terminal::headless::HeadlessHost;

/// Set in the re-run child: the kind of session its panic happens in.
const MODE: &str = "FFM_ABORT_HOOK_TEST_MODE";
const TEST: &str = "bootstrap::tests::abort_hook::\
    panic_restoration_precedes_the_previous_hook_and_respects_headless_sessions";

/// The re-run child: panic under the restoring hook, whose previous hook
/// marks its turn on stdout.
fn panic_in_child(mode: &str) {
    // A session of its own: no controlling terminal the restoration could reach.
    rustix::process::setsid().unwrap();
    let host = HeadlessHost::new();
    let _scope = (mode == "headless").then(|| host.enter());
    let previous = |_: &std::panic::PanicHookInfo<'_>| println!("previous-hook-ran");
    std::panic::set_hook(panic_restore_hook(Box::new(previous)));
    assert!(std::panic::catch_unwind(|| panic!("fixture panic")).is_err());
}

/// The stdout of this test re-run alone in `mode`, which must pass.
fn child_stdout(mode: &str) -> String {
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", TEST, "--nocapture"])
        .env(MODE, mode)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn panic_restoration_precedes_the_previous_hook_and_respects_headless_sessions() {
    if let Ok(mode) = std::env::var(MODE) {
        panic_in_child(&mode);
        return;
    }
    for mode in ["interactive", "headless"] {
        let text = child_stdout(mode);
        let previous = text.find("previous-hook-ran").expect("previous hook ran");
        if mode == "headless" {
            assert!(
                !text.contains('\x1b'),
                "headless output must contain no terminal commands"
            );
        } else {
            let restored = text.find("\x1b[?1049l").expect("alternate screen was left");
            assert!(restored < previous, "terminal restoration must run first");
        }
    }
}
