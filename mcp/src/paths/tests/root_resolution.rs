//! Purpose: Pins how `resolve_root` picks the sandbox root at startup, and
//! that it fails closed rather than guessing one.

use super::{root_of, RootEnvGuard};
use crate::paths::{resolve_root, ROOT_ENV};
#[cfg(unix)]
use std::fs;
use tempfile::TempDir;

#[test]
fn resolve_root_prefers_the_environment_variable() {
    let dir = TempDir::new().expect("temp dir");
    let _guard = RootEnvGuard::acquire();
    std::env::set_var(ROOT_ENV, dir.path());

    assert_eq!(
        resolve_root().expect("the configured root resolves"),
        root_of(&dir),
        "{ROOT_ENV} must decide the sandbox, not the working directory"
    );
}

#[test]
fn resolve_root_falls_back_to_the_working_directory() {
    let dir = TempDir::new().expect("temp dir");
    let _guard = RootEnvGuard::acquire();
    std::env::remove_var(ROOT_ENV);
    std::env::set_current_dir(dir.path()).expect("change working directory");

    assert_eq!(
        resolve_root().expect("the working directory is the fallback root"),
        root_of(&dir)
    );
}

#[test]
fn resolve_root_fails_closed_when_the_configured_root_is_missing() {
    let dir = TempDir::new().expect("temp dir");
    let missing = dir.path().join("no_such_root");
    let _guard = RootEnvGuard::acquire();
    std::env::set_var(ROOT_ENV, &missing);

    // Falling back to the cwd here would silently hand the server a sandbox
    // the operator never asked for, so a bad root has to abort startup.
    let error = resolve_root().expect_err("a missing root must not resolve to anything");
    assert!(
        error.contains(ROOT_ENV),
        "error should name the variable, got: {error}"
    );
    assert!(
        error.contains(&missing.display().to_string()),
        "error should name the unusable path, got: {error}"
    );
}

#[cfg(unix)]
#[test]
fn resolve_root_fails_closed_when_the_working_directory_is_gone() {
    let dir = TempDir::new().expect("temp dir");
    let gone = dir.path().to_path_buf();
    let _guard = RootEnvGuard::acquire();
    std::env::remove_var(ROOT_ENV);
    std::env::set_current_dir(&gone).expect("change working directory");
    // The directory the process sits in is removed underneath it, so even the
    // fallback has nothing to resolve. Guessing a root here would be worse
    // than refusing to start. The guard restores the previous cwd on drop.
    fs::remove_dir(&gone).expect("remove the working directory");

    let error = resolve_root().expect_err("there is no directory left to fall back to");
    assert!(
        error.contains("cannot determine current directory"),
        "got: {error}"
    );
}
