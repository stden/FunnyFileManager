//! Purpose: the rendered text of editor, new-file, platform, size-limit and vault outcomes, and that secrets never reach it.

use super::*;

#[test]
fn editor_open_statuses_match_legacy_strings() {
    assert_status_text(&[
        (Status::Editing("/e".into()), "Editing /e"),
        (Status::Edited("/e".into()), "Edited /e"),
        (
            Status::CannotEdit {
                path: PathBuf::from("/e"),
                error: "nope".into(),
            },
            "Cannot edit /e: nope",
        ),
        (
            Status::EditDirUnsupported,
            "Editing directories is not supported",
        ),
        (
            Status::FileTooLargeEdit {
                megabytes: 12.34,
                limit_megabytes: 10.0,
            },
            "File too large to edit (12.3 MB, max 10 MB)",
        ),
    ]);
}

#[test]
fn editor_save_statuses_match_legacy_strings() {
    assert_status_text(&[
        (
            Status::EditorExited {
                status: "exit status: 1".into(),
            },
            "Editor exited with status exit status: 1",
        ),
        (
            Status::LaunchEditorFailed { error: "x".into() },
            "Failed to launch editor: x",
        ),
        (Status::Saved(PathBuf::from("/s")), "Saved /s"),
        (Status::SaveFailed { error: "x".into() }, "Save failed: x"),
        (Status::ChangesDiscarded, "Changes discarded"),
        (
            Status::SetupSaved(PathBuf::from("/c/settings.yaml")),
            "Setup saved to /c/settings.yaml",
        ),
    ]);
}

#[test]
fn new_file_statuses_match_legacy_strings() {
    assert_status_text(&[
        (Status::FilenameEmpty, "Filename cannot be empty"),
        (
            Status::FileExists("a.txt".into()),
            "File already exists: a.txt",
        ),
        (Status::CreatedFile("a.txt".into()), "Created file: a.txt"),
        (
            Status::CreateFileFailed { error: "x".into() },
            "Failed to create file: x",
        ),
    ]);
}

#[test]
fn platform_statuses_match_legacy_strings() {
    assert_status_text(&[
        (
            Status::DeleteFailed { error: "x".into() },
            "Delete failed: x",
        ),
        (Status::DeleteCancelled, "Delete cancelled"),
        (Status::WipeCancelled, "Wipe cancelled"),
        (Status::NothingToWipe, "Nothing to wipe"),
        (
            Status::OpenDriveFailed { error: "x".into() },
            "Failed to open drive: x",
        ),
        (Status::ChangedTo(PathBuf::from("/p")), "Changed to /p"),
        (
            Status::S3ProfileSelected("dev".into()),
            "S3 profile 'dev' selected (S3 browser coming soon)",
        ),
        (Status::Error("x".into()), "Error: x"),
        (
            Status::WindowsOnly { tool: "system" },
            "Windows system checker is only available on Windows",
        ),
    ]);
}

/// The limit in the message must follow the limit that was actually
/// enforced — it used to be hard-coded as "max 10 MB" in the text.
#[test]
fn too_large_messages_report_the_carried_limit() {
    assert_eq!(
        Status::FileTooLarge {
            megabytes: 60.0,
            limit_megabytes: 50.0,
        }
        .to_string(),
        "File too large to view (60.0 MB, max 50 MB)"
    );
    assert_eq!(
        Status::FileTooLargeEdit {
            megabytes: 60.0,
            limit_megabytes: 50.0,
        }
        .to_string(),
        "File too large to edit (60.0 MB, max 50 MB)"
    );
}

/// The limit the handlers pass comes from one constant, so the rendered
/// message and the guard cannot drift apart.
#[test]
fn view_limit_message_matches_the_constant() {
    let limit = crate::utils::MAX_FILE_SIZE.megabytes();
    assert_eq!(
        Status::FileTooLarge {
            megabytes: limit + 1.0,
            limit_megabytes: limit,
        }
        .to_string(),
        "File too large to view (11.0 MB, max 10 MB)"
    );
}

/// A copy outcome names the *field label*, never the copied value — the
/// variant carries only a [`Secret`] discriminant, so the status bar
/// cannot leak what went to the clipboard.
#[test]
fn copied_status_names_the_label_not_the_secret() {
    assert_eq!(
        Status::Copied(Secret::Password).to_string(),
        "Password copied"
    );
    assert_eq!(
        Status::Copied(Secret::Username).to_string(),
        "User name copied"
    );
    assert_eq!(
        Status::CopiedViaOsc52(Secret::Password).to_string(),
        "Password sent to the terminal via OSC 52"
    );
    assert_eq!(
        Status::CopiedViaOsc52(Secret::Username).to_string(),
        "User name sent to the terminal via OSC 52"
    );
}

/// The vault outcomes carry no key material: the only payloads are the
/// file name in `VaultOpened` and the pre-redacted error in `CopyFailed`.
/// `UnlockFailed` is deliberately bare — the reason sits beside the
/// password field, not on the status bar.
#[test]
fn vault_statuses_show_only_the_file_name_and_a_redacted_error() {
    assert_eq!(
        Status::VaultOpened("vault.kdbx".to_owned()).to_string(),
        "Opened vault.kdbx: enter the master password"
    );
    assert_eq!(Status::VaultLocked.to_string(), "Vault locked");
    assert_eq!(
        Status::VaultUnlocked { entries: 5 }.to_string(),
        "Vault unlocked: 5 entries"
    );
    assert_eq!(Status::UnlockFailed.to_string(), "Unlock failed");
    assert_eq!(
        Status::CopyFailed {
            error: "no clipboard available".to_owned(),
        }
        .to_string(),
        "Copy failed: no clipboard available"
    );
}
