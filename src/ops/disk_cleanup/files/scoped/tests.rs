//! Purpose: A scope's root that cannot be listed is an error to report, not an
//! empty root: nothing is measured, nothing is removed, and the failure counts.

use super::*;

const SCOPES: [TempScope; 3] = [
    TempScope::MavenPartialDownloads,
    TempScope::GradleDaemonLogs,
    TempScope::GradleTransforms,
];

/// A root the operating system refuses to list for a reason other than it
/// being missing: its name holds a NUL character, which no path may.
fn unlistable_root() -> &'static Path {
    Path::new("tool\0home")
}

#[test]
fn a_root_that_cannot_be_listed_is_an_error_not_an_empty_listing() {
    for scope in SCOPES {
        let error = measure(unlistable_root(), scope).unwrap_err();
        assert_ne!(error.kind(), io::ErrorKind::NotFound, "{scope:?}");
    }
}

#[test]
fn removing_from_a_root_that_cannot_be_listed_counts_one_failure() {
    for scope in SCOPES {
        let mut outcome = CleanupOutcome::default();
        outcome.remove_scope(unlistable_root(), scope);
        assert_eq!(outcome.failures, 1, "{scope:?}");
        assert_eq!(outcome.freed.bytes(), 0, "{scope:?}");
    }
}

#[test]
fn a_missing_root_has_nothing_to_measure_or_remove() {
    let temp = tempfile::TempDir::new().unwrap();
    let missing = temp.path().join("absent");
    for scope in SCOPES {
        assert_eq!(measure(&missing, scope).unwrap(), None, "{scope:?}");
        let mut outcome = CleanupOutcome::default();
        outcome.remove_scope(&missing, scope);
        assert_eq!(outcome.failures, 0, "{scope:?}");
    }
}
