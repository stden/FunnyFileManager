//! Purpose: The Maven row. Its scope matches the partial-download markers under
//! the local repository; the artifacts beside them, a folder named like a
//! marker and a link to a marker all stay.

use tempfile::TempDir;

use super::{assert_kept, clean, measured_row, write};
use crate::models::{CleanupKind, TempScope};
use crate::test_support::EnvVarsGuard;

/// The markers the row removes, relative to the repository.
const MARKERS: [&str; 3] = [
    "org/x/lib/1.0/lib-1.0.pom.lastUpdated",
    "org/x/lib/1.0/maven-metadata-central.xml.lastUpdated",
    "org/x/lib/resolver-status.properties",
];

/// Files that must survive beside the markers: artifacts, the repository's own
/// bookkeeping, a properties file with another name and a folder named like a
/// marker.
const SURVIVORS: [&str; 5] = [
    "org/x/lib/1.0/lib-1.0.jar",
    "org/x/lib/1.0/lib-1.0.pom",
    "org/x/lib/1.0/_remote.repositories",
    "org/y/other.properties",
    "odd.lastUpdated/keep.txt",
];

#[test]
fn maven_partial_downloads_remove_the_markers_and_keep_the_artifacts() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    let repository = home.join(".m2/repository");
    for file in MARKERS.iter().chain(&SURVIVORS) {
        write(&repository.join(file), b"keep");
    }
    let _env = EnvVarsGuard::set(&[("HOME", home.as_path())]);

    let row = measured_row("Maven partial downloads");
    assert_eq!(
        row.kind,
        CleanupKind::Temp(TempScope::MavenPartialDownloads)
    );
    assert_eq!(row.path, repository);
    assert!(row.is_checked(), "partial downloads are temporary markers");
    let app = clean(vec![row]);

    for marker in MARKERS {
        assert!(!repository.join(marker).exists(), "{marker} is a marker");
    }
    for survivor in SURVIVORS {
        assert_kept(&repository.join(survivor), b"keep");
    }
    assert_eq!(app.status.to_string(), "Freed 12 B");
}

#[cfg(unix)]
#[test]
fn maven_partial_downloads_never_follow_a_link_to_a_marker() {
    let temp = TempDir::new().unwrap();
    let home = temp.path().join("home");
    let repository = home.join(".m2/repository");
    let outside = temp.path().join("outside.lastUpdated");
    write(&outside, b"keep");
    write(&repository.join("real.lastUpdated"), b"marker");
    std::os::unix::fs::symlink(&outside, repository.join("link.lastUpdated")).unwrap();
    let _env = EnvVarsGuard::set(&[("HOME", home.as_path())]);

    let row = measured_row("Maven partial downloads");
    assert!(
        row.is_checked(),
        "the markers are ticked as the dialog starts"
    );
    clean(vec![row]);

    assert!(!repository.join("real.lastUpdated").exists());
    assert!(repository
        .join("link.lastUpdated")
        .symlink_metadata()
        .is_ok());
    assert_kept(&outside, b"keep");
}

/// A home with no repository yet: the row is offered, but there is nothing to
/// measure, so it is not present and cannot be ticked.
#[test]
fn a_maven_repository_that_is_not_there_is_not_measured() {
    let temp = TempDir::new().unwrap();
    let _env = EnvVarsGuard::set(&[("HOME", temp.path())]);

    assert!(!measured_row("Maven partial downloads").exists());
}
