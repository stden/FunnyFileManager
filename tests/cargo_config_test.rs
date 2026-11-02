//! Keep local compiler caching opt-in, fail fast on a stalled compiler
//! wrapper, and keep package builds in one artifact tree.

use std::{path::Path, process::Command};

const CARGO_CONFIG: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/.cargo/config.toml"));

#[test]
fn an_inherited_compiler_cache_is_switched_off_and_never_run_in_the_foreground() {
    let settings = CARGO_CONFIG
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .collect::<Vec<_>>();
    assert!(
        settings.contains(&"rustc-wrapper = \"\""),
        "a globally forced sccache must not reach this repository's builds"
    );
    assert!(
        settings
            .iter()
            .all(|line| !line.contains("SCCACHE_NO_DAEMON")),
        "a foreground sccache server inherits Cargo's pipes and stalls the build"
    );
}

/// A stalled wrapper must fail every build entry point in seconds, not minutes.
#[test]
fn build_entry_points_run_the_wrapper_preflight_first() {
    for (script, text) in [
        (
            "ffm.py",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/ffm.py")),
        ),
        (
            "scripts/lint.py",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/lint.py")),
        ),
    ] {
        assert!(
            text.contains("scripts/cargo_preflight.py"),
            "{script} must run the compiler-wrapper preflight"
        );
    }
}

#[cfg(unix)]
#[path = "cargo_config_test/preflight.rs"]
mod preflight;

#[expect(
    clippy::expect_used,
    reason = "test fixture: unreadable Cargo metadata fails the calling test; clippy.toml allows \
              that in #[test] fns and #[cfg(test)] items, not in integration-test helpers"
)]
fn target_directory(package: &Path) -> String {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version=1", "--offline"])
        .current_dir(package)
        .output()
        .expect("read Cargo metadata");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata = String::from_utf8(output.stdout).expect("Cargo metadata is UTF-8");
    metadata
        .split_once("\"target_directory\":\"")
        .expect("target directory field")
        .1
        .split('"')
        .next()
        .expect("target directory value")
        .to_owned()
}

#[test]
fn packages_share_dependency_artifacts() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let target = target_directory(root);
    assert_ne!(target, "");
    assert_eq!(target, target_directory(&root.join("mcp")));
    assert_eq!(target, target_directory(&root.join("colorer-sys")));
}
