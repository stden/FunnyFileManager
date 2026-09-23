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
