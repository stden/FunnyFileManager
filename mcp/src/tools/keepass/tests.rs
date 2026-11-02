use super::*;
use kdbx::ExportError;
use rmcp::model::ErrorCode;

fn check(error: ExportError, code: ErrorCode, text: &str) {
    let result = export_error(error);
    assert_eq!(result.code, code);
    assert!(result.message.contains(text));
}

#[test]
fn credential_and_serialization_failures_keep_coarse_messages() {
    check(
        ExportError::Read(std::io::Error::other("fixture read failure")),
        ErrorCode::INTERNAL_ERROR,
        "failed to read KeePass data",
    );
    check(
        ExportError::InvalidKeyFile,
        ErrorCode::INVALID_PARAMS,
        "invalid KeePass key file",
    );
    check(
        ExportError::Decrypt,
        ErrorCode::INVALID_PARAMS,
        "could not decrypt KeePass database",
    );
    check(
        ExportError::Serialize,
        ErrorCode::INTERNAL_ERROR,
        "could not serialize KeePass database",
    );
}

#[test]
fn size_failures_explain_the_database_and_output_limits() {
    check(
        ExportError::DatabaseTooLarge(kdbx::MAX_DATABASE_BYTES + 1),
        ErrorCode::INVALID_PARAMS,
        "maximum is",
    );
    check(
        ExportError::OutputTooLarge(kdbx::MAX_OUTPUT_BYTES + 1),
        ErrorCode::INVALID_PARAMS,
        "maximum is",
    );
}

#[test]
fn keyfile_paths_are_confined_and_directories_are_refused_before_decryption() {
    let root = tempfile::tempdir().unwrap();
    let vault = root.path().join("vault.kdbx");
    std::fs::write(&vault, "invalid encrypted fixture").unwrap();
    let server = FfmServer::with_config(root.path().canonicalize().unwrap(), false, true);
    let result = server
        .kbdx(Parameters(KdbxParams {
            path: vault.display().to_string(),
            password: None,
            keyfile: Some(".".into()),
            format: KdbxFormat::Json,
        }))
        .unwrap_err();
    drop(server);
    assert_eq!(result.code, ErrorCode::INVALID_PARAMS);
    assert_eq!(result.message, "KeePass key file path is not a file");
}
