//! Purpose: Preserve useful failure categories without disclosing paths or command output.

use super::*;

#[test]
fn failure_messages_explain_recovery_without_forwarding_private_io_details() {
    let error = InstallError::from(std::io::Error::other("private project source"));
    assert!(error.to_string().contains("storage or process error"));
    assert!(!error.to_string().contains("private"));
    for (error, expected) in [
        (
            InstallError::Missing("Java 21 required"),
            "Java 21 required",
        ),
        (InstallError::Recipe, "invalid recipe"),
        (InstallError::Command, "check network/package manager"),
        (InstallError::Timeout, "timed out"),
        (InstallError::Cancelled, "cancelled"),
        (InstallError::Integrity, "checksum mismatch"),
        (InstallError::Archive, "unsafe or invalid archive"),
        (InstallError::Worker, "stopped unexpectedly"),
    ] {
        assert!(error.to_string().contains(expected));
    }
}

#[test]
fn only_a_program_that_exited_with_an_error_is_named_by_its_step() {
    assert_eq!(
        InstallError::Command.during(Step::Pip),
        InstallError::Failed(Step::Pip)
    );
    assert_eq!(
        InstallError::Timeout.during(Step::Pip),
        InstallError::Timeout
    );
    assert_eq!(
        InstallError::cache(std::io::Error::from(ErrorKind::PermissionDenied)),
        InstallError::Cache(ErrorKind::PermissionDenied)
    );
}

/// A failed step names what to fix, and so does a cache folder that cannot
/// be used: "check network/package manager" and "storage or process error"
/// sent a user without python3-venv or with a read-only config folder the
/// wrong way.
#[test]
fn a_failed_step_and_an_unusable_cache_say_what_to_fix() {
    for (error, expected) in [
        (InstallError::Failed(Step::Venv), "install python3-venv"),
        (InstallError::Failed(Step::Pip), "pip install failed"),
        (InstallError::Failed(Step::Npm), "npm install failed"),
        (InstallError::Failed(Step::Download), "the download failed"),
        (
            InstallError::Failed(Step::Custom),
            "an install command exited",
        ),
        (
            InstallError::Cache(ErrorKind::PermissionDenied),
            "the LSP cache folder is not writable",
        ),
        (
            InstallError::Cache(ErrorKind::ReadOnlyFilesystem),
            "the LSP cache folder is on a read-only file system",
        ),
        (
            InstallError::Cache(ErrorKind::StorageFull),
            "no space left for the LSP cache",
        ),
        (
            InstallError::Cache(ErrorKind::NotADirectory),
            "cannot use the LSP cache folder",
        ),
    ] {
        assert!(error.to_string().contains(expected), "{error:?}");
    }
}
