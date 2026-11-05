//! Purpose: The Gradle rows. Their scopes match files and folders by name under
//! one root; each test checks that the matches go while the artifacts, the
//! registry, the running state and the links beside them stay.

use tempfile::TempDir;

use super::{assert_kept, clean, measured_row, write};
use crate::models::{CleanupKind, TempScope};
use crate::test_support::EnvVarsGuard;

#[test]
fn gradle_daemon_logs_remove_only_the_logs_of_each_version_folder() {
    let temp = TempDir::new().unwrap();
    let gradle = temp.path().join("gradle");
    let daemon = gradle.join("daemon");
    let version = daemon.join("8.11.1");
    write(&version.join("daemon-221199.out.log"), b"log");
    write(&version.join("daemon-220001.log"), b"old");
    write(&version.join("registry.bin"), b"keep");
    write(&version.join("registry.bin.lock"), b"keep");
    write(&daemon.join("notes/old.log"), b"keep");
    let _env = EnvVarsGuard::set(&[("GRADLE_USER_HOME", gradle.as_path())]);

    let row = measured_row("Gradle daemon logs");

    assert_eq!(row.kind, CleanupKind::Temp(TempScope::GradleDaemonLogs));
    assert_eq!(row.path, daemon);
    assert!(row.is_checked(), "daemon logs are temporary output");
    let app = clean(vec![row]);
    assert!(!version.join("daemon-221199.out.log").exists());
    assert!(!version.join("daemon-220001.log").exists());
    assert_kept(&version.join("registry.bin"), b"keep");
    assert_kept(&version.join("registry.bin.lock"), b"keep");
    assert_kept(&daemon.join("notes/old.log"), b"keep");
    assert_eq!(app.status.to_string(), "Freed 6 B");
}

#[cfg(unix)]
#[test]
fn gradle_daemon_logs_never_follow_or_remove_a_link() {
    let temp = TempDir::new().unwrap();
    let gradle = temp.path().join("gradle");
    let version = gradle.join("daemon/8.11.1");
    let outside = temp.path().join("outside.txt");
    write(&outside, b"keep");
    write(&version.join("daemon-1.log"), b"log");
    std::os::unix::fs::symlink(&outside, version.join("link.log")).unwrap();
    let _env = EnvVarsGuard::set(&[("GRADLE_USER_HOME", gradle.as_path())]);

    let row = measured_row("Gradle daemon logs");
    assert!(row.is_checked(), "the logs are ticked as the dialog starts");
    clean(vec![row]);

    assert!(!version.join("daemon-1.log").exists());
    assert!(
        version.join("link.log").symlink_metadata().is_ok(),
        "a link is not a log"
    );
    assert_kept(&outside, b"keep");
}

#[test]
fn gradle_transforms_remove_the_transforms_folders_and_nothing_else() {
    let temp = TempDir::new().unwrap();
    let gradle = temp.path().join("gradle");
    let caches = gradle.join("caches");
    write(&caches.join("transforms-3/abc/file.bin"), b"cache");
    write(&caches.join("modules-2/files-2.1/lib.jar"), b"keep");
    write(&caches.join("transforms-7"), b"keep");
    let _env = EnvVarsGuard::set(&[("GRADLE_USER_HOME", gradle.as_path())]);

    let mut row = measured_row("Gradle transforms");

    assert_eq!(row.kind, CleanupKind::Temp(TempScope::GradleTransforms));
    assert_eq!(row.path, caches);
    assert!(
        !row.is_checked(),
        "Gradle rebuilds transforms, so the row starts unticked"
    );
    row.toggle_checked();
    let app = clean(vec![row]);
    assert!(!caches.join("transforms-3").exists());
    assert_kept(&caches.join("modules-2/files-2.1/lib.jar"), b"keep");
    assert_kept(&caches.join("transforms-7"), b"keep");
    assert_eq!(app.status.to_string(), "Freed 5 B");
}

#[cfg(unix)]
#[test]
fn gradle_transforms_skip_a_link_named_like_a_transforms_folder() {
    let temp = TempDir::new().unwrap();
    let gradle = temp.path().join("gradle");
    let caches = gradle.join("caches");
    write(&caches.join("transforms-3/file.bin"), b"cache");
    write(&caches.join("modules-2/lib.jar"), b"keep");
    std::os::unix::fs::symlink(caches.join("modules-2"), caches.join("transforms-link")).unwrap();
    let _env = EnvVarsGuard::set(&[("GRADLE_USER_HOME", gradle.as_path())]);

    let mut row = measured_row("Gradle transforms");
    row.toggle_checked();
    clean(vec![row]);

    assert!(!caches.join("transforms-3").exists());
    assert!(caches.join("transforms-link").symlink_metadata().is_ok());
    assert_kept(&caches.join("modules-2/lib.jar"), b"keep");
}

/// The Gradle caches row and the transforms row share a folder. Ticking both
/// removes the folder once; the transforms row then has nothing left to
/// remove, and that is not a failure.
#[test]
fn ticking_the_gradle_caches_and_its_transforms_reports_no_failure() {
    let temp = TempDir::new().unwrap();
    let gradle = temp.path().join("gradle");
    let caches = gradle.join("caches");
    write(&caches.join("transforms-3/file.bin"), b"cache");
    let _env = EnvVarsGuard::set(&[("GRADLE_USER_HOME", gradle.as_path())]);
    let mut whole = measured_row("Gradle caches");
    let mut transforms = measured_row("Gradle transforms");
    whole.toggle_checked();
    transforms.toggle_checked();

    let app = clean(vec![whole, transforms]);

    assert!(!caches.exists());
    assert_eq!(app.status.to_string(), "Freed 5 B");
}

#[test]
fn a_gradle_folder_that_is_not_there_is_not_offered() {
    let temp = TempDir::new().unwrap();
    let _env = EnvVarsGuard::set(&[("GRADLE_USER_HOME", &temp.path().join("absent"))]);

    let row = measured_row("Gradle transforms");

    assert!(!row.exists());
    assert!(!row.is_checked());
}
